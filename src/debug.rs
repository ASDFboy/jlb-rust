// jlb --debug <file>: runs a program for an editor's debugger.
//
// It works like --ide (output is sent right away, a crash adds a JSON report),
// and the editor can also stop the program and look inside it.
//
// The editor talks to jlb on standard input. A line starting with the MARK
// character is a command; any other line is an answer for INPUT.
//
//   MARK break <path> TAB <line>,<line>,...   the breakpoints for one file (replaces them)
//   MARK continue                             run until the next breakpoint
//   MARK step                                 stop at the very next statement (step into)
//   MARK over                                 stop at the next statement in this function or above
//   MARK out                                  stop after leaving this function
//   MARK pause                                stop as soon as possible (works while running)
//   MARK eval <id> TAB <expression>           work out a value while stopped
//   MARK stop                                 end the program
//
// jlb answers on standard error with MARK and one line of JSON:
//
//   {"event":"ready"}                         waiting for breakpoints, then continue or step
//   {"event":"paused","reason":"breakpoint|step|pause|error","path":...,"line":N,
//    "message":..., "stack":[{"name","path","line","vars":[value...]}...], "globals":[value...]}
//   {"event":"resumed"}
//   {"event":"eval","id":...,"ok":true,"value":value} or {"event":"eval","id":...,"ok":false,"message":...}
//
// A value is {"name","type","value"} plus "items" (a table's entries, as values)
// and "more" (how many entries were left out) for tables.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;

use crate::ast::StmtKind;
use crate::ide::{json_str, MARK};
use crate::interp::Interp;
use crate::lexer;
use crate::parser::Parser;
use crate::value::{quote_text, Fault, Value, R};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Run,
    Step,
    Over(usize),
    Out(usize),
}

pub struct Debugger {
    cmds: Receiver<String>,
    pause_req: Arc<AtomicBool>,
    breaks: HashMap<PathBuf, HashSet<usize>>,
    /// Paths as the editor wrote them, so replies use the editor's spelling.
    names: HashMap<PathBuf, String>,
    mode: Mode,
    /// The breakpoints of the file last looked up (by its Source's address), so the
    /// check before each statement doesn't look the path up every time.
    cache: Option<(usize, Option<HashSet<usize>>)>,
    /// Commands that arrived while running and only mean something once stopped.
    pending: std::collections::VecDeque<String>,
}

/// Starts reading standard input: commands go to the debugger, other lines to INPUT.
pub fn start() -> (Debugger, Receiver<String>) {
    let (cmd_tx, cmds) = channel();
    let (input_tx, inputs) = channel();
    let pause_req = Arc::new(AtomicBool::new(false));
    let flag = pause_req.clone();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            match stdin.lock().read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            let t = line.trim_end_matches(['\n', '\r']);
            if let Some(cmd) = t.strip_prefix(MARK) {
                if cmd.trim() == "pause" {
                    flag.store(true, Ordering::SeqCst);
                } else if cmd_tx.send(cmd.to_string()).is_err() {
                    break;
                }
            } else if input_tx.send(t.to_string()).is_err() {
                break;
            }
        }
    });
    (Debugger { cmds, pause_req, breaks: HashMap::new(), names: HashMap::new(), mode: Mode::Run, cache: None, pending: Default::default() }, inputs)
}

fn canon(p: &str) -> PathBuf {
    Path::new(p).canonicalize().unwrap_or_else(|_| PathBuf::from(p))
}

impl Debugger {
    /// Remembers how the editor spells a file's path.
    pub fn name_path(&mut self, given: &str) -> PathBuf {
        let c = canon(given);
        self.names.insert(c.clone(), given.to_string());
        c
    }

    fn shown(&self, p: &Path) -> String {
        self.names.get(p).cloned().unwrap_or_else(|| p.to_string_lossy().into_owned())
    }

    fn set_breaks(&mut self, arg: &str) {
        let (path, lines) = arg.split_once('\t').unwrap_or((arg, ""));
        let c = self.name_path(path.trim());
        self.cache = None;
        let set: HashSet<usize> = lines.split(',').filter_map(|n| n.trim().parse().ok()).collect();
        if set.is_empty() {
            self.breaks.remove(&c);
        } else {
            self.breaks.insert(c, set);
        }
    }

    /// Handles commands that arrived while the program was running: breakpoints
    /// change straight away, and the rest wait (in order) for the next stop.
    fn drain(&mut self) {
        loop {
            match self.cmds.try_recv() {
                Ok(c) => match c.strip_prefix("break ") {
                    Some(arg) if self.pending.is_empty() => self.set_breaks(arg),
                    _ => self.pending.push_back(c),
                },
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
    }

    fn next_command(&mut self) -> Option<String> {
        self.pending.pop_front().or_else(|| self.cmds.recv().ok())
    }
}

fn emit(json: &str) {
    let mut e = std::io::stderr().lock();
    let _ = writeln!(e, "{}{}", MARK, json);
    let _ = e.flush();
}

// ---------- values as JSON ----------

/// Limits how much of a big table is sent at each stop.
struct Budget(usize);

fn short(v: &Value, out: &mut String, room: usize) {
    if out.len() > room {
        return;
    }
    match v {
        Value::Text(s) => {
            let q = quote_text(s);
            if q.chars().count() > 80 {
                out.extend(q.chars().take(77));
                out.push('…');
            } else {
                out.push_str(&q);
            }
        }
        Value::Table(t, _) => {
            out.push('[');
            let mut first = true;
            let mut sep = |out: &mut String| {
                if !first {
                    out.push_str(", ");
                }
                first = false;
            };
            for s in &t.items {
                sep(out);
                if out.len() > room {
                    out.push('…');
                    out.push(']');
                    return;
                }
                short(&s.borrow(), out, room);
            }
            for (k, s) in t.named.iter() {
                sep(out);
                if out.len() > room {
                    out.push('…');
                    out.push(']');
                    return;
                }
                out.push_str(k);
                out.push_str(": ");
                short(&s.borrow(), out, room);
            }
            out.push(']');
        }
        o => out.push_str(&crate::value::repr(o)),
    }
}

fn value_json(name: &str, v: &Value, depth: usize, budget: &mut Budget) -> String {
    budget.0 = budget.0.saturating_sub(1);
    let mut summary = String::new();
    short(v, &mut summary, 100);
    let mut o = format!("{{\"name\":{},\"type\":\"{}\",\"value\":{}", json_str(name), v.type_name(), json_str(&summary));
    if let Value::Table(t, _) = v {
        let total = t.items.len() + t.named.len();
        let mut items = Vec::new();
        if depth < 6 {
            for (i, s) in t.items.iter().enumerate() {
                if budget.0 == 0 || items.len() >= 200 {
                    break;
                }
                items.push(value_json(&(i + 1).to_string(), &s.borrow(), depth + 1, budget));
            }
            for (k, s) in t.named.iter() {
                if budget.0 == 0 || items.len() >= 200 {
                    break;
                }
                items.push(value_json(k, &s.borrow(), depth + 1, budget));
            }
        }
        o.push_str(&format!(",\"items\":[{}],\"more\":{}", items.join(","), total - items.len()));
    }
    o.push('}');
    o
}

fn vars_json(vars: &[(std::rc::Rc<str>, Value)], budget: &mut Budget) -> String {
    let items: Vec<String> = vars.iter().map(|(k, v)| value_json(k, v, 0, budget)).collect();
    format!("[{}]", items.join(","))
}

// ---------- the interpreter's side ----------

impl Interp {
    /// Runs before every statement while debugging: stops when it should.
    pub fn debug_check(&mut self, line: usize) -> R<()> {
        let depth = self.frames.len();
        let Some(d) = self.debug.as_mut() else { return Ok(()) };
        d.drain();
        let at_break = !d.breaks.is_empty() && {
            let src = &self.frames.last().unwrap().src;
            let key = std::rc::Rc::as_ptr(src) as usize;
            if d.cache.as_ref().map(|c| c.0) != Some(key) {
                d.cache = Some((key, d.breaks.get(&src.canon).cloned()));
            }
            d.cache.as_ref().unwrap().1.as_ref().is_some_and(|s| s.contains(&line))
        };
        let reason = if at_break {
            "breakpoint"
        } else if d.pause_req.swap(false, Ordering::SeqCst) {
            "pause"
        } else {
            match d.mode {
                Mode::Step => "step",
                Mode::Over(n) if depth <= n => "step",
                Mode::Out(n) if depth < n => "step",
                _ => return Ok(()),
            }
        };
        self.debug_pause(reason, None)
    }

    /// Stops: tells the editor where and what everything holds, then follows its commands.
    pub fn debug_pause(&mut self, reason: &str, message: Option<&str>) -> R<()> {
        let _ = self.out.flush();
        let d = self.debug.as_ref().unwrap();
        let mut budget = Budget(4000);
        let mut stack = Vec::new();
        for fr in self.frames.iter().rev() {
            if fr.src.name == "quick-try" {
                continue;
            }
            let name = match &fr.func {
                Some(f) => f.decl.name.as_deref().unwrap_or("FUNC").to_string(),
                None => String::new(),
            };
            let mut vars = Vec::new();
            if fr.func.is_some() {
                for sc in &fr.scopes {
                    vars.extend(crate::interp::vars_of(sc));
                }
            } else {
                // A file's top level: only its loop variables and FAIL names (its
                // other names are the globals).
                for sc in &fr.scopes {
                    vars.extend(crate::interp::vars_of(sc));
                }
            }
            stack.push(format!(
                "{{\"name\":{},\"path\":{},\"line\":{},\"vars\":{}}}",
                json_str(&name),
                json_str(&d.shown(&fr.src.canon)),
                fr.line,
                vars_json(&vars, &mut budget)
            ));
        }
        let top = self.frames.last().unwrap();
        let globals = crate::interp::vars_of(&top.env.borrow());
        let json = format!(
            "{{\"event\":\"paused\",\"reason\":\"{}\",\"path\":{},\"line\":{},\"message\":{},\"stack\":[{}],\"globals\":{}}}",
            reason,
            json_str(&d.shown(&top.src.canon)),
            top.line,
            message.map(json_str).unwrap_or_else(|| "null".into()),
            stack.join(","),
            vars_json(&globals, &mut budget)
        );
        emit(&json);
        self.debug_commands()?;
        emit("{\"event\":\"resumed\"}");
        Ok(())
    }

    /// Follows the editor's commands until one says to carry on.
    pub fn debug_commands(&mut self) -> R<()> {
        let depth = self.frames.len();
        loop {
            let cmd = match self.debug.as_mut().unwrap().next_command() {
                Some(c) => c,
                // The editor went away: end the program.
                None => return Err(Fault::End),
            };
            let (word, arg) = cmd.split_once(' ').unwrap_or((cmd.as_str(), ""));
            let d = self.debug.as_mut().unwrap();
            match word {
                "continue" => d.mode = Mode::Run,
                "step" => d.mode = Mode::Step,
                "over" => d.mode = if depth == 0 { Mode::Step } else { Mode::Over(depth) },
                "out" => d.mode = Mode::Out(depth),
                "stop" => return Err(Fault::End),
                "break" => {
                    d.set_breaks(arg);
                    continue;
                }
                "eval" => {
                    let (id, expr) = arg.split_once('\t').unwrap_or((arg, ""));
                    let reply = self.debug_eval(id, expr);
                    emit(&reply);
                    continue;
                }
                _ => continue,
            }
            // A new stop request made while stopped would stop again straight away.
            self.debug.as_ref().unwrap().pause_req.store(false, Ordering::SeqCst);
            return Ok(());
        }
    }

    fn debug_eval(&mut self, id: &str, expr: &str) -> String {
        let fail = |m: &str| format!("{{\"event\":\"eval\",\"id\":{},\"ok\":false,\"message\":{}}}", json_str(id), json_str(m));
        let toks = match lexer::lex(expr) {
            Ok(t) => t,
            Err(e) => return fail(&e.msg),
        };
        let prog = match Parser::new(toks).program() {
            Ok(p) => p,
            Err(e) => return fail(&e.msg),
        };
        let [s] = prog.as_slice() else { return fail("only one expression can be worked out here") };
        let StmtKind::Expr(e) = &s.kind else { return fail("only an expression can be worked out here, not a statement") };
        // Running code here mustn't stop at breakpoints, and mustn't move the current line.
        let saved = self.debug.take();
        let line = self.frames.last().map(|f| f.line).unwrap_or(0);
        let r = self.eval(e);
        if let Some(f) = self.frames.last_mut() {
            f.line = line;
        }
        self.debug = saved;
        match r {
            Ok(v) => format!(
                "{{\"event\":\"eval\",\"id\":{},\"ok\":true,\"value\":{}}}",
                json_str(id),
                value_json(expr, &v, 0, &mut Budget(4000))
            ),
            Err(Fault::Error(e)) => fail(&e.msg),
            Err(Fault::End) => fail("END can't be used here"),
        }
    }
}

/// Runs a program under the debugger (jlb --debug <file>).
pub fn run(path: &str) -> i32 {
    let (mut d, inputs) = start();
    let main = d.name_path(path);
    let mut it = Interp::new();
    it.ide = true;
    it.debug = Some(Box::new(d));
    it.debug_input = Some(inputs);
    emit("{\"event\":\"ready\"}");
    match it.debug_commands() {
        Ok(()) => it.run_main(&main),
        Err(_) => 0,
    }
}

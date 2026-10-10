// Walks the syntax tree and runs it.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{BufWriter, Stdout, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::ast::*;
use crate::builtins;
use crate::lexer;
use crate::number::{Number, Rounding};
use crate::ops::{self, truth};
use crate::parser::Parser;
use crate::value::*;

const MAX_DEPTH: usize = 2500;

pub enum Flow {
    Normal,
    Break,
    Skip,
    Return(Value),
    Goto(Rc<str>, usize),
}

pub struct Frame {
    /// None for a file's top level.
    pub func: Option<Rc<Function>>,
    /// For functions, scopes[0] holds its locals. Loop variables get their own scope.
    pub scopes: Vec<OrdMap>,
    pub env: Env,
    pub line: usize,
    pub src: Rc<Source>,
}

pub enum Key {
    Name(Rc<str>),
    Pos(i64),
}

pub struct Interp {
    pub frames: Vec<Frame>,
    pub out: BufWriter<Stdout>,
    pub start: Instant,
    pub last_delta: Instant,
    pub rng: u64,
    /// Running for an editor: output is sent right away and crashes add a JSON report.
    pub ide: bool,
    imported: HashSet<PathBuf>,
    loaded: HashMap<PathBuf, Value>,
}

fn key_from(v: &Value) -> R<Key> {
    match v {
        Value::Num(n) => {
            if !n.is_integer() {
                return err(format!("positions must be whole numbers, not {}", fmt_num(n)));
            }
            Ok(Key::Pos(n.to_i64_saturating()))
        }
        Value::Text(s) => Ok(Key::Name(s.clone())),
        o => err(format!("a {} can't be used as a position or a name", o.type_name())),
    }
}

/// 1-based position (negative counts from the end) to a 0-based index, if it exists.
pub fn resolve(p: i64, len: usize) -> Option<usize> {
    let len = len as i64;
    let i = if p > 0 {
        p - 1
    } else if p < 0 {
        len + p
    } else {
        return None;
    };
    if i >= 0 && i < len {
        Some(i as usize)
    } else {
        None
    }
}

/// Only tables and text can be changed in place, so only they are truly shared by `@`.
fn shareable(v: &Value) -> bool {
    matches!(v, Value::Table(..) | Value::Text(_))
}

fn vars_of(m: &OrdMap) -> Vec<(Rc<str>, Value)> {
    m.iter().filter(|(_, s)| !s.borrow().is_func()).map(|(k, s)| (k.clone(), s.borrow().clone())).collect()
}

fn find_point(stmts: &[Stmt], name: &str) -> Option<usize> {
    stmts.iter().position(|s| matches!(&s.kind, StmtKind::Point(p) if &**p == name))
}

fn fname(f: &Function) -> String {
    f.decl.name.as_deref().unwrap_or("FUNC").to_string()
}

fn table_entries(t: &Table) -> Vec<(Value, Slot)> {
    let mut v: Vec<(Value, Slot)> =
        t.items.iter().enumerate().map(|(i, s)| (Value::int(i as i64 + 1), s.clone())).collect();
    v.extend(t.named.iter().map(|(k, s)| (Value::Text(k.clone()), s.clone())));
    v
}

fn whole_val(v: &Value, what: &str) -> R<i64> {
    match v {
        Value::Num(n) if n.is_integer() => Ok(n.to_i64_saturating()),
        Value::Num(n) => err(format!("{} must be a whole number, not {}", what, fmt_num(n))),
        o => err(format!("{} must be a number, not a {}", what, o.type_name())),
    }
}

pub fn index_get(b: &Value, i: &Value) -> R<Value> {
    match b {
        Value::Table(t, _) => match key_from(i)? {
            Key::Pos(p) => Ok(resolve(p, t.items.len()).map(|x| t.item(x)).unwrap_or(Value::Null)),
            Key::Name(n) => Ok(t.named.get(&n).map(|s| s.borrow().clone()).unwrap_or(Value::Null)),
        },
        Value::Text(s) => match key_from(i)? {
            Key::Pos(p) => {
                let cs: Vec<char> = s.chars().collect();
                Ok(resolve(p, cs.len()).map(|x| Value::text(&cs[x].to_string())).unwrap_or(Value::Null))
            }
            Key::Name(_) => err("text positions must be numbers"),
        },
        Value::Null => err("can't use [ ] on NULL"),
        o => err(format!("can't use [ ] on a {}", o.type_name())),
    }
}

fn get_entry(bs: &Slot, key: &Key) -> R<Value> {
    let b = bs.borrow();
    let kv = match key {
        Key::Name(n) => Value::Text(n.clone()),
        Key::Pos(p) => Value::int(*p),
    };
    index_get(&b, &kv)
}

fn set_entry(bs: &Slot, key: &Key, v: Value) -> R<()> {
    let mut b = bs.borrow_mut();
    match &mut *b {
        Value::Table(rc, _) => {
            let t = Rc::make_mut(rc);
            match key {
                Key::Name(n) => {
                    if let Some(s) = t.named.get(n) {
                        *s.borrow_mut() = v;
                    } else {
                        t.named.insert(n.clone(), new_slot(v));
                    }
                    Ok(())
                }
                Key::Pos(p) => {
                    let len = t.items.len();
                    if let Some(i) = resolve(*p, len) {
                        *t.items[i].borrow_mut() = v;
                        return Ok(());
                    }
                    if *p == len as i64 + 1 {
                        t.items.push(new_slot(v));
                        return Ok(());
                    }
                    if *p == 0 {
                        return err("positions start at 1, so [0] can't be written to");
                    }
                    err(format!("position {} is past the end of the table (it has {} items)", p, len))
                }
            }
        }
        Value::Text(s) => {
            let p = match key {
                Key::Pos(p) => *p,
                Key::Name(_) => return err("text positions must be numbers"),
            };
            let mut cs: Vec<char> = s.chars().collect();
            match resolve(p, cs.len()) {
                Some(i) => {
                    let rep: Vec<char> = display(&v).chars().collect();
                    cs.splice(i..i + 1, rep);
                    *s = Rc::from(cs.into_iter().collect::<String>().as_str());
                    Ok(())
                }
                None if p == 0 => err("positions start at 1, so [0] can't be written to"),
                None => err(format!("position {} is past the end of the text", p)),
            }
        }
        Value::Null => err("can't set an entry of NULL"),
        o => err(format!("a {} has no entries to set", o.type_name())),
    }
}

fn set_entry_slot(bs: &Slot, key: &Key, sh: Slot) -> R<()> {
    let mut b = bs.borrow_mut();
    match &mut *b {
        Value::Table(rc, _) => {
            let t = Rc::make_mut(rc);
            match key {
                Key::Name(n) => {
                    t.named.insert(n.clone(), sh);
                    Ok(())
                }
                Key::Pos(p) => {
                    let len = t.items.len();
                    if let Some(i) = resolve(*p, len) {
                        t.items[i] = sh;
                        Ok(())
                    } else if *p == len as i64 + 1 {
                        t.items.push(sh);
                        Ok(())
                    } else {
                        err(format!("position {} is outside the table", p))
                    }
                }
            }
        }
        o => err(format!("@= can only share into a table, not a {}", o.type_name())),
    }
}

fn insert_index(p: i64, len: usize) -> R<usize> {
    let q = if p > 0 {
        p
    } else if p < 0 {
        len as i64 + 2 + p
    } else {
        return err("positions start at 1, so 0 isn't a position");
    };
    if q >= 1 && q <= len as i64 + 1 {
        Ok((q - 1) as usize)
    } else {
        err(format!("position {} is outside (there are {} items)", p, len))
    }
}

fn apply_action(v: &mut Value, insert: bool, extra: &[Value]) -> R<()> {
    let name = if insert { "INSERT" } else { "DROP" };
    let p = whole_val(if insert { &extra[1] } else { &extra[0] }, "the position")?;
    match v {
        Value::Table(rc, _) => {
            let t = Rc::make_mut(rc);
            let len = t.items.len();
            if insert {
                let i = insert_index(p, len)?;
                t.items.insert(i, new_slot(copy_val(extra[0].clone())));
            } else {
                match resolve(p, len) {
                    Some(i) => {
                        t.items.remove(i);
                    }
                    None => return err(format!("DROP: position {} doesn't exist (there are {} items)", p, len)),
                }
            }
            Ok(())
        }
        Value::Text(s) => {
            let mut cs: Vec<char> = s.chars().collect();
            let len = cs.len();
            if insert {
                let i = insert_index(p, len)?;
                let add: Vec<char> = display(&extra[0]).chars().collect();
                cs.splice(i..i, add);
            } else {
                match resolve(p, len) {
                    Some(i) => {
                        cs.remove(i);
                    }
                    None => return err(format!("DROP: position {} doesn't exist (the text has {} characters)", p, len)),
                }
            }
            *s = Rc::from(cs.into_iter().collect::<String>().as_str());
            Ok(())
        }
        o => err(format!("{} works on tables and text, not a {}", name, o.type_name())),
    }
}

impl Default for Interp {
    fn default() -> Self {
        Self::new()
    }
}

impl Interp {
    pub fn new() -> Self {
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(42);
        Interp {
            frames: Vec::new(),
            out: BufWriter::new(std::io::stdout()),
            start: Instant::now(),
            last_delta: Instant::now(),
            rng: seed ^ 0x9E37_79B9_7F4A_7C15,
            ide: false,
            imported: HashSet::new(),
            loaded: HashMap::new(),
        }
    }

    fn frame(&self) -> &Frame {
        self.frames.last().unwrap()
    }
    fn frame_mut(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    // ---------- files ----------

    pub fn run_main(&mut self, path: &Path) -> i32 {
        let env: Env = Rc::new(RefCell::new(OrdMap::new()));
        if let Ok(c) = path.canonicalize() {
            self.imported.insert(c);
        }
        let r = self.run_module(path, env);
        let _ = self.out.flush();
        match r {
            Ok(()) | Err(Fault::End) => 0,
            Err(Fault::Error(e)) => {
                eprintln!("{}", report(&e));
                if self.ide {
                    eprintln!("{}{}", crate::ide::MARK, crate::ide::report_json(&e));
                }
                1
            }
        }
    }

    // ---------- Quick-try sessions (jlb --repl) ----------

    pub fn start_session(&mut self, dir: PathBuf) {
        let env: Env = Rc::new(RefCell::new(OrdMap::new()));
        let src = Rc::new(Source { name: "quick-try".into(), dir });
        self.frames.push(Frame { func: None, scopes: Vec::new(), env, line: 0, src });
    }

    /// Runs a snippet in the session. A single expression gives back its value.
    pub fn run_snippet(&mut self, text: &str) -> R<Option<Value>> {
        // Tidy up after an earlier error, which can leave loop scopes or calls behind.
        self.frames.truncate(1);
        self.frames[0].scopes.clear();
        let syntax = |msg: String, line: usize| {
            Fault::Error(Box::new(ErrorInfo {
                msg,
                line,
                file: "quick-try".into(),
                captured: true,
                traced: false,
                frames: Vec::new(),
                outside: Vec::new(),
            }))
        };
        let toks = lexer::lex(text).map_err(|e| syntax(e.msg, e.line))?;
        let prog = Parser::new(toks).program().map_err(|e| syntax(e.msg, e.line))?;
        if let [s] = prog.as_slice() {
            if let StmtKind::Expr(e) = &s.kind {
                self.frame_mut().line = s.line;
                return match self.eval(e) {
                    // A command that gives back nothing (like PRINT) shows no result.
                    Ok(Value::Null) if matches!(e, Expr::Call(..)) => Ok(None),
                    Ok(v) => Ok(Some(v)),
                    Err(mut f) => {
                        self.capture(&mut f, s.line);
                        Err(f)
                    }
                };
            }
        }
        match self.exec_block(&prog)? {
            Flow::Goto(n, line) => Err(self.goto_error(&n, line)),
            _ => Ok(None),
        }
    }

    fn load_program(&mut self, path: &Path) -> R<(Vec<Stmt>, Rc<Source>)> {
        let text = std::fs::read_to_string(path).map_err(|e| fault(format!("couldn't open {}: {}", path.display(), e)))?;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let syntax = |msg: String, line: usize| {
            Fault::Error(Box::new(ErrorInfo {
                msg,
                line,
                file: name.clone(),
                captured: true,
                traced: false,
                frames: Vec::new(),
                outside: Vec::new(),
            }))
        };
        let toks = lexer::lex(&text).map_err(|e| syntax(e.msg, e.line))?;
        let prog = Parser::new(toks).program().map_err(|e| syntax(e.msg, e.line))?;
        Ok((prog, Rc::new(Source { name: name.clone(), dir })))
    }

    fn run_module(&mut self, path: &Path, env: Env) -> R<()> {
        let (prog, src) = self.load_program(path)?;
        if self.frames.len() > 200 {
            return err("too many files loading each other");
        }
        self.frames.push(Frame { func: None, scopes: Vec::new(), env, line: 0, src });
        let r = match self.exec_block(&prog) {
            Ok(Flow::Goto(n, line)) => Err(self.goto_error(&n, line)),
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        };
        self.frames.pop();
        r
    }

    fn file_path(&self, p: &str) -> R<PathBuf> {
        let path = self.frame().src.dir.join(p);
        path.canonicalize().map_err(|_| fault(format!("couldn't find the file {}", p)))
    }

    fn do_import(&mut self, p: &str) -> R<()> {
        let path = self.file_path(p)?;
        if !self.imported.insert(path.clone()) {
            return Ok(());
        }
        let env = self.frame().env.clone();
        self.run_module(&path, env)
    }

    fn do_load(&mut self, p: &str) -> R<()> {
        let path = self.file_path(p)?;
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase().replace(['-', ' '], "_"))
            .unwrap_or_default();
        if !is_ident(&stem) {
            return err(format!("LOAD can't make a name out of the file name {}", p));
        }
        let v = match self.loaded.get(&path) {
            Some(v) => v.clone(),
            None => {
                let env: Env = Rc::new(RefCell::new(OrdMap::new()));
                self.run_module(&path, env.clone())?;
                let mut t = Table::new();
                for (k, s) in env.borrow().iter() {
                    t.named.insert(k.clone(), new_slot(s.borrow().clone()));
                }
                let v = Value::new_table(t);
                self.loaded.insert(path, v.clone());
                v
            }
        };
        self.set_var(&Rc::from(stem.as_str()), copy_val(v));
        Ok(())
    }

    // ---------- errors ----------

    fn capture(&mut self, f: &mut Fault, line: usize) {
        let Fault::Error(e) = f else { return };
        if e.captured {
            return;
        }
        self.frame_mut().line = line;
        e.captured = true;
        e.traced = true;
        e.line = line;
        e.file = self.frame().src.name.clone();
        for fr in self.frames.iter().rev() {
            if let Some(func) = &fr.func {
                let mut vars = Vec::new();
                for sc in &fr.scopes {
                    vars.extend(vars_of(sc));
                }
                e.frames.push((fname(func), fr.line, vars));
            }
        }
        e.outside = vars_of(&self.frame().env.borrow());
    }

    fn goto_error(&mut self, name: &str, line: usize) -> Fault {
        let mut f = fault(format!(
            "GOTO {}: there's no POINT {} it can reach (GOTO can't jump into a block or out of a function)",
            name, name
        ));
        self.capture(&mut f, line);
        f
    }

    fn error_table(&self, e: &ErrorInfo) -> Value {
        let mut t = Table::new();
        t.named.insert("message".into(), new_slot(Value::text(&e.msg)));
        t.named.insert("line".into(), new_slot(Value::int(e.line as i64)));
        t.named.insert("file".into(), new_slot(Value::text(&e.file)));
        let mut vars = Table::new();
        if let Some(f) = e.frames.first() {
            for (k, v) in &f.2 {
                vars.named.insert(k.clone(), new_slot(v.clone()));
            }
        }
        for (k, v) in &e.outside {
            if vars.named.get(k).is_none() {
                vars.named.insert(k.clone(), new_slot(v.clone()));
            }
        }
        t.named.insert("variables".into(), new_slot(Value::new_table(vars)));
        t.report = Some(Rc::from(report(e).as_str()));
        Value::new_table(t)
    }

    // ---------- variables ----------

    pub fn lookup(&self, name: &str) -> Option<Slot> {
        let f = self.frame();
        for s in f.scopes.iter().rev() {
            if let Some(x) = s.get(name) {
                return Some(x.clone());
            }
        }
        let r = f.env.borrow().get(name).cloned();
        r
    }

    fn create_var(&mut self, name: Rc<str>, s: Slot) {
        let f = self.frame_mut();
        if f.func.is_some() {
            f.scopes[0].insert(name, s);
        } else {
            f.env.borrow_mut().insert(name, s);
        }
    }

    pub fn set_var(&mut self, name: &Rc<str>, v: Value) {
        match self.lookup(name) {
            Some(s) => *s.borrow_mut() = v,
            None => self.create_var(name.clone(), new_slot(v)),
        }
    }

    fn bind_var(&mut self, name: &Rc<str>, s: Slot) {
        let f = self.frame_mut();
        for sc in f.scopes.iter_mut().rev() {
            if sc.get(name).is_some() {
                sc.insert(name.clone(), s);
                return;
            }
        }
        let in_env = f.env.borrow().get(name).is_some();
        if in_env {
            f.env.borrow_mut().insert(name.clone(), s);
            return;
        }
        self.create_var(name.clone(), s)
    }

    fn remove_var(&mut self, name: &str) -> bool {
        let f = self.frame_mut();
        for sc in f.scopes.iter_mut().rev() {
            if sc.remove(name).is_some() {
                return true;
            }
        }
        let r = f.env.borrow_mut().remove(name).is_some();
        r
    }

    fn func_frame(&self, f: &Rc<Function>) -> R<usize> {
        for (i, fr) in self.frames.iter().enumerate().rev() {
            if let Some(g) = &fr.func {
                if Rc::ptr_eq(f, g) {
                    return Ok(i);
                }
            }
        }
        err(format!("{} isn't running, so its local variables don't exist right now", fname(f)))
    }

    fn func_local(&self, f: &Rc<Function>, name: &str) -> R<Slot> {
        let i = self.func_frame(f)?;
        for sc in self.frames[i].scopes.iter().rev() {
            if let Some(s) = sc.get(name) {
                return Ok(s.clone());
            }
        }
        err(format!("{} has no local variable named {}", fname(f), name))
    }

    fn make_func(&self, d: &Rc<FuncDecl>) -> Value {
        let f = self.frame();
        Value::Func(Rc::new(Function { decl: d.clone(), env: f.env.clone(), src: f.src.clone() }))
    }

    // ---------- statements ----------

    pub fn exec_block(&mut self, stmts: &[Stmt]) -> R<Flow> {
        let mut i = 0;
        while i < stmts.len() {
            match self.exec_stmt(&stmts[i])? {
                Flow::Normal => i += 1,
                Flow::Goto(name, line) => match find_point(stmts, &name) {
                    Some(j) => i = j + 1,
                    None => return Ok(Flow::Goto(name, line)),
                },
                f => return Ok(f),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, s: &Stmt) -> R<Flow> {
        self.frame_mut().line = s.line;
        match self.exec_kind(s) {
            Err(mut f) => {
                self.capture(&mut f, s.line);
                Err(f)
            }
            ok => ok,
        }
    }

    fn loop_body(&mut self, body: &[Stmt]) -> R<Option<Flow>> {
        // None = keep looping, Some(Normal) = BREAK, Some(other) = leave with it
        match self.exec_block(body)? {
            Flow::Normal | Flow::Skip => Ok(None),
            Flow::Break => Ok(Some(Flow::Normal)),
            f => Ok(Some(f)),
        }
    }

    fn exec_kind(&mut self, s: &Stmt) -> R<Flow> {
        match &s.kind {
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(Flow::Normal)
            }
            StmtKind::Assign(t, op, rhs) => {
                self.assign(t, op, rhs)?;
                Ok(Flow::Normal)
            }
            StmtKind::Local(name, e) => {
                let v = match e {
                    Some(e) => copy_val(self.eval(e)?),
                    None => Value::Null,
                };
                self.frame_mut().scopes[0].insert(name.clone(), new_slot(v));
                Ok(Flow::Normal)
            }
            StmtKind::If(branches, otherwise) => {
                for (c, b) in branches {
                    let v = self.eval(c)?;
                    if truth(&v)? {
                        return self.exec_block(b);
                    }
                }
                match otherwise {
                    Some(b) => self.exec_block(b),
                    None => Ok(Flow::Normal),
                }
            }
            StmtKind::While(c, b) => {
                loop {
                    let v = self.eval(c)?;
                    if !truth(&v)? {
                        break;
                    }
                    if let Some(f) = self.loop_body(b)? {
                        return Ok(f);
                    }
                }
                Ok(Flow::Normal)
            }
            StmtKind::Repeat(n, b) => {
                let v = self.eval(n)?;
                let count = match v {
                    Value::Num(x) if x.is_neg() => return err("REPEAT can't run a negative number of times"),
                    Value::Num(x) if !x.is_integer() => return err("REPEAT needs a whole number of times"),
                    Value::Num(x) => x.to_i64_saturating() as u64,
                    o => return err(format!("REPEAT needs a number, not a {}", o.type_name())),
                };
                for _ in 0..count {
                    if let Some(f) = self.loop_body(b)? {
                        return Ok(f);
                    }
                }
                Ok(Flow::Normal)
            }
            StmtKind::ForTo(var, a, b, step, body) => self.for_to(var, a, b, step.as_ref(), body),
            StmtKind::ForIn(a, b, iter, body) => self.for_in(a, b.as_ref(), iter, body),
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Skip => Ok(Flow::Skip),
            StmtKind::Return(e) => {
                let v = match e {
                    Some(e) => copy_val(self.eval(e)?),
                    None => Value::Null,
                };
                Ok(Flow::Return(v))
            }
            StmtKind::End => Err(Fault::End),
            StmtKind::FuncDef(d) => {
                let f = self.make_func(d);
                self.set_var(d.name.as_ref().unwrap(), f);
                Ok(Flow::Normal)
            }
            StmtKind::Try(body, name, fail) => match self.exec_block(body) {
                Err(Fault::Error(e)) => {
                    let t = self.error_table(&e);
                    let mut sc = OrdMap::new();
                    sc.insert(name.clone(), new_slot(t));
                    self.frame_mut().scopes.push(sc);
                    let r = self.exec_block(fail);
                    self.frame_mut().scopes.pop();
                    r
                }
                other => other,
            },
            StmtKind::Goto(n) => Ok(Flow::Goto(n.clone(), s.line)),
            StmtKind::Point(_) => Ok(Flow::Normal),
            StmtKind::Import(p) => {
                self.do_import(p)?;
                Ok(Flow::Normal)
            }
            StmtKind::Load(p) => {
                self.do_load(p)?;
                Ok(Flow::Normal)
            }
        }
    }

    fn loop_num(&mut self, e: &Expr, what: &str) -> R<Number> {
        match self.eval(e)? {
            Value::Num(n) => Ok(n),
            o => err(format!("the {} of a counting loop must be a number, not a {}", what, o.type_name())),
        }
    }

    fn for_to(&mut self, var: &Rc<str>, a: &Expr, b: &Expr, step: Option<&Expr>, body: &[Stmt]) -> R<Flow> {
        let start = self.loop_num(a, "start")?;
        let end = self.loop_num(b, "end")?;
        let st = match step {
            Some(e) => self.loop_num(e, "STEP")?,
            None => Number::int(1),
        };
        if st.is_zero() {
            return err("STEP 0 would loop forever");
        }
        let up = start.compare(&end).is_le();
        let st = if up { st.abs() } else { st.abs().neg() };
        // Exact numbers count exactly. Approximate ones get a little tidying.
        let approx = start.is_float() || end.is_float() || st.is_float();
        let count = if approx {
            let (s0, e0, t0) = (start.to_f64(), end.to_f64(), st.to_f64());
            (((e0 - s0) / t0) + 1e-9).floor() as i64 + 1
        } else {
            end.sub(&start)?.div(&st)?.round(0, Rounding::Floor)?.to_i64_saturating().saturating_add(1)
        };
        self.frame_mut().scopes.push(OrdMap::new());
        let mut result = Ok(Flow::Normal);
        let mut v = start.clone();
        for k in 0..count.max(0) {
            if k > 0 {
                let next = if approx {
                    Number::from_f64_rounded(start.to_f64() + k as f64 * st.to_f64(), 12)
                } else {
                    v.add(&st)
                };
                match next {
                    Ok(n) => v = n,
                    Err(e) => {
                        result = Err(e);
                        break;
                    }
                }
            }
            self.frame_mut().scopes.last_mut().unwrap().insert(var.clone(), new_slot(Value::Num(v.clone())));
            match self.loop_body(body) {
                Ok(None) => {}
                Ok(Some(f)) => {
                    result = Ok(f);
                    break;
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        self.frame_mut().scopes.pop();
        result
    }

    fn loop_entries(&mut self, iter: &Expr, want_slots: bool) -> R<Vec<(Value, Slot)>> {
        if want_slots && is_lvalue(iter) {
            let s = self.lvalue_slot(iter)?;
            let mut v = s.borrow_mut();
            if let Value::Table(rc, _) = &mut *v {
                return Ok(table_entries(Rc::make_mut(rc)));
            }
        }
        match self.eval(iter)? {
            Value::Table(rc, _) => Ok(table_entries(&rc)),
            Value::Text(s) => Ok(s
                .chars()
                .enumerate()
                .map(|(i, c)| (Value::int(i as i64 + 1), new_slot(Value::text(&c.to_string()))))
                .collect()),
            o => err(format!("FOR ... IN needs a table or text, not a {}", o.type_name())),
        }
    }

    fn for_in(&mut self, a: &LoopVar, b: Option<&LoopVar>, iter: &Expr, body: &[Stmt]) -> R<Flow> {
        let want = a.shared || b.is_some_and(|x| x.shared);
        let entries = self.loop_entries(iter, want)?;
        let bind = |lv: &LoopVar, s: &Slot| -> Slot {
            if lv.shared && shareable(&s.borrow()) {
                s.clone()
            } else {
                new_slot(copy_val(s.borrow().clone()))
            }
        };
        self.frame_mut().scopes.push(OrdMap::new());
        let mut result = Ok(Flow::Normal);
        for (key, slot) in entries {
            {
                let sc = self.frame_mut().scopes.last_mut().unwrap();
                match b {
                    None => sc.insert(a.name.clone(), bind(a, &slot)),
                    Some(bv) => {
                        sc.insert(a.name.clone(), new_slot(key));
                        sc.insert(bv.name.clone(), bind(bv, &slot));
                    }
                }
            }
            match self.loop_body(body) {
                Ok(None) => {}
                Ok(Some(f)) => {
                    result = Ok(f);
                    break;
                }
                Err(e) => {
                    result = Err(e);
                    break;
                }
            }
        }
        self.frame_mut().scopes.pop();
        result
    }

    // ---------- assignment ----------

    fn share_slot(&mut self, e: &Expr) -> R<Slot> {
        if is_lvalue(e) {
            let s = self.lvalue_slot(e)?;
            if shareable(&s.borrow()) {
                return Ok(s);
            }
            let v = s.borrow().clone();
            return Ok(new_slot(v));
        }
        Ok(new_slot(self.eval(e)?))
    }

    fn lvalue_slot(&mut self, e: &Expr) -> R<Slot> {
        match e {
            Expr::Var(n) => self.lookup(n).map_or_else(|| err(format!("{} isn't defined", n)), Ok),
            Expr::Field(b, k) => {
                let bs = self.lvalue_slot(b)?;
                self.entry_slot(&bs, &Key::Name(k.clone()))
            }
            Expr::Index(b, i) => {
                let bs = self.lvalue_slot(b)?;
                let k = key_from(&self.eval(i)?)?;
                self.entry_slot(&bs, &k)
            }
            _ => err("this can't be changed"),
        }
    }

    fn slot_func(bs: &Slot) -> Option<Rc<Function>> {
        match &*bs.borrow() {
            Value::Func(f) => Some(f.clone()),
            _ => None,
        }
    }

    fn entry_slot(&mut self, bs: &Slot, key: &Key) -> R<Slot> {
        if let Some(f) = Self::slot_func(bs) {
            return match key {
                Key::Name(n) => self.func_local(&f, n),
                Key::Pos(_) => err("use .name to reach a function's local variable"),
            };
        }
        let mut v = bs.borrow_mut();
        match &mut *v {
            Value::Table(rc, _) => {
                let t = Rc::make_mut(rc);
                match key {
                    Key::Name(n) => t.named.get(n).cloned().map_or_else(|| err(format!("the table has no entry named {}", n)), Ok),
                    Key::Pos(p) => match resolve(*p, t.items.len()) {
                        Some(i) => Ok(t.items[i].clone()),
                        None => err(format!("position {} is outside the table", p)),
                    },
                }
            }
            Value::Text(_) => err("a single character of text can't be shared or changed this way"),
            o => err(format!("a {} has no entries", o.type_name())),
        }
    }

    fn assign(&mut self, target: &Expr, op: &str, rhs: &Expr) -> R<()> {
        match target {
            Expr::Var(n) => {
                if op == "@=" {
                    let s = self.share_slot(rhs)?;
                    self.bind_var(n, s);
                    return Ok(());
                }
                if op == "=" {
                    let v = copy_val(self.eval(rhs)?);
                    self.set_var(n, v);
                    return Ok(());
                }
                let slot = self.lookup(n).map_or_else(|| err(format!("{} isn't defined", n)), Ok)?;
                let r = self.eval(rhs)?;
                self.compound(&slot, op, r)
            }
            Expr::Field(b, k) => {
                let bs = self.lvalue_slot(b)?;
                self.assign_entry(&bs, Key::Name(k.clone()), op, rhs)
            }
            Expr::Index(b, i) => {
                let bs = self.lvalue_slot(b)?;
                let k = key_from(&self.eval(i)?)?;
                self.assign_entry(&bs, k, op, rhs)
            }
            _ => err("this can't be assigned to"),
        }
    }

    fn compound(&mut self, slot: &Slot, op: &str, r: Value) -> R<()> {
        let base = &op[..op.len() - 1];
        // Fast path: `table += value` adds to the end in place.
        if base == "+" && !matches!(r, Value::Table(..)) {
            let mut v = slot.borrow_mut();
            if let Value::Table(rc, _) = &mut *v {
                Rc::make_mut(rc).items.push(new_slot(copy_val(r)));
                return Ok(());
            }
        }
        let cur = slot.borrow().clone();
        let nv = ops::binop(base, cur, r)?;
        *slot.borrow_mut() = nv;
        Ok(())
    }

    fn assign_entry(&mut self, bs: &Slot, key: Key, op: &str, rhs: &Expr) -> R<()> {
        if let Some(f) = Self::slot_func(bs) {
            let n = match &key {
                Key::Name(n) => n.clone(),
                Key::Pos(_) => return err("use .name to reach a function's local variable"),
            };
            let s = self.func_local(&f, &n)?;
            if op == "=" || op == "@=" {
                let v = copy_val(self.eval(rhs)?);
                *s.borrow_mut() = v;
                return Ok(());
            }
            let r = self.eval(rhs)?;
            return self.compound(&s, op, r);
        }
        if op == "@=" {
            let sh = self.share_slot(rhs)?;
            return set_entry_slot(bs, &key, sh);
        }
        let r = self.eval(rhs)?;
        if op == "=" {
            return set_entry(bs, &key, copy_val(r));
        }
        let cur = get_entry(bs, &key)?;
        let nv = ops::binop(&op[..op.len() - 1], cur, r)?;
        set_entry(bs, &key, nv)
    }

    fn undefine(&mut self, t: &Expr) -> R<()> {
        match t {
            Expr::Var(n) => {
                if self.remove_var(n) {
                    Ok(())
                } else {
                    err(format!("{} isn't defined, so it can't be UNDEFINEd", n))
                }
            }
            Expr::Field(b, k) => {
                let bs = self.lvalue_slot(b)?;
                self.remove_entry(&bs, Key::Name(k.clone()))
            }
            Expr::Index(b, i) => {
                let bs = self.lvalue_slot(b)?;
                let k = key_from(&self.eval(i)?)?;
                self.remove_entry(&bs, k)
            }
            _ => err("UNDEFINE needs a name"),
        }
    }

    fn remove_entry(&mut self, bs: &Slot, key: Key) -> R<()> {
        if let Some(f) = Self::slot_func(bs) {
            let Key::Name(n) = key else { return err("use .name to reach a function's local variable") };
            let i = self.func_frame(&f)?;
            for sc in self.frames[i].scopes.iter_mut().rev() {
                if sc.remove(&n).is_some() {
                    return Ok(());
                }
            }
            return err(format!("{} has no local variable named {}", fname(&f), n));
        }
        let mut v = bs.borrow_mut();
        match &mut *v {
            Value::Table(rc, _) => {
                let t = Rc::make_mut(rc);
                match key {
                    Key::Name(n) => match t.named.remove(&n) {
                        Some(_) => Ok(()),
                        None => err(format!("the table has no entry named {} to UNDEFINE", n)),
                    },
                    Key::Pos(p) => match resolve(p, t.items.len()) {
                        Some(i) => {
                            t.items.remove(i);
                            Ok(())
                        }
                        None => err(format!("position {} doesn't exist", p)),
                    },
                }
            }
            o => err(format!("a {} has no entries to UNDEFINE", o.type_name())),
        }
    }

    // ---------- expressions ----------

    pub fn eval(&mut self, e: &Expr) -> R<Value> {
        match e {
            Expr::Num(n) => Ok(Value::Num(n.clone())),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Var(n) => match self.lookup(n) {
                Some(s) => {
                    let v = s.borrow().clone();
                    Ok(v)
                }
                None => err(format!("{} isn't defined", n)),
            },
            Expr::Builtin(b) => Ok(Value::Builtin(b)),
            Expr::Table(items) => {
                let mut t = Table::new();
                for (k, ex) in items {
                    let v = copy_val(self.eval(ex)?);
                    match k {
                        Some(k) => t.named.insert(k.clone(), new_slot(v)),
                        None => t.items.push(new_slot(v)),
                    }
                }
                Ok(Value::new_table(t))
            }
            Expr::Field(b, k) => {
                let bv = self.eval(b)?;
                match bv {
                    Value::Table(t, _) => Ok(t.named.get(k).map(|s| s.borrow().clone()).unwrap_or(Value::Null)),
                    Value::Func(f) => {
                        let s = self.func_local(&f, k)?;
                        let v = s.borrow().clone();
                        Ok(v)
                    }
                    Value::Null => err(format!("can't read .{} of NULL", k)),
                    o => err(format!("can't read .{} of a {}", k, o.type_name())),
                }
            }
            Expr::Index(b, i) => {
                let bv = self.eval(b)?;
                let iv = self.eval(i)?;
                index_get(&bv, &iv)
            }
            Expr::Slice(b, x, y, st) => {
                let bv = self.eval(b)?;
                let a = whole_val(&self.eval(x)?, "the slice start")?;
                let z = whole_val(&self.eval(y)?, "the slice end")?;
                let step = match st {
                    Some(s) => whole_val(&self.eval(s)?, "the slice STEP")?,
                    None => 1,
                };
                slice(&bv, a, z, step)
            }
            Expr::Call(c, args) => self.call(c, args),
            Expr::Unary(op, x) => {
                let v = self.eval(x)?;
                if *op == "-" {
                    ops::negate(v)
                } else {
                    Ok(Value::Bool(!truth(&v)?))
                }
            }
            Expr::Binary(op, a, b) => self.binary(op, a, b),
            Expr::Func(d) => Ok(self.make_func(d)),
            Expr::Undefine(t) => {
                self.undefine(t)?;
                Ok(Value::Null)
            }
            Expr::Insert(args) => self.action(true, args),
            Expr::Drop(args) => self.action(false, args),
        }
    }

    fn binary(&mut self, op: &str, a: &Expr, b: &Expr) -> R<Value> {
        let t = |me: &mut Self, e: &Expr| -> R<bool> {
            let v = me.eval(e)?;
            truth(&v)
        };
        let res = match op {
            "&" | "!&" => {
                let r = t(self, a)? && t(self, b)?;
                if op == "!&" { !r } else { r }
            }
            "&&" | "!&&" => {
                let l = t(self, a)?;
                let r = t(self, b)?;
                if op == "!&&" { !(l && r) } else { l && r }
            }
            "|" | "!|" => {
                let r = t(self, a)? || t(self, b)?;
                if op == "!|" { !r } else { r }
            }
            "||" | "!||" => {
                let l = t(self, a)?;
                let r = t(self, b)?;
                if op == "!||" { !(l || r) } else { l || r }
            }
            "~" | "!~" => {
                let l = t(self, a)?;
                let r = t(self, b)?;
                if op == "!~" { l == r } else { l != r }
            }
            "==" | "!==" | "===" | "!===" | "<" | ">" | "<=" | ">=" | "<==" | ">==" => {
                let x = self.eval(a)?;
                let y = self.eval(b)?;
                match op {
                    "==" => ops::loose_eq(&x, &y),
                    "!==" => !ops::loose_eq(&x, &y),
                    "===" => ops::strict_eq(&x, &y),
                    "!===" => !ops::strict_eq(&x, &y),
                    _ => ops::compare(op, &x, &y)?,
                }
            }
            _ => {
                let x = self.eval(a)?;
                let y = self.eval(b)?;
                return ops::binop(op, x, y);
            }
        };
        Ok(Value::Bool(res))
    }

    fn call(&mut self, c: &Expr, args: &[Expr]) -> R<Value> {
        let f = self.eval(c)?;
        match f {
            Value::Builtin(name) => {
                let mut v = Vec::with_capacity(args.len());
                for a in args {
                    v.push(self.eval(a)?);
                }
                builtins::call(self, name, v)
            }
            Value::Func(func) => self.call_func(func, args),
            Value::Null => err("can't call NULL like a function"),
            o => err(format!("a {} can't be called like a function", o.type_name())),
        }
    }

    fn call_func(&mut self, func: Rc<Function>, args: &[Expr]) -> R<Value> {
        let decl = func.decl.clone();
        let params = &decl.params;
        let has_rest = params.last().is_some_and(|p| p.rest);
        let fixed = params.len() - has_rest as usize;
        if args.len() > fixed && !has_rest {
            return err(format!(
                "{} takes at most {} input{}, but got {}",
                fname(&func),
                fixed,
                if fixed == 1 { "" } else { "s" },
                args.len()
            ));
        }
        let mut vals: Vec<Slot> = Vec::with_capacity(args.len());
        for (i, a) in args.iter().enumerate() {
            if i < fixed && params[i].shared {
                vals.push(self.share_slot(a)?);
            } else {
                let v = self.eval(a)?;
                vals.push(new_slot(copy_val(v)));
            }
        }
        if self.frames.len() >= MAX_DEPTH {
            return err("too many function calls inside each other (is a function calling itself forever?)");
        }
        let line = self.frame().line;
        self.frames.push(Frame {
            func: Some(func.clone()),
            scopes: vec![OrdMap::new()],
            env: func.env.clone(),
            line,
            src: func.src.clone(),
        });
        let r = self.run_call(&func, &decl, vals);
        self.frames.pop();
        r
    }

    fn run_call(&mut self, func: &Rc<Function>, decl: &FuncDecl, vals: Vec<Slot>) -> R<Value> {
        let mut it = vals.into_iter();
        for p in &decl.params {
            if p.rest {
                let rest: Vec<Value> = it.by_ref().map(|s| s.borrow().clone()).collect();
                let t = Value::new_table(Table::from_values(rest));
                self.frame_mut().scopes[0].insert(p.name.clone(), new_slot(t));
                break;
            }
            let given = it.next();
            let is_null = given.as_ref().is_none_or(|s| matches!(*s.borrow(), Value::Null));
            let slot = if !is_null {
                given.unwrap()
            } else if let Some(d) = &p.default {
                let v = copy_val(self.eval(d)?);
                new_slot(v)
            } else if let Some(s) = given {
                s
            } else {
                return err(format!("{} is required (it's an input of {})", p.name, fname(func)));
            };
            self.frame_mut().scopes[0].insert(p.name.clone(), slot);
        }
        let body = decl.body.clone();
        match self.exec_block(&body)? {
            Flow::Return(v) => Ok(v),
            Flow::Goto(n, line) => Err(self.goto_error(&n, line)),
            _ => Ok(Value::Null),
        }
    }

    fn action(&mut self, insert: bool, args: &[Expr]) -> R<Value> {
        let (name, want) = if insert { ("INSERT", 3) } else { ("DROP", 2) };
        if args.len() != want {
            return err(format!("{} takes {} inputs, but got {}", name, want, args.len()));
        }
        let mut extra = Vec::new();
        for a in &args[1..] {
            extra.push(self.eval(a)?);
        }
        if is_lvalue(&args[0]) {
            let s = self.lvalue_slot(&args[0])?;
            let mut v = s.borrow_mut();
            apply_action(&mut v, insert, &extra)?;
            return Ok(copy_val(v.clone()));
        }
        let mut v = self.eval(&args[0])?;
        apply_action(&mut v, insert, &extra)?;
        Ok(v)
    }
}

pub fn slice(bv: &Value, a: i64, z: i64, step: i64) -> R<Value> {
    if step == 0 {
        return err("a slice STEP can't be 0");
    }
    let chars: Vec<char>;
    let len = match bv {
        Value::Table(t, _) => {
            chars = Vec::new();
            t.items.len()
        }
        Value::Text(s) => {
            chars = s.chars().collect();
            chars.len()
        }
        o => return err(format!("can't take a slice of a {}", o.type_name())),
    } as i64;
    let fix = |p: i64| if p < 0 { len + 1 + p } else { p };
    let (a, z) = (fix(a), fix(z));
    let dir = if a <= z { 1 } else { -1 };
    let st = step.abs() * dir;
    let mut idx = Vec::new();
    let mut p = a;
    if dir > 0 && p < 1 {
        p += ((1 - p + st - 1) / st) * st;
    }
    if dir < 0 && p > len {
        let d = -st;
        p -= ((p - len + d - 1) / d) * d;
    }
    while (dir > 0 && p <= z && p <= len) || (dir < 0 && p >= z && p >= 1) {
        if p >= 1 && p <= len {
            idx.push((p - 1) as usize);
        }
        p += st;
    }
    match bv {
        Value::Table(t, _) => Ok(Value::new_table(Table::from_values(idx.iter().map(|&i| copy_val(t.item(i))).collect()))),
        _ => Ok(Value::text(&idx.iter().map(|&i| chars[i]).collect::<String>())),
    }
}

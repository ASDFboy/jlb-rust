// Modes used by editors and the JLB IDE. Every reply is one line of JSON, so any
// editor can read it without guessing. The C interpreter (jlb-c) offers the same modes.
//
//   jlb --check <file | ->   Check the syntax without running anything.
//   jlb --ide <file>         Run a program for an editor: output is sent as soon as it's
//                            printed, and a crash also prints a JSON report.
//   jlb --repl               A Quick-try session that keeps its variables between snippets.

use std::io::{BufRead, Read, Write};
use std::path::PathBuf;

use crate::interp::Interp;
use crate::lexer;
use crate::parser::Parser;
use crate::value::{repr, ErrorInfo, Fault, VarList};

/// Marks a line meant for the editor rather than the person (ASCII "record separator").
pub const MARK: char = '\u{1e}';

pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// 1-based column of a character position.
fn column(src: &str, pos: usize) -> usize {
    let before: Vec<char> = src.chars().take(pos).collect();
    match before.iter().rposition(|&c| c == '\n') {
        Some(nl) => pos - nl,
        None => pos + 1,
    }
}

/// The first syntax problem: (message, line, character position if known).
fn first_problem(src: &str) -> Option<(String, usize, Option<usize>)> {
    match lexer::lex(src) {
        Err(e) => Some((e.msg, e.line, Some(e.pos))),
        Ok(toks) => match Parser::new(toks).program() {
            Err(e) => Some((e.msg, e.line, e.pos)),
            Ok(_) => None,
        },
    }
}

/// How a line's brackets balance, ignoring text in quotes and comments:
/// (net `{`, starts with `}`, net `(` and `[`).
fn bracket_shape(line: &str) -> (i32, bool, i32) {
    let mut braces = 0;
    let mut others = 0;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let starts_closed = line.trim_start().starts_with('}');
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if q == '\'' && c == '\\' {
                    chars.next();
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                '>' if chars.peek() == Some(&'>') => break,
                '{' => braces += 1,
                '}' => braces -= 1,
                '(' | '[' => others += 1,
                ')' | ']' => others -= 1,
                _ => {}
            },
        }
    }
    (braces, starts_closed, others)
}

/// A stand-in for a broken line that keeps the blocks around it in shape, so
/// checking can carry on past it. None when there's no safe stand-in.
fn stand_in(line: &str) -> Option<String> {
    let (braces, starts_closed, others) = bracket_shape(line);
    if others != 0 {
        return None; // a list or call that runs over several lines
    }
    let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
    let opens_again = starts_closed && line.trim_end().ends_with('{');
    let fill = if opens_again {
        // "} ELIF ... {" or "} FAIL e {": close the block above and open the next.
        if line.contains("FAIL") { "} FAIL e_ {".to_string() } else { "} ELIF TRUE {".to_string() }
    } else if braces > 0 {
        "IF TRUE {".repeat(braces as usize)
    } else if braces < 0 {
        "}".repeat((-braces) as usize)
    } else {
        String::new()
    };
    Some(indent + &fill)
}

/// `{"ok":true}`, or the syntax problems: the first one's message, line and
/// column, plus every problem found (up to 10) in "errors".
pub fn check(src: &str) -> String {
    let mut lines: Vec<String> = src.split('\n').map(str::to_string).collect();
    let mut found: Vec<(String, usize, Option<usize>)> = Vec::new();
    while found.len() < 10 {
        let text = lines.join("\n");
        let Some((msg, line, pos)) = first_problem(&text) else { break };
        let col = pos.map(|p| column(&text, p));
        // The same line failing again means the stand-in didn't help: stop there.
        if found.iter().any(|f| f.1 == line) {
            break;
        }
        let done = line == 0 || line > lines.len() || msg.contains("never closed");
        found.push((msg, line, col));
        if done {
            break;
        }
        match stand_in(&lines[line - 1]) {
            Some(s) => lines[line - 1] = s,
            None => break,
        }
    }
    if found.is_empty() {
        return "{\"ok\":true}".into();
    }
    let col_text = |c: &Option<usize>| c.map(|c| c.to_string()).unwrap_or_else(|| "null".into());
    let list: Vec<String> = found
        .iter()
        .map(|(m, l, c)| format!("{{\"message\":{},\"line\":{},\"col\":{}}}", json_str(m), l, col_text(c)))
        .collect();
    let (m, l, c) = &found[0];
    format!(
        "{{\"ok\":false,\"message\":{},\"line\":{},\"col\":{},\"errors\":[{}]}}",
        json_str(m),
        l,
        col_text(c),
        list.join(",")
    )
}

pub fn run_check(arg: &str) -> i32 {
    let src = if arg == "-" {
        let mut s = String::new();
        let _ = std::io::stdin().read_to_string(&mut s);
        s
    } else {
        match std::fs::read_to_string(arg) {
            Ok(s) => s,
            Err(e) => {
                println!("{{\"ok\":false,\"message\":{},\"line\":1,\"col\":null}}", json_str(&format!("couldn't open {}: {}", arg, e)));
                return 2;
            }
        }
    };
    let out = check(&src);
    println!("{}", out);
    if out.starts_with("{\"ok\":true") {
        0
    } else {
        1
    }
}

fn vars_json(vars: &VarList) -> String {
    let items: Vec<String> = vars
        .iter()
        .map(|(k, v)| format!("{{\"name\":{},\"value\":{},\"type\":\"{}\"}}", json_str(k), json_str(&repr(v)), v.type_name()))
        .collect();
    format!("[{}]", items.join(","))
}

/// A crash report as one line of JSON.
pub fn report_json(e: &ErrorInfo) -> String {
    let frames: Vec<String> = e
        .frames
        .iter()
        .zip(e.frame_paths.iter().map(String::as_str).chain(std::iter::repeat("")))
        .map(|((name, line, vars), path)| {
            format!("{{\"name\":{},\"line\":{},\"path\":{},\"vars\":{}}}", json_str(name), line, json_str(path), vars_json(vars))
        })
        .collect();
    format!(
        "{{\"message\":{},\"file\":{},\"path\":{},\"line\":{},\"traced\":{},\"frames\":[{}],\"outside\":{}}}",
        json_str(&e.msg),
        json_str(&e.file),
        json_str(&e.path),
        e.line,
        e.traced,
        frames.join(","),
        vars_json(&e.outside)
    )
}

/// Quick-try: each snippet ends with a line holding only the MARK character.
/// After running it, jlb prints MARK followed by one line of JSON:
/// `{"ok":true,"value":"5","type":"NUMBER"}` for an expression, `{"ok":true}` for
/// statements, or `{"ok":false,"error":{...crash report...}}`.
/// A line holding only MARK + "RESET" clears every variable.
/// INPUT prints MARK + `{"input":"the prompt"}` and reads its answer as the next line.
pub fn run_repl() -> i32 {
    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut it = Interp::new();
    it.ide = true;
    it.repl = true;
    it.start_session(dir.clone());
    let stdin = std::io::stdin();
    let mut chunk = String::new();
    let mut line = String::new();
    loop {
        line.clear();
        match stdin.lock().read_line(&mut line) {
            Ok(0) | Err(_) => return 0,
            Ok(_) => {}
        }
        let t = line.trim_end_matches(['\n', '\r']);
        if t == format!("{}RESET", MARK) {
            it = Interp::new();
            it.ide = true;
            it.repl = true;
            it.start_session(dir.clone());
            chunk.clear();
            println!("{}{{\"ok\":true}}", MARK);
            let _ = std::io::stdout().flush();
            continue;
        }
        if t != MARK.to_string() {
            chunk.push_str(t);
            chunk.push('\n');
            continue;
        }
        let reply = match it.run_snippet(&chunk) {
            Ok(Some(v)) => format!("{{\"ok\":true,\"value\":{},\"type\":\"{}\"}}", json_str(&repr(&v)), v.type_name()),
            Ok(None) => "{\"ok\":true}".into(),
            Err(Fault::End) | Err(Fault::Cancelled) => "{\"ok\":true,\"ended\":true}".into(),
            Err(Fault::Error(e)) | Err(Fault::TaskError(e)) => format!("{{\"ok\":false,\"error\":{}}}", report_json(&e)),
        };
        let _ = it.out.flush();
        chunk.clear();
        println!("{}{}", MARK, reply);
        let _ = std::io::stdout().flush();
    }
}


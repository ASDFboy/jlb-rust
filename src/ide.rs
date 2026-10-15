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

/// `{"ok":true}`, or the first syntax problem with its line and column.
pub fn check(src: &str) -> String {
    let problem = match lexer::lex(src) {
        Err(e) => Some((e.msg, e.line, Some(e.pos))),
        Ok(toks) => match Parser::new(toks).program() {
            Err(e) => Some((e.msg, e.line, e.pos)),
            Ok(_) => None,
        },
    };
    match problem {
        None => "{\"ok\":true}".into(),
        Some((msg, line, pos)) => {
            let col = pos.map(|p| column(src, p).to_string()).unwrap_or_else(|| "null".into());
            format!("{{\"ok\":false,\"message\":{},\"line\":{},\"col\":{}}}", json_str(&msg), line, col)
        }
    }
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
pub fn run_repl() -> i32 {
    let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut it = Interp::new();
    it.ide = true;
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
            Err(Fault::End) => "{\"ok\":true,\"ended\":true}".into(),
            Err(Fault::Error(e)) => format!("{{\"ok\":false,\"error\":{}}}", report_json(&e)),
        };
        let _ = it.out.flush();
        chunk.clear();
        println!("{}{}", MARK, reply);
        let _ = std::io::stdout().flush();
    }
}


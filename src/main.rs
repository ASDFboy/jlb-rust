// jlb: interpreter for the .jlb language.
// Usage: jlb [--pause | --ide] <file.jlb>, jlb --check <file.jlb | ->, jlb --repl

mod ast;
mod builtins;
mod ide;
mod interp;
mod lexer;
mod number;
mod ops;
mod parser;
mod value;

use std::io::{BufRead, Write};
use std::path::PathBuf;

/// True when jlb has a console window all to itself, which on Windows means it
/// was started by double-clicking a .jlb file rather than from a terminal.
#[cfg(windows)]
fn own_console_window() -> bool {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleProcessList(list: *mut u32, count: u32) -> u32;
    }
    let mut ids = [0u32; 4];
    // SAFETY: the buffer is valid for 4 entries, which is the count passed.
    let n = unsafe { GetConsoleProcessList(ids.as_mut_ptr(), ids.len() as u32) };
    n == 1
}

#[cfg(not(windows))]
fn own_console_window() -> bool {
    false
}

fn wait_for_enter() {
    eprint!("\nPress Enter to close...");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut pause = false;
    let mut for_ide = false;
    match args.first().map(String::as_str) {
        Some("--pause") => {
            pause = true;
            args.remove(0);
        }
        Some("--ide") => {
            for_ide = true;
            args.remove(0);
        }
        Some("--check") => {
            let target = args.get(1).cloned().unwrap_or_else(|| "-".into());
            std::process::exit(ide::run_check(&target));
        }
        Some("--repl") => {
            let child = std::thread::Builder::new()
                .stack_size(256 * 1024 * 1024)
                .spawn(ide::run_repl)
                .expect("couldn't start the interpreter thread");
            std::process::exit(child.join().unwrap_or(101));
        }
        _ => {}
    }
    let Some(first) = args.first() else {
        eprintln!("Usage: jlb <file.jlb>");
        std::process::exit(2);
    };
    if first == "--help" || first == "-h" {
        eprintln!("Usage: jlb <file.jlb>");
        eprintln!("       jlb --pause <file.jlb>   wait for Enter when the program ends");
        eprintln!("       jlb --check <file.jlb>   check the syntax only (- reads the code from input)");
        eprintln!("       jlb --ide <file.jlb>     run for an editor (live output, JSON crash report)");
        eprintln!("       jlb --repl               start a Quick-try session for an editor");
        return;
    }
    if first == "--version" {
        println!("jlb {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let mut path = PathBuf::from(first);
    let double_clicked = !for_ide && own_console_window();
    pause |= double_clicked;
    if !path.is_file() {
        eprintln!("ERROR: couldn't find the file {}", path.display());
        if pause {
            wait_for_enter();
        }
        std::process::exit(2);
    }
    // A double-clicked program runs inside its own folder, so READ and WRITE find files next to it.
    if double_clicked {
        if let Ok(abs) = path.canonicalize() {
            if let Some(dir) = abs.parent() {
                let _ = std::env::set_current_dir(dir);
            }
            path = abs;
        }
    }
    // A large stack lets .jlb programs recurse deeply; it is only reserved, not used up front.
    let child = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(move || {
            let mut it = interp::Interp::new();
            it.ide = for_ide;
            it.run_main(&path)
        })
        .expect("couldn't start the interpreter thread");
    let code = child.join().unwrap_or(101);
    if pause {
        wait_for_enter();
    }
    std::process::exit(code);
}

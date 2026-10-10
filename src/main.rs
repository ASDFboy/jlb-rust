// jlb: interpreter for the .jlb language.
// Usage: jlb [--pause] <file.jlb>

mod ast;
mod builtins;
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
    if args.first().is_some_and(|a| a == "--pause") {
        pause = true;
        args.remove(0);
    }
    let Some(first) = args.first() else {
        eprintln!("Usage: jlb <file.jlb>");
        std::process::exit(2);
    };
    if first == "--help" || first == "-h" {
        eprintln!("Usage: jlb <file.jlb>");
        return;
    }
    if first == "--version" {
        println!("jlb {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let mut path = PathBuf::from(first);
    let double_clicked = own_console_window();
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
        .spawn(move || interp::Interp::new().run_main(&path))
        .expect("couldn't start the interpreter thread");
    let code = child.join().unwrap_or(101);
    if pause {
        wait_for_enter();
    }
    std::process::exit(code);
}

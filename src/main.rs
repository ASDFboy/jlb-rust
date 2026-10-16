// jlb: interpreter for the .jlb language.
// Usage: jlb [--pause | --ide] <file.jlb>, jlb --check <file.jlb | ->, jlb --repl, jlb --debug <file.jlb>
// A program whose first line is `>> @interpreter c` is handed to jlb-c (next to jlb).

mod ast;
mod builtins;
mod debug;
mod ide;
mod interp;
mod lexer;
mod number;
mod ops;
mod parser;
mod tasks;
mod value;

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// The interpreter a program's first line asks for, like `>> @interpreter c`
/// (lowercased), or None when it doesn't ask.
pub fn asked_interpreter(first_line: &str) -> Option<String> {
    let l = first_line.trim_start_matches('\u{feff}').trim();
    let rest = l.strip_prefix(">>")?;
    if rest.starts_with('>') {
        return None; // a >>> comment
    }
    let rest = rest.trim_start().strip_prefix("@interpreter")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim().to_lowercase())
}

fn first_line(path: &Path) -> String {
    let mut line = String::new();
    if let Ok(f) = std::fs::File::open(path) {
        let _ = std::io::BufReader::new(f).read_line(&mut line);
    }
    line
}

/// Runs the program with jlb-c instead, when its first line asks for it. Gives back
/// the exit code to finish with, or None to run it here.
fn hand_off(path: &Path, for_ide: bool) -> Option<i32> {
    let asked = asked_interpreter(&first_line(path))?;
    match asked.as_str() {
        "rust" => return None,
        "c" => {}
        other => {
            eprintln!("ERROR in {}, line 1: there's no interpreter called {}. Use >> @interpreter rust or >> @interpreter c", path.display(), other);
            return Some(1);
        }
    }
    // Already handed over once: run here rather than going back and forth.
    if std::env::var_os("JLB_HANDED_OFF").is_some() {
        return None;
    }
    let name = if cfg!(windows) { "jlb-c.exe" } else { "jlb-c" };
    let exe = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join(name)));
    let Some(exe) = exe.filter(|e| e.is_file()) else {
        eprintln!("ERROR in {}, line 1: this program asks for the C interpreter (>> @interpreter c), but {} isn't in the same folder as jlb", path.display(), name);
        return Some(2);
    };
    let mut cmd = std::process::Command::new(exe);
    if for_ide {
        cmd.arg("--ide");
    }
    cmd.arg(path).env("JLB_HANDED_OFF", "1");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = cmd.exec(); // only comes back if it couldn't start
        eprintln!("ERROR: couldn't start the C interpreter: {}", e);
        Some(2)
    }
    #[cfg(not(unix))]
    {
        match cmd.status() {
            Ok(st) => Some(st.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("ERROR: couldn't start the C interpreter: {}", e);
                Some(2)
            }
        }
    }
}

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
        Some("--debug") => {
            let Some(file) = args.get(1).cloned() else {
                eprintln!("Usage: jlb --debug <file.jlb>");
                std::process::exit(2);
            };
            let child = std::thread::Builder::new()
                .stack_size(256 * 1024 * 1024)
                .spawn(move || debug::run(&file))
                .expect("couldn't start the interpreter thread");
            std::process::exit(child.join().unwrap_or(101));
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
        eprintln!("       jlb --debug <file.jlb>   run for an editor's debugger (see src/debug.rs)");
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
    // `>> @interpreter c` on the first line hands the program to jlb-c.
    if let Some(code) = hand_off(&path, for_ide) {
        if pause {
            wait_for_enter();
        }
        std::process::exit(code);
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

#[cfg(test)]
mod tests {
    use super::asked_interpreter;

    #[test]
    fn reads_the_interpreter_line() {
        assert_eq!(asked_interpreter(">> @interpreter c\n").as_deref(), Some("c"));
        assert_eq!(asked_interpreter("\u{feff}  >>   @interpreter   Rust  \r\n").as_deref(), Some("rust"));
        assert_eq!(asked_interpreter(">>> @interpreter c <<<"), None);
        assert_eq!(asked_interpreter(">> @interpreterc"), None);
        assert_eq!(asked_interpreter("PRINT(1)"), None);
        assert_eq!(asked_interpreter(">> just a note"), None);
    }
}

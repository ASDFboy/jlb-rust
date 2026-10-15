// The editor modes: --check, --ide and --repl. The C interpreter (jlb-c) must match.

use std::io::Write;
use std::process::{Command, Stdio};

fn jlb(args: &[&str], input: &str) -> (String, String, i32) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_jlb"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n"),
        out.status.code().unwrap_or(-1),
    )
}

#[test]
fn check_mode() {
    assert_eq!(jlb(&["--check", "-"], "PRINT(1)\n").0, "{\"ok\":true}\n");
    let (out, _, code) = jlb(&["--check", "-"], "x = 5\ny = x != 3\n");
    assert_eq!(code, 1);
    let e = "{\"message\":\"!= isn't an operator in .jlb; use !== (or !=== for the strict version)\",\"line\":2,\"col\":7}";
    assert_eq!(
        out,
        format!("{{\"ok\":false,\"message\":\"!= isn't an operator in .jlb; use !== (or !=== for the strict version)\",\"line\":2,\"col\":7,\"errors\":[{e}]}}\n")
    );
    // Every problem, not just the first.
    let (out, _, _) = jlb(&["--check", "-"], "x = 5\nIF x != 3 {\n    Print(x)\n}\nmyScore = 1\n");
    assert!(out.contains("\"line\":2,\"col\":6}"), "{}", out);
    assert!(out.contains("did you mean PRINT?"), "{}", out);
    assert!(out.contains("my_score"), "{}", out);
    assert_eq!(out.matches("\"message\"").count(), 4, "{}", out); // the first one twice, then the other two
    let (out, _, _) = jlb(&["--check", "-"], "x = 1\nIF x > 0 {\n  PRINT(x)\n");
    assert!(out.contains("\"line\":2,\"col\":10"), "{}", out);
    let (out, _, _) = jlb(&["--check", "-"], "POINT a\nPOINT a\n");
    assert!(out.contains("\"line\":2,\"col\":null"), "{}", out);
}

#[test]
fn ide_mode_crash_report() {
    let dir = std::env::temp_dir().join(format!("jlb-ide-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("crash.jlb");
    std::fs::write(&file, "FUNC f(n) {\n  LOCAL k = n * 2\n  RETURN k / 0\n}\nname = \"Alex\"\nf(3)\n").unwrap();
    let (out, err, code) = jlb(&["--ide", file.to_str().unwrap()], "");
    assert_eq!(code, 1);
    assert_eq!(out, "");
    let json = err.lines().find(|l| l.starts_with('\u{1e}')).expect("JSON report line");
    let p = file.to_str().unwrap().replace('\\', "\\\\");
    assert_eq!(
        json,
        format!(
            "\u{1e}{{\"message\":\"can't divide by zero\",\"file\":\"crash.jlb\",\"path\":\"{p}\",\"line\":3,\"traced\":true,\"frames\":[{{\"name\":\"f\",\"line\":3,\"path\":\"{p}\",\"vars\":[{{\"name\":\"n\",\"value\":\"3\",\"type\":\"NUMBER\"}},{{\"name\":\"k\",\"value\":\"6\",\"type\":\"NUMBER\"}}]}}],\"outside\":[{{\"name\":\"name\",\"value\":\"\\\"Alex\\\"\",\"type\":\"TEXT\"}}]}}"
        )
    );
    assert!(err.starts_with("ERROR in crash.jlb, line 3: can't divide by zero"));
}

#[test]
fn repl_mode() {
    let m = "\u{1e}";
    let input = format!("x = 5\n{m}\nx * 2\n{m}\n\"25\" * 2\n{m}\nPRINT(x)\n{m}\nx / 0\n{m}\n{m}RESET\nx\n{m}\n");
    let (out, _, code) = jlb(&["--repl"], &input);
    assert_eq!(code, 0);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], format!("{m}{{\"ok\":true}}"));
    assert_eq!(lines[1], format!("{m}{{\"ok\":true,\"value\":\"10\",\"type\":\"NUMBER\"}}"));
    assert_eq!(lines[2], format!("{m}{{\"ok\":true,\"value\":\"\\\"2525\\\"\",\"type\":\"TEXT\"}}"));
    assert_eq!(lines[3], "5");
    assert_eq!(lines[4], format!("{m}{{\"ok\":true}}"));
    assert!(lines[5].contains("can't divide by zero"));
    assert_eq!(lines[6], format!("{m}{{\"ok\":true}}"));
    assert!(lines[7].contains("x isn't defined"));
}

#[test]
fn repl_input_asks_the_editor() {
    let m = "\u{1e}";
    let input = format!("n = INPUT(\"Name? \")\n{m}\nAlex\nn * -1\n{m}\n");
    let (out, _, _) = jlb(&["--repl"], &input);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], format!("{m}{{\"input\":\"Name? \"}}"));
    assert_eq!(lines[1], format!("{m}{{\"ok\":true}}"));
    assert_eq!(lines[2], format!("{m}{{\"ok\":true,\"value\":\"\\\"xelA\\\"\",\"type\":\"TEXT\"}}"));
}

// Runs every program in tests/cases and compares its output (stdout, then stderr)
// with the matching .out file. A matching .in file is fed in as typed input.
// The same cases are used by the C interpreter (jlb-c).

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[test]
fn shared_cases() {
    let cases = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_jlb"));
    let mut files: Vec<PathBuf> = std::fs::read_dir(&cases)
        .expect("tests/cases folder")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "jlb"))
        .collect();
    files.sort();
    let mut failures = Vec::new();
    for path in &files {
        let tmp = std::env::temp_dir().join(format!("jlb-test-{}-{}", std::process::id(), path.file_stem().unwrap().to_string_lossy()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let expected = std::fs::read_to_string(path.with_extension("out")).unwrap_or_default();
        let input = std::fs::read_to_string(path.with_extension("in")).unwrap_or_default();
        let mut child = Command::new(&exe)
            .arg(path)
            .current_dir(&tmp)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let out = child.wait_with_output().unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
        let actual = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        let norm = |s: &str| s.replace("\r\n", "\n").trim_end().to_string();
        if norm(&actual) != norm(&expected) {
            failures.push(format!(
                "--- {} ---\nexpected:\n{}\n\nactual:\n{}\n",
                path.file_name().unwrap().to_string_lossy(),
                norm(&expected),
                norm(&actual)
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{} of {} cases failed:\n\n{}", failures.len(), files.len(), failures.join("\n"));
    }
    println!("all {} cases passed", files.len());
}

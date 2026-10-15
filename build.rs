// On Windows, gives jlb.exe the JLB logo (windows/jlb.ico) as its icon, which is
// also the icon .jlb files get once windows\install.bat has set them up.
// Needs a resource compiler: windres (MinGW) or rc.exe (Visual Studio). Without
// one, jlb.exe is built with the default icon and everything else still works.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=windows/jlb.ico");
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("windows").join("jlb.ico");
    let rc = out.join("jlb.rc");
    // Resource scripts want forward slashes (or doubled backslashes).
    let ico_path = ico.to_string_lossy().replace('\\', "/");
    if std::fs::write(&rc, format!("1 ICON \"{}\"\n", ico_path)).is_err() {
        return;
    }

    if target.contains("msvc") {
        let res = out.join("jlb.res");
        let ok = Command::new("rc.exe").arg("/nologo").arg("/fo").arg(&res).arg(&rc).status().map(|s| s.success()).unwrap_or(false);
        if ok {
            println!("cargo:rustc-link-arg-bins={}", res.display());
        } else {
            println!("cargo:warning=rc.exe wasn't found, so jlb.exe gets the default icon");
        }
        return;
    }

    let obj = out.join("jlb-icon.o");
    let tools = ["x86_64-w64-mingw32-windres", "windres"];
    let ok = tools.iter().any(|tool| {
        Command::new(tool)
            .args(["-O", "coff", "-i"])
            .arg(&rc)
            .arg("-o")
            .arg(&obj)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });
    if ok {
        println!("cargo:rustc-link-arg-bins={}", obj.display());
    } else {
        println!("cargo:warning=windres wasn't found, so jlb.exe gets the default icon");
    }
}

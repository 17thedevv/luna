use std::fs;
use std::process::Command;

fn run_luna_source(name: &str, source: &str) -> i32 {
    let temp_dir = std::env::temp_dir().join(format!("luna_macro_decouple_{}_{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("main.ln");
    fs::write(&file_path, source).unwrap();

    let exe_path = temp_dir.join(if cfg!(windows) { "main.exe" } else { "main" });

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_luna"));
    cmd.arg("build")
        .arg(&file_path)
        .arg("-o")
        .arg(&exe_path)
        .arg("--quiet");

    if let Ok(runtime_lib) = std::env::var("LUNA_RUNTIME_LIB") {
        cmd.env("LUNA_RUNTIME_LIB", runtime_lib);
    }

    let output = cmd.output().expect("Failed to execute luna build");
    assert!(
        output.status.success(),
        "Compilation failed for {}:\nstdout:\n{}\nstderr:\n{}",
        name,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let run_output = Command::new(&exe_path)
        .output()
        .expect("Failed to execute compiled binary");

    let _ = fs::remove_dir_all(&temp_dir);
    run_output.status.code().unwrap_or(-1)
}

#[test]
fn test_user_defined_macros_with_stdlib_names_not_intercepted() {
    let source = r#"
macro print {
    (@msg: expr) => {
        10
    }
}

macro println {
    (@msg: expr) => {
        20
    }
}

macro out {
    (@msg: expr) => {
        30
    }
}

macro outln {
    (@msg: expr) => {
        40
    }
}

macro err {
    (@msg: expr) => {
        50
    }
}

macro errln {
    (@msg: expr) => {
        60
    }
}

macro eprint {
    (@msg: expr) => {
        70
    }
}

macro eprintln {
    (@msg: expr) => {
        80
    }
}

fn main() -> i32 {
    // Strings contain format braces {} which previously triggered compiler format interception
    dec a = print!("val: {} {unused}");
    dec b = println!("{0} {}");
    dec c = out!("string with {expr}");
    dec d = outln!("hello {name}");
    dec e = err!("{x} + {y}");
    dec f = errln!("{}");
    dec g = eprint!("{");
    dec h = eprintln!("}");

    if a != 10 { return 1; }
    if b != 20 { return 2; }
    if c != 30 { return 3; }
    if d != 40 { return 4; }
    if e != 50 { return 5; }
    if f != 60 { return 6; }
    if g != 70 { return 7; }
    if h != 80 { return 8; }

    return 0;
}
"#;

    let exit_code = run_luna_source("user_defined_macros", source);
    assert_eq!(exit_code, 0, "All 8 user-defined macros must expand cleanly without compiler interception");
}

#[test]
fn test_namespaced_macro_shadowing() {
    let source = r#"
module custom {
    export macro println {
        (@val: expr) => {
            (@val) * 2
        }
    }
}

fn main() -> i32 {
    dec res = custom::println!(21);
    if res != 42 { return 1; }
    return 0;
}
"#;

    let exit_code = run_luna_source("namespaced_macro", source);
    assert_eq!(exit_code, 0, "Namespaced macro shadowing must evaluate user rule");
}

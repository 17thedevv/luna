use std::fs;
use std::process::Command;

fn assert_fixture_runs(name: &str, code: &str) {
    let work = std::env::temp_dir().join(format!(
        "luna_compound_assign_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&work).unwrap();
    let source = work.join("main.ln");
    let executable = work.join(if cfg!(windows) { "main.exe" } else { "main" });
    fs::write(&source, code).unwrap();

    let options = luna_driver::CompilerOptions {
        output_path: Some(executable.to_string_lossy().into_owned()),
        quiet: true,
        ..Default::default()
    };
    luna_driver::compile(&source.to_string_lossy(), code.to_string(), &options)
        .unwrap_or_else(|diagnostics| panic!("{name} must compile: {diagnostics:#?}"));

    let output = Command::new(executable).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{name} exited with non-zero status: {output:?}");
    let _ = fs::remove_dir_all(work);
}

#[test]
fn test_compound_assignment_all_primitive_ops() {
    let code = r#"
fn main() -> i32 {
    dec rw a: i32 = 10;
    a += 5;
    if a != 15 { return 1; }
    a -= 3;
    if a != 12 { return 2; }
    a *= 2;
    if a != 24 { return 3; }
    a /= 4;
    if a != 6 { return 4; }
    a %= 4;
    if a != 2 { return 5; }

    dec rw b: u32 = 0b1010;
    b &= 0b1100;
    if b != 0b1000 { return 6; }
    b |= 0b0011;
    if b != 0b1011 { return 7; }
    b ^= 0b0110;
    if b != 0b1101 { return 8; }
    b <<= 2;
    if b != 0b110100 { return 9; }
    b >>= 3;
    if b != 0b110 { return 10; }

    return 0;
}
"#;
    assert_fixture_runs("all_primitive_ops", code);
}

#[test]
fn test_compound_assignment_struct_field() {
    let code = r#"
struct Point {
    x: i32,
    y: i32,
};

fn main() -> i32 {
    dec rw pt = Point { x: 10, y: 20 };
    pt.x += 5;
    pt.y *= 2;
    if pt.x != 15 { return 1; }
    if pt.y != 40 { return 2; }
    return 0;
}
"#;
    assert_fixture_runs("struct_field", code);
}

#[test]
fn test_compound_assignment_array_index() {
    let code = r#"
fn main() -> i32 {
    dec rw arr = [10, 20, 30];
    arr[1] += 5;
    if arr[1] != 25 { return 1; }
    return 0;
}
"#;
    assert_fixture_runs("array_index", code);
}

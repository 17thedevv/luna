use std::{fs, path::PathBuf, process::Command};
use luna_driver::{compile, CompilerOptions};

fn create_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("luna_c_gap_04_chain_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_c_gap_04_deep_provider_chain_bounds_dedup() {
    let temp = create_temp_dir("deep_chain");

    // Provider 0 declares a trait and a constrained function
    fs::write(
        temp.join("p0.ln"),
        r#"
export module probe0 {
    export trait Mark { fn mark(self: &Self) -> i32; }
    export fn constrained<T: Mark>(value: &T) -> i32 { return value.mark(); }
    export fn tag() -> i32 { return 0; }
}
"#,
    ).unwrap();

    // Create chain p1 -> p0, p2 -> p1, ... up to p12
    for i in 1..=12 {
        let prev = i - 1;
        fs::write(
            temp.join(format!("p{}.ln", i)),
            format!(
                r#"import "p{}";
export module probe{} {{ export fn tag() -> i32 {{ return {}; }} }}
"#,
                prev, i, i
            ),
        ).unwrap();
    }

    // Main imports p12 and calls probe12::tag()
    let main_path = temp.join("main.ln");
    fs::write(
        &main_path,
        r#"import "p12";
fn main() -> i32 {
    return probe12::tag() - 12;
}
"#,
    ).unwrap();

    let exe_name = if cfg!(windows) { "main.exe" } else { "main" };
    let exe_path = temp.join(exe_name);
    let options = CompilerOptions {
        output_path: Some(exe_path.to_str().unwrap().to_string()),
        search_paths: vec![temp.to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let source = fs::read_to_string(&main_path).unwrap();
    let result = compile(&main_path.to_string_lossy(), source, &options);
    assert!(result.is_ok(), "Compilation of 12-deep provider chain must succeed: {:?}", result.err());

    let output = Command::new(&exe_path)
        .output()
        .expect("execution of deep chain binary");
    assert_eq!(output.status.code(), Some(0), "Binary should return 0");
}

#[test]
fn test_c_gap_04_diamond_import_bounds_idempotence() {
    let temp = create_temp_dir("diamond_import");

    // Base provider A
    fs::write(
        temp.join("base_a.ln"),
        r#"
export module base_a {
    export trait Worker { fn work(self: &Self) -> i32; }
    export fn run<T: Worker>(item: &T) -> i32 { return item.work(); }
}
"#,
    ).unwrap();

    // Provider B imports A
    fs::write(
        temp.join("branch_b.ln"),
        r#"import "base_a";
export module branch_b {
    export fn call_b() -> i32 { return 10; }
}
"#,
    ).unwrap();

    // Provider C imports A
    fs::write(
        temp.join("branch_c.ln"),
        r#"import "base_a";
export module branch_c {
    export fn call_c() -> i32 { return 20; }
}
"#,
    ).unwrap();

    // Main imports both B and C (diamond)
    let main_path = temp.join("main.ln");
    fs::write(
        &main_path,
        r#"import "base_a";
import "branch_b";
import "branch_c";

struct MyWorker {
    val: i32,
};

impl base_a::Worker for MyWorker {
    fn work(self: &Self) -> i32 { return self.val; }
}

fn main() -> i32 {
    dec w = MyWorker { val: 42 };
    dec r = base_a::run<MyWorker>(&w);
    if r != 42 { return 1; }
    return (branch_b::call_b() + branch_c::call_c()) - 30;
}
"#,
    ).unwrap();

    let exe_name = if cfg!(windows) { "main.exe" } else { "main" };
    let exe_path = temp.join(exe_name);
    let options = CompilerOptions {
        output_path: Some(exe_path.to_str().unwrap().to_string()),
        search_paths: vec![temp.to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let source = fs::read_to_string(&main_path).unwrap();
    let result = compile(&main_path.to_string_lossy(), source, &options);
    assert!(result.is_ok(), "Diamond import must compile: {:?}", result.err());

    let output = Command::new(&exe_path)
        .output()
        .expect("execution of diamond import binary");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_c_gap_04_missing_bound_rejected() {
    let temp = create_temp_dir("missing_bound");

    fs::write(
        temp.join("bound_provider.ln"),
        r#"
export module bound_provider {
    export trait Required { fn req(self: &Self) -> i32; }
    export fn execute<T: Required>(val: &T) -> i32 { return val.req(); }
}
"#,
    ).unwrap();

    let main_path = temp.join("main.ln");
    fs::write(
        &main_path,
        r#"import "bound_provider";

struct NotImplementing {
    x: i32,
};

fn main() -> i32 {
    dec obj = NotImplementing { x: 1 };
    // NotImplementing does not implement Required; must be rejected!
    return bound_provider::execute<NotImplementing>(&obj);
}
"#,
    ).unwrap();

    let options = CompilerOptions {
        search_paths: vec![temp.to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let source = fs::read_to_string(&main_path).unwrap();
    let result = compile(&main_path.to_string_lossy(), source, &options);
    assert!(result.is_err(), "Missing trait bound must be rejected");
}

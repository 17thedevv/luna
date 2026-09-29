use luna_driver::{check, compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

static SOURCE_ONLY_ROOT: OnceLock<PathBuf> = OnceLock::new();

fn create_temp_dir(test_name: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("luna_stdlib_01a_acceptance_tests")
        .join(test_name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("Failed to create test temp dir");
    dir
}

fn source_only_sysroot() -> Sysroot {
    let root = SOURCE_ONLY_ROOT.get_or_init(|| {
        let canonical = Sysroot::discover_for_test().expect("test sysroot must exist");
        let root = create_temp_dir("source_only_sysroot");
        copy_tree(canonical.external_dir(), &root.join("libs").join("external"));
        remove_artifacts(&root.join("libs").join("external"));
        root
    });
    Sysroot::from_root(root.clone()).expect("source-only test sysroot")
}

fn copy_tree(source: &std::path::Path, destination: &std::path::Path) {
    fs::create_dir_all(destination).expect("create source-only sysroot directory");
    for entry in fs::read_dir(source).expect("enumerate source sysroot") {
        let entry = entry.expect("read source sysroot entry");
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() { copy_tree(&from, &to); }
        else { fs::copy(from, to).expect("copy source sysroot file"); }
    }
}

fn remove_artifacts(root: &std::path::Path) {
    for entry in fs::read_dir(root).expect("enumerate source-only sysroot") {
        let path = entry.expect("read source-only sysroot entry").path();
        if path.is_dir() { remove_artifacts(&path); }
        else if matches!(path.extension().and_then(|ext| ext.to_str()), Some("llib" | "obj")) {
            fs::remove_file(path).expect("remove stale sysroot artifact");
        }
    }
}

fn make_opts(test_sysroot: &Sysroot) -> CompilerOptions {
    CompilerOptions {
        search_paths: vec![test_sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    }
}

/// 1. Option::as_ref valid read
#[test]
fn test_opt_as_ref_valid_read() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_as_ref_valid_read");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn read_opt(opt: &std::Option<i32>) -> i32 {
    dec ref_opt = opt.as_ref();
    if ref_opt.is_some() {
        match ref_opt {
            std::Option::Some(r) -> { return *r; },
            std::Option::None -> { return 0; },
        }
    }
    return 0;
}

fn main() -> i32 {
    dec o = std::Option::Some(42);
    return read_opt(&o);
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_ok(), "Option::as_ref valid read MUST succeed: {:?}", res.err());
}

/// 2. Option::as_ref mutation conflict rejected
#[test]
fn test_opt_as_ref_mutation_conflict_rejected() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_as_ref_mutation_conflict");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn main() {
    dec rw opt = std::Option::Some(10);
    dec r = opt.as_ref();
    opt = std::Option::Some(20); // Mutating owner while `r` is live MUST be rejected!
    match r {
        std::Option::Some(val) -> { dec _v = *val; },
        std::Option::None -> (),
    }
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_err(), "Mutating Option while borrowed via as_ref() MUST be rejected!");
    let errs = res.err().unwrap();
    assert!(
        errs.iter().any(|d| d.message.contains("borrowed") || d.message.contains("Cannot write") || d.message.contains("conflict")),
        "Expected borrow conflict error, got: {:?}",
        errs
    );
}

/// 3. Option::as_ref mutation allowed after borrow dies (NLL)
#[test]
fn test_opt_as_ref_mutation_after_drop_allowed() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_as_ref_after_drop");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn main() {
    dec rw opt = std::Option::Some(10);
    dec r = opt.as_ref();
    match r {
        std::Option::Some(val) -> { dec _v1 = *val; },
        std::Option::None -> (),
    }
    // `r` is dead now; mutating `opt` MUST succeed
    opt = std::Option::Some(30);
    dec r2 = opt.as_ref();
    match r2 {
        std::Option::Some(val2) -> { dec _v2 = *val2; },
        std::Option::None -> (),
    }
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_ok(), "Mutating Option after as_ref borrow dies MUST succeed: {:?}", res.err());
}

/// 4. Option::as_mut valid write
#[test]
fn test_opt_as_mut_valid_write() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_as_mut_valid_write");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn modify_opt(opt: &rw std::Option<i32>) {
    dec rw mut_opt = opt.as_mut();
    match mut_opt {
        std::Option::Some(r) -> *r = 99,
        std::Option::None -> (),
    }
}

fn main() -> i32 {
    dec rw o = std::Option::Some(1);
    modify_opt(&rw o);
    match o {
        std::Option::Some(v) -> { return v; },
        std::Option::None -> { return 0; },
    }
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_ok(), "Option::as_mut valid write MUST succeed: {:?}", res.err());
}

/// 5. Option::as_mut exclusive borrow conflict rejected
#[test]
fn test_opt_as_mut_exclusive_borrow_conflict_rejected() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_as_mut_conflict");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn main() {
    dec rw opt = std::Option::Some(10);
    dec m = opt.as_mut();
    dec r = opt.as_ref(); // Cannot take shared borrow while exclusive borrow `m` is active!
    match m {
        std::Option::Some(v) -> { dec _v1 = *v; },
        std::Option::None -> (),
    }
    match r {
        std::Option::Some(v) -> { dec _v2 = *v; },
        std::Option::None -> (),
    }
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_err(), "Taking shared borrow while as_mut is active MUST be rejected!");
    let errs = res.err().unwrap();
    assert!(
        errs.iter().any(|d| d.message.contains("borrowed as &rw") || d.message.contains("Cannot access")),
        "Expected exclusive borrow conflict error, got: {:?}",
        errs
    );
}

/// 6. Result::as_ref and Result::as_mut on Ok and Err branches
#[test]
fn test_res_as_ref_and_as_mut_contract() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("res_as_ref_as_mut");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn check_ok(res: &std::Result<i32, i32>) -> bool {
    dec r = res.as_ref();
    return r.is_ok();
}

fn check_err(res: &std::Result<i32, i32>) -> bool {
    dec r = res.as_ref();
    return r.is_err();
}

fn modify_res(res: &rw std::Result<i32, i32>) {
    dec rw m = res.as_mut();
    match m {
        std::Result::Ok(val) -> *val = 100,
        std::Result::Err(err) -> *err = 500,
    }
}

fn main() {
    dec rw r_ok: std::Result<i32, i32> = std::Result::Ok(1);
    dec rw r_err: std::Result<i32, i32> = std::Result::Err(2);

    dec _c1 = check_ok(&r_ok);
    dec _c2 = check_err(&r_err);

    modify_res(&rw r_ok);
    modify_res(&rw r_err);
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_ok(), "Result as_ref and as_mut contract MUST succeed: {:?}", res.err());
}

/// 7. Returning as_ref() of local Option escapes function scope -> REJECT E3005
#[test]
fn test_opt_res_local_escape_rejected() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_res_local_escape");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn escape_opt() -> std::Option<&i32> {
    dec local_opt = std::Option::Some(42);
    return local_opt.as_ref();
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_err(), "Returning as_ref() of local variable MUST be rejected with E3005!");
    let errs = res.err().unwrap();
    assert!(
        errs.iter().any(|d| d.message.contains("E3005") || d.message.contains("LocalBorrowEscape")),
        "Expected E3005 LocalBorrowEscape error, got: {:?}",
        errs
    );
}

/// 8. Generic nested projection: Option<Option<&T>>
#[test]
fn test_opt_res_nested_generic_projection() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_res_nested_generic");
    let opts = make_opts(&test_sysroot);

    let src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

fn inspect_nested<T>(outer: &std::Option<std::Option<T>>) -> bool {
    dec o = outer.as_ref();
    match o {
        std::Option::Some(inner_ref) -> {
            dec i = inner_ref.as_ref();
            return i.is_some();
        }
        std::Option::None -> { return false; },
    }
}

fn main() {
    dec nested: std::Option<std::Option<i32>> = std::Option::Some(std::Option::Some(99));
    dec _r = inspect_nested<i32>(&nested);
}
"#;
    let path = dir.join("main.ln");
    fs::write(&path, src).unwrap();
    let res = check(path.to_str().unwrap(), src.to_string(), &opts);
    assert!(res.is_ok(), "Nested generic Option projection MUST succeed: {:?}", res.err());
}

/// 9. Source .ln vs .llib parity for Option / Result as_ref and as_mut
#[test]
fn test_opt_res_source_and_llib_parity() {
    let test_sysroot = source_only_sysroot();
    let dir = create_temp_dir("opt_res_llib_parity");
    let lib_src_path = dir.join("my_provider.ln");
    let lib_bin_path = dir.join("my_provider.llib");
    let consumer_path = dir.join("consumer.ln");

    let lib_src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;

module my_provider {
    const DUMMY: i32 = 0;

    export fn extract_opt_val(opt: &std::Option<i32>) -> &i32 life_from(opt) {
        dec r = opt.as_ref();
        match r {
            std::Option::Some(val) -> { return val; },
            std::Option::None -> { return &DUMMY; },
        }
    }
}
"#;
    fs::write(&lib_src_path, lib_src).unwrap();

    let compile_opts = CompilerOptions {
        output_path: Some(lib_bin_path.to_string_lossy().to_string()),
        emit_mlib: true,
        no_link: true,
        quiet: true,
        search_paths: vec![test_sysroot.root().to_string_lossy().to_string()],
        ..Default::default()
    };
    let res_compile = compile(lib_src_path.to_str().unwrap(), lib_src.to_string(), &compile_opts);
    assert!(res_compile.is_ok(), "Compiling my_provider.ln to .llib MUST succeed: {:?}", res_compile.err());

    let consumer_src = r#"
import <core/panic>;
import <result>;
import <mem>;
import <slice>;
import <copy>;
import <clone>;
import <ptr>;
import <iter_adapters>;
import <iter_consumers>;
import "my_provider";

fn main() {
    dec rw opt = std::Option::Some(100);
    dec v = my_provider::extract_opt_val(&opt);
    opt = std::Option::Some(200); // Conflict: mutating `opt` while `v` (borrowed from `opt`) is live!
    dec _v = *v;
}
"#;
    fs::write(&consumer_path, consumer_src).unwrap();

    let consumer_opts = CompilerOptions {
        search_paths: vec![
            dir.to_string_lossy().to_string(),
            test_sysroot.root().to_string_lossy().to_string(),
        ],
        quiet: true,
        ..Default::default()
    };

    let res_consumer = check(consumer_path.to_str().unwrap(), consumer_src.to_string(), &consumer_opts);
    assert!(
        res_consumer.is_err(),
        "Consumer mutating Option while borrow from .llib method is live MUST reject!"
    );
    let errs = res_consumer.err().unwrap();
    assert!(
        errs.iter().any(|d| d.message.contains("borrowed") || d.message.contains("Cannot write")),
        "Expected borrow conflict error in consumer, got: {:?}",
        errs
    );
}

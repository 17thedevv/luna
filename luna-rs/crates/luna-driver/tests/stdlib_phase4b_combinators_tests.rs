use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_comb_{}_{}_{}",
        prefix,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("Failed to create temp dir");
    dir
}

fn copy_dir_all(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create source sysroot directory");
    for entry in fs::read_dir(src).expect("read external provider directory") {
        let entry = entry.expect("read provider entry");
        let target = dst.join(entry.file_name());
        if entry.file_type().expect("provider entry type").is_dir() {
            copy_dir_all(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy provider file");
        }
    }
}

fn remove_artifacts(dir: &Path) {
    for entry in fs::read_dir(dir).expect("read source sysroot directory") {
        let path = entry.expect("read sysroot entry").path();
        if path.is_dir() {
            remove_artifacts(&path);
        } else if matches!(path.extension().and_then(|ext| ext.to_str()), Some("llib" | "obj")) {
            fs::remove_file(path).expect("remove stale provider artifact");
        }
    }
}

fn source_sysroot() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let external_dir = manifest_dir
        .parent().expect("crates directory")
        .parent().expect("workspace root")
        .join("libs").join("external");
    let root = create_temp_dir("source_sysroot");
    copy_dir_all(&external_dir, &root);
    remove_artifacts(&root);
    root
}

fn run_binary(dir: &Path, src: &str, opts: &CompilerOptions) -> Result<(i32, String, String), String> {
    let src_path = dir.join("main.ln");
    let exe_path = dir.join(if cfg!(windows) { "main.exe" } else { "main" });
    fs::write(&src_path, src).map_err(|e| e.to_string())?;

    let mut compile_opts = opts.clone();
    compile_opts.output_path = Some(exe_path.to_str().unwrap().to_string());

    let compile_res = compile(src_path.to_str().unwrap(), src.to_string(), &compile_opts);
    if let Err(err) = compile_res {
        return Err(format!("Compilation failed: {:?}", err));
    }

    let output = Command::new(&exe_path)
        .output()
        .map_err(|e| format!("Execution failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);

    Ok((code, stdout, stderr))
}

#[test]
fn test_option_map_and_and_then() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("opt_map");

    let src = r#"
        fn double_it(x: i32) -> i32 {
            return x * 2;
        }

        fn checked_recip(x: i32) -> std::Option<i32> {
            if x == 0 {
                return std::Option::None;
            }
            return std::Option::Some(100 / x);
        }

        fn main() -> i32 {
            // Test map
            dec some_val1: std::Option<i32> = std::Option::Some(10);
            dec m1 = some_val1.map<i32>(double_it);
            if m1.unwrap_or(0) != 20 { return 1; }

            dec none_val1: std::Option<i32> = std::Option::None;
            dec m2 = none_val1.map<i32>(double_it);
            if m2.is_some() == true { return 2; }

            // Test and_then
            dec some_val2: std::Option<i32> = std::Option::Some(10);
            dec at1 = some_val2.and_then<i32>(checked_recip);
            if at1.unwrap_or(0) != 10 { return 3; }

            dec zero_val: std::Option<i32> = std::Option::Some(0);
            dec at2 = zero_val.and_then<i32>(checked_recip);
            if at2.is_some() == true { return 4; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_option_unwrap_or_else_and_or_else() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("opt_or_else");

    let src = r#"
        fn fallback_val() -> i32 {
            return 42;
        }

        fn fallback_opt() -> std::Option<i32> {
            return std::Option::Some(99);
        }

        fn main() -> i32 {
            // unwrap_or_else
            dec s1: std::Option<i32> = std::Option::Some(7);
            dec n1: std::Option<i32> = std::Option::None;
            if s1.unwrap_or_else(fallback_val) != 7 { return 1; }
            if n1.unwrap_or_else(fallback_val) != 42 { return 2; }

            // or_else
            dec s2: std::Option<i32> = std::Option::Some(7);
            dec n2: std::Option<i32> = std::Option::None;
            dec o1 = s2.or_else(fallback_opt);
            if o1.unwrap_or(0) != 7 { return 3; }

            dec o2 = n2.or_else(fallback_opt);
            if o2.unwrap_or(0) != 99 { return 4; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_option_filter() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("opt_filter");

    let src = r#"
        fn is_even(x: &i32) -> bool {
            return (*x % 2) == 0;
        }

        fn main() -> i32 {
            dec even_val: std::Option<i32> = std::Option::Some(4);
            dec odd_val: std::Option<i32> = std::Option::Some(7);
            dec none_val: std::Option<i32> = std::Option::None;

            dec f1 = even_val.filter(is_even);
            if f1.is_some() == false { return 1; }
            if f1.unwrap_or(0) != 4 { return 2; }

            dec f2 = odd_val.filter(is_even);
            if f2.is_some() == true { return 3; }

            dec f3 = none_val.filter(is_even);
            if f3.is_some() == true { return 4; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_result_map_and_map_err() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("res_map");

    let src = r#"
        import <result>;

        fn add_ten(x: i32) -> i32 {
            return x + 10;
        }

        fn negate_err(e: i32) -> i32 {
            return 0 - e;
        }

        fn main() -> i32 {
            // map
            dec ok_val1: std::Result<i32, i32> = std::Result::Ok(5);
            dec err_val1: std::Result<i32, i32> = std::Result::Err(3);

            dec m_ok = ok_val1.map<i32>(add_ten);
            if m_ok.is_ok() == false { return 1; }
            if m_ok.unwrap() != 15 { return 2; }

            dec m_err = err_val1.map<i32>(add_ten);
            if m_err.is_ok() == true { return 3; }
            if m_err.is_err() == false { return 4; }
            dec opt_err1 = m_err.err();
            if opt_err1.unwrap() != 3 { return 5; }

            // map_err
            dec ok_val2: std::Result<i32, i32> = std::Result::Ok(5);
            dec err_val2: std::Result<i32, i32> = std::Result::Err(3);

            dec me_ok = ok_val2.map_err<i32>(negate_err);
            if me_ok.is_ok() == false { return 6; }
            if me_ok.unwrap() != 5 { return 7; }

            dec me_err = err_val2.map_err<i32>(negate_err);
            if me_err.is_ok() == true { return 8; }
            if me_err.is_err() == false { return 9; }
            dec opt_err2 = me_err.err();
            if opt_err2.unwrap() != (0 - 3) { return 10; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_result_and_then_and_unwrap_or_else() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("res_and_then");

    let src = r#"
        import <result>;

        fn parse_positive(x: i32) -> std::Result<i32, i32> {
            if x > 0 {
                return std::Result::Ok(x * 2);
            }
            return std::Result::Err(0 - 1);
        }

        fn handle_err(e: i32) -> i32 {
            return 100 + e;
        }

        fn main() -> i32 {
            dec ok_val: std::Result<i32, i32> = std::Result::Ok(5);
            dec zero_val: std::Result<i32, i32> = std::Result::Ok(0);
            dec err_val: std::Result<i32, i32> = std::Result::Err(50);

            // and_then Ok -> Ok
            dec at1 = ok_val.and_then<i32>(parse_positive);
            if at1.is_ok() == false { return 1; }
            if at1.unwrap() != 10 { return 2; }

            // and_then Ok -> Err
            dec at2 = zero_val.and_then<i32>(parse_positive);
            if at2.is_ok() == true { return 3; }
            dec opt_err1 = at2.err();
            if opt_err1.unwrap() != (0 - 1) { return 4; }

            // and_then Err -> Err
            dec at3 = err_val.and_then<i32>(parse_positive);
            if at3.is_ok() == true { return 5; }
            dec opt_err2 = at3.err();
            if opt_err2.unwrap() != 50 { return 6; }

            // unwrap_or_else
            dec ok_val2: std::Result<i32, i32> = std::Result::Ok(77);
            dec err_val2: std::Result<i32, i32> = std::Result::Err(5);
            if ok_val2.unwrap_or_else(handle_err) != 77 { return 7; }
            if err_val2.unwrap_or_else(handle_err) != 105 { return 8; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_combinator_laziness_short_circuit() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("comb_laziness");

    let src = r#"
        import <core/panic>;
        import <result>;

        fn must_not_be_called_val() -> i32 {
            __luna_panic();
            return 999;
        }

        fn must_not_be_called_opt() -> std::Option<i32> {
            __luna_panic();
            return std::Option::None;
        }

        fn must_not_be_called_res(e: i32) -> std::Result<i32, i32> {
            __luna_panic();
            return std::Result::Err(e);
        }

        fn fallback_val() -> i32 {
            return 42;
        }

        fn fallback_opt() -> std::Option<i32> {
            return std::Option::Some(84);
        }

        fn fallback_res(e: i32) -> std::Result<i32, i32> {
            return std::Result::Ok(e + 10);
        }

        fn main() -> i32 {
            // 1. std::Option::unwrap_or_else on Some does NOT call callback (would panic)
            dec some_opt: std::Option<i32> = std::Option::Some(100);
            dec v1 = some_opt.unwrap_or_else(must_not_be_called_val);
            if v1 != 100 { return 1; }

            // std::Option::unwrap_or_else on None calls callback exactly once
            dec none_opt: std::Option<i32> = std::Option::None;
            dec v2 = none_opt.unwrap_or_else(fallback_val);
            if v2 != 42 { return 2; }

            // 2. std::Option::or_else on Some does NOT call callback (would panic)
            dec some_opt2: std::Option<i32> = std::Option::Some(200);
            dec o1 = some_opt2.or_else(must_not_be_called_opt);
            if o1.unwrap_or(0) != 200 { return 3; }

            // std::Option::or_else on None calls callback exactly once
            dec none_opt2: std::Option<i32> = std::Option::None;
            dec o2 = none_opt2.or_else(fallback_opt);
            if o2.unwrap_or(0) != 84 { return 4; }

            // 3. std::Result::or_else on Ok does NOT call callback (would panic)
            dec ok_res: std::Result<i32, i32> = std::Result::Ok(300);
            dec r1 = ok_res.or_else<i32>(must_not_be_called_res);
            if r1.unwrap() != 300 { return 5; }

            // std::Result::or_else on Err calls callback exactly once
            dec err_res: std::Result<i32, i32> = std::Result::Err(5);
            dec r2 = err_res.or_else<i32>(fallback_res);
            if r2.unwrap() != 15 { return 6; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_flatten_family() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("comb_flatten");

    let src = r#"
        import <result>;

        fn main() -> i32 {
            // std::Option::option_flatten: std::Option<std::Option<T>> -> std::Option<T>
            dec some_some: std::Option<std::Option<i32>> = std::Option::Some(std::Option::Some(42));
            dec f1 = std::option_flatten<i32>(some_some);
            if f1.is_some() == false { return 1; }
            if f1.unwrap() != 42 { return 2; }

            dec some_none: std::Option<std::Option<i32>> = std::Option::Some(std::Option::None);
            dec f2 = std::option_flatten<i32>(some_none);
            if f2.is_some() == true { return 3; }

            dec none_none: std::Option<std::Option<i32>> = std::Option::None;
            dec f3 = std::option_flatten<i32>(none_none);
            if f3.is_some() == true { return 4; }

            // std::Result::result_flatten: std::Result<std::Result<T, E>, E> -> std::Result<T, E>
            dec ok_ok: std::Result<std::Result<i32, i32>, i32> = std::Result::Ok(std::Result::Ok(99));
            dec rf1 = std::result_flatten<i32, i32>(ok_ok);
            if rf1.is_ok() == false { return 5; }
            if rf1.unwrap() != 99 { return 6; }

            dec ok_err: std::Result<std::Result<i32, i32>, i32> = std::Result::Ok(std::Result::Err(7));
            dec rf2 = std::result_flatten<i32, i32>(ok_err);
            if rf2.is_err() == false { return 7; }
            if rf2.err().unwrap() != 7 { return 8; }

            dec err_outer: std::Result<std::Result<i32, i32>, i32> = std::Result::Err(13);
            dec rf3 = std::result_flatten<i32, i32>(err_outer);
            if rf3.is_err() == false { return 9; }
            if rf3.err().unwrap() != 13 { return 10; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_transpose_family() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("comb_transpose");

    let src = r#"
        import <result>;

        fn main() -> i32 {
            // std::Option::option_transpose: std::Option<std::Result<T, E>> -> std::Result<std::Option<T>, E>
            dec opt_ok: std::Option<std::Result<i32, i32>> = std::Option::Some(std::Result::Ok(55));
            dec t1 = std::option_transpose<i32, i32>(opt_ok);
            if t1.is_ok() == false { return 1; }
            dec inner1 = t1.unwrap();
            if inner1.is_some() == false { return 2; }
            if inner1.unwrap() != 55 { return 3; }

            dec opt_err: std::Option<std::Result<i32, i32>> = std::Option::Some(std::Result::Err(9));
            dec t2 = std::option_transpose<i32, i32>(opt_err);
            if t2.is_err() == false { return 4; }
            if t2.err().unwrap() != 9 { return 5; }

            dec opt_none: std::Option<std::Result<i32, i32>> = std::Option::None;
            dec t3 = std::option_transpose<i32, i32>(opt_none);
            if t3.is_ok() == false { return 6; }
            dec inner3 = t3.unwrap();
            if inner3.is_some() == true { return 7; }

            // std::Result::result_transpose: std::Result<std::Option<T>, E> -> std::Option<std::Result<T, E>>
            dec res_some: std::Result<std::Option<i32>, i32> = std::Result::Ok(std::Option::Some(77));
            dec rt1 = std::result_transpose<i32, i32>(res_some);
            if rt1.is_some() == false { return 8; }
            dec inner_res1 = rt1.unwrap();
            if inner_res1.is_ok() == false { return 9; }
            if inner_res1.unwrap() != 77 { return 10; }

            dec res_none: std::Result<std::Option<i32>, i32> = std::Result::Ok(std::Option::None);
            dec rt2 = std::result_transpose<i32, i32>(res_none);
            if rt2.is_some() == true { return 11; }

            dec res_err: std::Result<std::Option<i32>, i32> = std::Result::Err(15);
            dec rt3 = std::result_transpose<i32, i32>(res_err);
            if rt3.is_some() == false { return 12; }
            dec inner_res3 = rt3.unwrap();
            if inner_res3.is_err() == false { return 13; }
            if inner_res3.err().unwrap() != 15 { return 14; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_result_or_else() {
    let sysroot = source_sysroot();
    let dir = create_temp_dir("res_or_else");

    let src = r#"
        import <result>;
        import <core/panic>;

        fn must_not_run(e: i32) -> std::Result<i32, i32> {
            __luna_panic();
            return std::Result::Err(999);
        }

        fn recover_ok(e: i32) -> std::Result<i32, i32> {
            return std::Result::Ok(e * 10);
        }

        fn transform_err(e: i32) -> std::Result<i32, i32> {
            return std::Result::Err(e + 500);
        }

        fn main() -> i32 {
            // Case 1: Ok(x).or_else(...) -> callback called 0 times (would panic if called), returns Ok(x)
            dec ok_val: std::Result<i32, i32> = std::Result::Ok(42);
            dec r1 = ok_val.or_else<i32>(must_not_run);
            if r1.is_ok() == false { return 1; }
            if r1.unwrap() != 42 { return 2; }

            // Case 2: Err(e).or_else(recover_ok) -> callback called exactly 1 time, recovers to Ok(e * 10)
            dec err_val: std::Result<i32, i32> = std::Result::Err(7);
            dec r2 = err_val.or_else<i32>(recover_ok);
            if r2.is_ok() == false { return 3; }
            if r2.unwrap() != 70 { return 4; }

            // Case 3: Err(e).or_else(transform_err) -> callback called exactly 1 time, yields new Err(e + 500)
            dec err_val2: std::Result<i32, i32> = std::Result::Err(13);
            dec r3 = err_val2.or_else<i32>(transform_err);
            if r3.is_err() == false { return 5; }
            if r3.err().unwrap() != 513 { return 6; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}


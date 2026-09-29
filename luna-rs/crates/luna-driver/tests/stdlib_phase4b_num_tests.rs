use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_num_{}_{}_{}",
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
fn test_checked_div_and_abs_pre_operation_checks() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("div_abs");

    let src = r#"
        import <num>;

        fn main() -> i32 {
            // 1. Division by zero -> None
            dec d1 = std::checked_div_i32(10, 0);
            if d1.is_some() { return 1; }

            dec d2 = std::checked_div_u64(100 as u64, 0 as u64);
            if d2.is_some() { return 2; }

            // 2. Signed MIN / -1 overflow -> None
            dec min_i32: i32 = (0 - 2147483647) - 1;
            dec neg_one: i32 = 0 - 1;
            dec d3 = std::checked_div_i32(min_i32, neg_one);
            if d3.is_some() { return 3; }

            dec min_i64: i64 = ((0 as i64) - (9223372036854775807 as i64)) - (1 as i64);
            dec neg_one_64: i64 = (0 as i64) - (1 as i64);
            dec d4 = std::checked_div_i64(min_i64, neg_one_64);
            if d4.is_some() { return 4; }

            // 3. Normal division -> Some(result)
            dec d5 = std::checked_div_i32(100, 5);
            if d5.unwrap() != 20 { return 5; }

            dec d6 = std::checked_div_u64(1000 as u64, 8 as u64);
            if d6.unwrap() != (125 as u64) { return 6; }

            // 4. checked_abs(MIN) -> None
            dec a1 = std::checked_abs_i32(min_i32);
            if a1.is_some() { return 7; }

            dec a2 = std::checked_abs_i64(min_i64);
            if a2.is_some() { return 8; }

            // 5. Normal checked_abs -> Some(abs)
            dec a3 = std::checked_abs_i32(0 - 42);
            if a3.unwrap() != 42 { return 9; }

            dec a4 = std::checked_abs_i64((0 as i64) - (999 as i64));
            if a4.unwrap() != (999 as i64) { return 10; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_checked_arithmetic_overflow_detection() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("arith_overflow");

    let src = r#"
        import <num>;

        fn main() -> i32 {
            // Unsigned overflow
            dec max_u8: u8 = 255 as u8;
            dec add_u8 = std::checked_add_u8(max_u8, 1 as u8);
            if add_u8.is_some() { return 1; }

            dec sub_u8 = std::checked_sub_u8(0 as u8, 1 as u8);
            if sub_u8.is_some() { return 2; }

            dec mul_u8 = std::checked_mul_u8(max_u8, 2 as u8);
            if mul_u8.is_some() { return 3; }

            // Signed overflow
            dec max_i32: i32 = 2147483647;
            dec min_i32: i32 = (0 - 2147483647) - 1;

            dec add_i32 = std::checked_add_i32(max_i32, 1);
            if add_i32.is_some() { return 4; }

            dec sub_i32 = std::checked_sub_i32(min_i32, 1);
            if sub_i32.is_some() { return 5; }

            dec mul_i32 = std::checked_mul_i32(max_i32, 2);
            if mul_i32.is_some() { return 6; }

            dec neg_one: i32 = 0 - 1;
            dec mul_min = std::checked_mul_i32(min_i32, neg_one);
            if mul_min.is_some() { return 7; }

            // Successful arithmetic
            dec ok_add = std::checked_add_i32(100, 200);
            if ok_add.unwrap() != 300 { return 8; }

            dec ok_sub = std::checked_sub_i32(100, 40);
            if ok_sub.unwrap() != 60 { return 9; }

            dec ok_mul = std::checked_mul_i32(12, 12);
            if ok_mul.unwrap() != 144 { return 10; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_saturating_arithmetic_clamping() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("saturating");

    let src = r#"
        import <num>;

        fn main() -> i32 {
            // Unsigned saturating
            dec max_u8: u8 = 255 as u8;
            if std::saturating_add_u8(max_u8, 10 as u8) != max_u8 { return 1; }
            if std::saturating_sub_u8(5 as u8, 10 as u8) != (0 as u8) { return 2; }
            if std::saturating_mul_u8(200 as u8, 2 as u8) != max_u8 { return 3; }

            // Signed saturating
            dec max_i32: i32 = 2147483647;
            dec min_i32: i32 = (0 - 2147483647) - 1;

            if std::saturating_add_i32(max_i32, 100) != max_i32 { return 4; }
            if std::saturating_sub_i32(min_i32, 100) != min_i32 { return 5; }
            if std::saturating_mul_i32(max_i32, 2) != max_i32 { return 6; }
            if std::saturating_mul_i32(min_i32, 2) != min_i32 { return 7; }

            // Normal non-saturated
            if std::saturating_add_i32(10, 20) != 30 { return 8; }
            if std::saturating_sub_i32(50, 20) != 30 { return 9; }
            if std::saturating_mul_i32(6, 7) != 42 { return 10; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_cmp_min_max_clamp_utilities() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("cmp_utils");

    let src = r#"
        import <cmp>;

        fn main() -> i32 {
            // min and max
            if std::min<i32>(10, 20) != 10 { return 1; }
            if std::min<i32>(30, 20) != 20 { return 2; }
            if std::max<i32>(10, 20) != 20 { return 3; }
            if std::max<i32>(30, 20) != 30 { return 4; }

            // clamp within bounds
            if std::clamp<i32>(15, 10, 20) != 15 { return 5; }
            // clamp below lower bound
            if std::clamp<i32>(5, 10, 20) != 10 { return 6; }
            // clamp above upper bound
            if std::clamp<i32>(25, 10, 20) != 20 { return 7; }

            // unsigned u64 variants
            dec lo: u64 = 100 as u64;
            dec hi: u64 = 200 as u64;
            if std::min<u64>(lo, hi) != lo { return 8; }
            if std::max<u64>(lo, hi) != hi { return 9; }
            if std::clamp<u64>(50 as u64, lo, hi) != lo { return 10; }
            if std::clamp<u64>(250 as u64, lo, hi) != hi { return 11; }
            if std::clamp<u64>(150 as u64, lo, hi) != (150 as u64) { return 12; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

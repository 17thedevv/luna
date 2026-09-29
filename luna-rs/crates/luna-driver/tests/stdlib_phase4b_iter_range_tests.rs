use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_iter_{}_{}_{}",
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
fn test_peekable_peek_does_not_consume() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("peekable");

    let src = r#"
        import <iter_adapters>;

        fn main() -> i32 {
            dec r = std::iter::range(10, 13);
            dec rw p = std::iter::iter_peekable(r);

            // First peek returns Some(&10)
            dec pk1 = p.peek();
            if pk1.is_none() { return 1; }
            if *pk1.unwrap() != 10 { return 2; }

            // Second peek still returns Some(&10) without consuming
            dec pk2 = p.peek();
            if pk2.is_none() { return 3; }
            if *pk2.unwrap() != 10 { return 4; }

            // next consumes 10
            dec n1 = p.next();
            if n1.is_none() { return 5; }
            if n1.unwrap() != 10 { return 6; }

            // next consumes 11
            dec n2 = p.next();
            if n2.is_none() { return 7; }
            if n2.unwrap() != 11 { return 8; }

            // peek now views 12
            dec pk3 = p.peek();
            if pk3.is_none() { return 9; }
            if *pk3.unwrap() != 12 { return 10; }

            // next consumes 12
            dec n3 = p.next();
            if n3.is_none() { return 11; }
            if n3.unwrap() != 12 { return 12; }

            // peek at end returns None
            dec pk_end = p.peek();
            if pk_end.is_some() { return 13; }

            // next at end returns None
            dec n_end = p.next();
            if n_end.is_some() { return 14; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, stdout, stderr) = run_binary(&dir, src, &opts).expect("execution failed");
    assert_eq!(code, 0, "test_peekable failed. stdout: {}, stderr: {}", stdout, stderr);
}

#[test]
fn test_filter_map_and_take_while() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("fm_tw");

    let src = r#"
        import <iter_adapters>;
        import <iter_consumers>;

        fn halve_evens(x: i32) -> std::Option<i32> {
            if x % 2 == 0 {
                return std::Option::Some(x / 2);
            }
            return std::Option::None;
        }

        fn less_than_five(x: &i32) -> bool {
            return *x < 5;
        }

        fn add_pair(acc: i32, v: i32) -> i32 {
            return acc + v;
        }

        fn main() -> i32 {
            // Range 0..10: 0, 1, 2, 3, 4, 5, 6, 7, 8, 9
            // halve_evens yields: 0, 1, 2, 3, 4
            dec r = std::iter::range(0, 10);
            dec fm = std::iter::iter_filter_map(r, halve_evens);

            dec rw tw = std::iter::iter_take_while(fm, less_than_five);
            // tw should yield 0, 1, 2, 3, 4 then terminate
            dec mut_acc: i32 = 0;
            dec sum = std::iter::iter_fold(tw, mut_acc, add_pair);
            // 0 + 1 + 2 + 3 + 4 = 10
            if sum != 10 { return 1; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, stdout, stderr) = run_binary(&dir, src, &opts).expect("execution failed");
    assert_eq!(code, 0, "test_filter_map_and_take_while failed. stdout: {}, stderr: {}", stdout, stderr);
}

#[test]
fn test_range_and_inclusive_boundaries() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("range_bounds");

    let src = r#"
        import <iter_adapters>;
        import <iter_consumers>;

        fn main() -> i32 {
            // --- Part 1: All 10 integer types instantiate Range<T> ---
            // 1. u8
            dec r_u8 = std::iter::range(1 as u8, 4 as u8);
            if std::iter::iter_count(r_u8) != (3 as u64) { return 1; }

            // 2. u16
            dec r_u16 = std::iter::range(1 as u16, 4 as u16);
            if std::iter::iter_count(r_u16) != (3 as u64) { return 2; }

            // 3. u32
            dec r_u32 = std::iter::range(1 as u32, 4 as u32);
            if std::iter::iter_count(r_u32) != (3 as u64) { return 3; }

            // 4. u64
            dec r_u64 = std::iter::range(1 as u64, 4 as u64);
            if std::iter::iter_count(r_u64) != (3 as u64) { return 4; }

            // 5. usize
            dec r_usize = std::iter::range(1 as usize, 4 as usize);
            if std::iter::iter_count(r_usize) != (3 as u64) { return 5; }

            // 6. i8
            dec r_i8 = std::iter::range(1 as i8, 4 as i8);
            if std::iter::iter_count(r_i8) != (3 as u64) { return 6; }

            // 7. i16
            dec r_i16 = std::iter::range(1 as i16, 4 as i16);
            if std::iter::iter_count(r_i16) != (3 as u64) { return 7; }

            // 8. i32
            dec r_i32 = std::iter::range(1, 4);
            if std::iter::iter_count(r_i32) != (3 as u64) { return 8; }

            // 9. i64
            dec r_i64 = std::iter::range(1 as i64, 4 as i64);
            if std::iter::iter_count(r_i64) != (3 as u64) { return 9; }

            // 10. isize
            dec r_isize = std::iter::range(1 as isize, 4 as isize);
            if std::iter::iter_count(r_isize) != (3 as u64) { return 10; }

            // --- Part 2: Boundary behaviors and edge conditions ---
            // Empty range: 5..5 yields count 0
            dec r_empty = std::iter::range(5, 5);
            if std::iter::iter_count(r_empty) != (0 as u64) { return 11; }

            // Single item inclusive: 5..=5 yields count 1
            dec ri_single = std::iter::range_inclusive(5, 5);
            if std::iter::iter_count(ri_single) != (1 as u64) { return 12; }

            // Inverted range: 5..1 yields count 0
            dec r_inv = std::iter::range(5, 1);
            if std::iter::iter_count(r_inv) != (0 as u64) { return 13; }

            // Inverted inclusive: 5..=1 yields count 0
            dec ri_inv = std::iter::range_inclusive(5, 1);
            if std::iter::iter_count(ri_inv) != (0 as u64) { return 14; }

            // --- Part 3: MAX boundary stress on RangeInclusive ---
            // Boundary condition 1: u8 upper limit 254..=255 terminates cleanly without wrapping to 0!
            dec rw ri_u8 = std::iter::range_inclusive(254 as u8, 255 as u8);
            dec v1 = ri_u8.next();
            if v1.is_none() { return 15; }
            if v1.unwrap() != (254 as u8) { return 16; }

            dec v2 = ri_u8.next();
            if v2.is_none() { return 17; }
            if v2.unwrap() != (255 as u8) { return 18; }

            dec v3 = ri_u8.next();
            if v3.is_some() { return 19; } // Must be exhausted, not wrapped!

            // Boundary condition 2: u64 upper limit (MAX-1)..=MAX terminates cleanly without overflow!
            dec u64_max = (0 as u64) - (1 as u64);
            dec u64_max_minus_1 = u64_max - (1 as u64);
            dec rw ri_u64 = std::iter::range_inclusive(u64_max_minus_1, u64_max);

            dec u_v1 = ri_u64.next();
            if u_v1.is_none() { return 20; }
            if u_v1.unwrap() != u64_max_minus_1 { return 21; }

            dec u_v2 = ri_u64.next();
            if u_v2.is_none() { return 22; }
            if u_v2.unwrap() != u64_max { return 23; }

            dec u_v3 = ri_u64.next();
            if u_v3.is_some() { return 24; } // Must be exhausted!

            // Boundary condition 3: i64 upper limit (MAX-1)..=MAX terminates cleanly without overflow!
            dec i64_max = (((0 as u64) - (1 as u64)) / (2 as u64)) as i64;
            dec i64_max_minus_1 = i64_max - (1 as i64);
            dec rw ri_i64 = std::iter::range_inclusive(i64_max_minus_1, i64_max);

            dec i_v1 = ri_i64.next();
            if i_v1.is_none() { return 25; }
            if i_v1.unwrap() != i64_max_minus_1 { return 26; }

            dec i_v2 = ri_i64.next();
            if i_v2.is_none() { return 27; }
            if i_v2.unwrap() != i64_max { return 28; }

            dec i_v3 = ri_i64.next();
            if i_v3.is_some() { return 29; } // Must be exhausted!

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, stdout, stderr) = run_binary(&dir, src, &opts).expect("execution failed");
    assert_eq!(code, 0, "test_range_and_inclusive_boundaries failed. stdout: {}, stderr: {}", stdout, stderr);
}

#[test]
fn test_iter_sum_family() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("iter_sum");

    let src = r#"
        import <iter_adapters>;
        import <iter_consumers>;

        fn main() -> i32 {
            // --- Part 1: All 10 integer types sum execution ---
            // 1. u8: 1..=10 = 55
            dec r_u8 = std::iter::range_inclusive(1 as u8, 10 as u8);
            if std::iter::iter_sum_u8(r_u8) != (55 as u8) { return 1; }

            // 2. u16: 1..=10 = 55
            dec r_u16 = std::iter::range_inclusive(1 as u16, 10 as u16);
            if std::iter::iter_sum_u16(r_u16) != (55 as u16) { return 2; }

            // 3. u32: 1..=10 = 55
            dec r_u32 = std::iter::range_inclusive(1 as u32, 10 as u32);
            if std::iter::iter_sum_u32(r_u32) != (55 as u32) { return 3; }

            // 4. u64: 1..=100 = 5050
            dec r_u64 = std::iter::range_inclusive(1 as u64, 100 as u64);
            if std::iter::iter_sum_u64(r_u64) != (5050 as u64) { return 4; }

            // 5. usize: 1..=10 = 55
            dec r_usize = std::iter::range_inclusive(1 as usize, 10 as usize);
            if std::iter::iter_sum_usize(r_usize) != (55 as usize) { return 5; }

            // 6. i8: -5..=5 = 0
            dec r_i8 = std::iter::range_inclusive((0 as i8) - (5 as i8), 5 as i8);
            if std::iter::iter_sum_i8(r_i8) != (0 as i8) { return 6; }

            // 7. i16: 1..=10 = 55
            dec r_i16 = std::iter::range_inclusive(1 as i16, 10 as i16);
            if std::iter::iter_sum_i16(r_i16) != (55 as i16) { return 7; }

            // 8. i32: 1..11 = 55
            dec r_i32 = std::iter::range(1, 11);
            if std::iter::iter_sum_i32(r_i32) != 55 { return 8; }

            // 9. i64: -10..=10 = 0
            dec r_i64 = std::iter::range_inclusive((0 as i64) - (10 as i64), 10 as i64);
            if std::iter::iter_sum_i64(r_i64) != (0 as i64) { return 9; }

            // 10. isize: 1..=10 = 55
            dec r_isize = std::iter::range_inclusive(1 as isize, 10 as isize);
            if std::iter::iter_sum_isize(r_isize) != (55 as isize) { return 10; }

            // --- Part 2: Empty iterator sums -> 0 ---
            dec empty_u32 = std::iter::range(10 as u32, 10 as u32);
            if std::iter::iter_sum_u32(empty_u32) != (0 as u32) { return 11; }

            dec empty_i64 = std::iter::range(100 as i64, 100 as i64);
            if std::iter::iter_sum_i64(empty_i64) != (0 as i64) { return 12; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, stdout, stderr) = run_binary(&dir, src, &opts).expect("execution failed");
    assert_eq!(code, 0, "test_iter_sum_family failed. stdout: {}, stderr: {}", stdout, stderr);
}

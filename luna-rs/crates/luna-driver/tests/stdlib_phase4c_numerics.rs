use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{check, compile, CompilerOptions};

static FRESH_SYSROOT_ROOT: OnceLock<PathBuf> = OnceLock::new();

fn fresh_phase4c_sysroot() -> Sysroot {
    let root = FRESH_SYSROOT_ROOT.get_or_init(|| {
        let canonical = Sysroot::discover_for_test().expect("canonical sysroot required");
        let base = create_temp_dir("fresh_sysroot");
        let external = base.join("libs").join("external");
        copy_dir_all(canonical.external_dir(), &external).expect("copy isolated source sysroot");
        let isolated = Sysroot::discover(Some(base.to_string_lossy().as_ref()))
            .expect("discover isolated sysroot");
        SysrootBuilder::new(isolated)
            .build_all(true)
            .expect("build fresh Phase 4C test sysroot");
        base
    });
    Sysroot::discover(Some(root.to_string_lossy().as_ref())).expect("reload fresh sysroot")
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_4c_{}_{}_{}",
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

fn run_binary_with_output(
    dir: &Path,
    src: &str,
    opts: &CompilerOptions,
) -> Result<(i32, String, String), String> {
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
fn test_default_protocol() {
    let sysroot = fresh_phase4c_sysroot();
    let dir = create_temp_dir("default_protocol");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let src = r#"
        import <default>;
        import <vec>;
        import <string>;
        import <hashmap>;
        import <hashset>;

        fn main() -> i32 {
            dec b: bool = std::Default::default();
            if b != false { return 1; }

            dec i8_val: i8 = std::Default::default();
            if i8_val != (0 as i8) { return 2; }

            dec i16_val: i16 = std::Default::default();
            if i16_val != (0 as i16) { return 3; }

            dec i32_val: i32 = std::Default::default();
            if i32_val != 0 { return 4; }

            dec i64_val: i64 = std::Default::default();
            if i64_val != (0 as i64) { return 5; }

            dec isize_val: isize = std::Default::default();
            if isize_val != (0 as isize) { return 6; }

            dec u8_val: u8 = std::Default::default();
            if u8_val != (0 as u8) { return 7; }

            dec u16_val: u16 = std::Default::default();
            if u16_val != (0 as u16) { return 8; }

            dec u32_val: u32 = std::Default::default();
            if u32_val != (0 as u32) { return 9; }

            dec u64_val: u64 = std::Default::default();
            if u64_val != (0 as u64) { return 10; }

            dec usize_val: usize = std::Default::default();
            if usize_val != (0 as usize) { return 11; }

            dec f32_val: f32 = std::Default::default();
            if f32_val != (0.0 as f32) { return 12; }

            dec f64_val: f64 = std::Default::default();
            if f64_val != 0.0 { return 13; }

            dec option: std::Option<i32> = std::Default::default();
            if option.is_some() { return 18; }

            // Containers
            dec v: std::Vec<i32> = std::Default::default();
            if v.len() != (0 as u64) { return 14; }

            dec s: std::String = std::Default::default();
            if s.len() != (0 as u64) { return 15; }

            dec m: std::HashMap<i32, i32> = std::Default::default();
            if m.len() != (0 as u64) { return 16; }

            dec set: std::HashSet<i32> = std::Default::default();
            if set.len() != (0 as u64) { return 17; }

            // Creating empty containers must not require Hash or Eq impls.
            dec non_hash_map: std::HashMap<EmptyKey, EmptyValue> = std::Default::default();
            dec non_hash_set: std::HashSet<EmptyKey> = std::Default::default();

            return 0;
        }

        struct EmptyKey {};
        struct EmptyValue {};
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "test_default_protocol failed with code {}", code);
        }
        Err(e) => {
            panic!("test_default_protocol failed: {}", e);
        }
    }
}

#[test]
fn test_standard_conversions() {
    let sysroot = fresh_phase4c_sysroot();
    let dir = create_temp_dir("standard_conversions");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let src = r#"
        import <convert>;
        import <float>;
        import <result>;

        fn test_generic_convert<From: std::Convert<To>, To>(val: From) -> To {
            return val.convert();
        }

        fn main() -> i32 {
            // Blanket reflexive
            dec x: u8 = 42 as u8;
            dec x_ref: u8 = test_generic_convert<u8, u8>(x);
            if x_ref != (42 as u8) { return 1; }

            dec u_size: usize = 100 as usize;
            dec u_ref: usize = test_generic_convert<usize, usize>(u_size);
            if u_ref != (100 as usize) { return 2; }

            // Safe widening
            dec w_u16: u16 = test_generic_convert<u8, u16>(x);
            if w_u16 != (42 as u16) { return 3; }

            dec w_u32: u32 = test_generic_convert<u8, u32>(x);
            if w_u32 != (42 as u32) { return 4; }

            dec w_u64: u64 = test_generic_convert<u8, u64>(x);
            if w_u64 != (42 as u64) { return 5; }

            dec w_i16: i16 = test_generic_convert<u8, i16>(x);
            if w_i16 != (42 as i16) { return 6; }

            dec w_i32: i32 = test_generic_convert<u8, i32>(x);
            if w_i32 != 42 { return 7; }

            dec w_i64: i64 = test_generic_convert<u8, i64>(x);
            if w_i64 != (42 as i64) { return 8; }

            dec f_val: f32 = 1.5 as f32;
            dec f_wide: f64 = test_generic_convert<f32, f64>(f_val);
            if f_wide != 1.5 { return 9; }

            // std::TryConvert success
            dec big_u16: u16 = 200 as u16;
            dec try_ok: std::Result<u8, std::TryConvertError> = std::TryConvert::try_convert(big_u16);
            match try_ok {
                std::Result::Ok(val) -> {
                    if val != (200 as u8) { return 10; }
                },
                std::Result::Err(_) -> { return 11; },
            }

            // std::TryConvert overflow
            dec overflow_u16: u16 = 256 as u16;
            dec try_err: std::Result<u8, std::TryConvertError> = std::TryConvert::try_convert(overflow_u16);
            match try_err {
                std::Result::Ok(_) -> { return 12; },
                std::Result::Err(std::TryConvertError::Overflow) -> {},
                std::Result::Err(_) -> { return 13; },
            }

            dec exact_u16: u16 = 255 as u16;
            dec exact_u16_result: std::Result<u8, std::TryConvertError> = std::TryConvert::try_convert(exact_u16);
            match exact_u16_result {
                std::Result::Ok(v) -> { if v != (255 as u8) { return 14; } },
                std::Result::Err(_) -> { return 15; },
            }
            dec signed_ok: i16 = 127 as i16;
            dec signed_ok_result: std::Result<i8, std::TryConvertError> = std::TryConvert::try_convert(signed_ok);
            match signed_ok_result {
                std::Result::Ok(v) -> { if v != (127 as i8) { return 16; } },
                std::Result::Err(_) -> { return 17; },
            }
            dec signed_high: i16 = 128 as i16;
            dec signed_high_result: std::Result<i8, std::TryConvertError> = std::TryConvert::try_convert(signed_high);
            match signed_high_result {
                std::Result::Err(std::TryConvertError::Overflow) -> {},
                _ -> { return 18; },
            }
            dec signed_low: i16 = -129 as i16;
            dec signed_low_result: std::Result<i8, std::TryConvertError> = std::TryConvert::try_convert(signed_low);
            match signed_low_result {
                std::Result::Err(std::TryConvertError::Underflow) -> {},
                _ -> { return 19; },
            }
            dec unsigned_low: i32 = -1;
            dec unsigned_low_result: std::Result<u32, std::TryConvertError> = std::TryConvert::try_convert(unsigned_low);
            match unsigned_low_result {
                std::Result::Err(std::TryConvertError::Underflow) -> {},
                _ -> { return 20; },
            }
            dec unsigned_zero: i32 = 0;
            dec unsigned_zero_result: std::Result<u32, std::TryConvertError> = std::TryConvert::try_convert(unsigned_zero);
            match unsigned_zero_result {
                std::Result::Ok(v) -> { if v != (0 as u32) { return 33; } },
                std::Result::Err(_) -> { return 34; },
            }
            dec wide_unsigned: u32 = 42 as u32;
            dec wide_unsigned_result: std::Result<i64, std::TryConvertError> = std::TryConvert::try_convert(wide_unsigned);
            match wide_unsigned_result {
                std::Result::Ok(v) -> { if v != (42 as i64) { return 20; } },
                std::Result::Err(_) -> { return 21; },
            }
            dec too_wide: u64 = 3000000000u64;
            dec too_wide_result: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(too_wide);
            match too_wide_result {
                std::Result::Err(std::TryConvertError::Overflow) -> {},
                _ -> { return 22; },
            }

            dec signed_float: f32 = 127.9 as f32;
            dec signed_float_result: std::Result<i8, std::TryConvertError> = std::TryConvert::try_convert(signed_float);
            match signed_float_result {
                std::Result::Ok(v) -> { if v != (127 as i8) { return 23; } },
                std::Result::Err(_) -> { return 24; },
            }
            dec signed_float_low: f32 = -128.5 as f32;
            dec signed_float_low_result: std::Result<i8, std::TryConvertError> = std::TryConvert::try_convert(signed_float_low);
            match signed_float_low_result {
                std::Result::Ok(v) -> { if v != (-128 as i8) { return 25; } },
                std::Result::Err(_) -> { return 26; },
            }
            dec i32_min_float: f32 = -2147483648.0 as f32;
            dec i32_min_result: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(i32_min_float);
            match i32_min_result {
                std::Result::Ok(v) -> { if v != (-2147483648 as i32) { return 35; } },
                std::Result::Err(_) -> { return 36; },
            }
            dec float_low: f32 = -0.5 as f32;
            dec float_low_result: std::Result<u32, std::TryConvertError> = std::TryConvert::try_convert(float_low);
            match float_low_result {
                std::Result::Err(std::TryConvertError::Underflow) -> {},
                _ -> { return 27; },
            }
            dec float_nan: f32 = std::f32_nan();
            dec float_nan_result: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(float_nan);
            match float_nan_result {
                std::Result::Err(std::TryConvertError::Invalid) -> {},
                _ -> { return 28; },
            }
            dec float_inf: f32 = std::f32_infinity();
            dec float_inf_result: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(float_inf);
            match float_inf_result {
                std::Result::Err(std::TryConvertError::Overflow) -> {},
                _ -> { return 29; },
            }
            dec float_neg_inf: f64 = std::f64_neg_infinity();
            dec float_neg_inf_result: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(float_neg_inf);
            match float_neg_inf_result {
                std::Result::Err(std::TryConvertError::Underflow) -> {},
                _ -> { return 30; },
            }
            dec float64_ok: f64 = 42.75;
            dec float64_ok_result: std::Result<i32, std::TryConvertError> = std::TryConvert::try_convert(float64_ok);
            match float64_ok_result {
                std::Result::Ok(v) -> { if v != 42 { return 31; } },
                std::Result::Err(_) -> { return 32; },
            }

            return 0;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "test_standard_conversions failed with code {}", code);
        }
        Err(e) => {
            panic!("test_standard_conversions failed: {}", e);
        }
    }
}

#[test]
fn test_float_semantics_and_operations() {
    let sysroot = fresh_phase4c_sysroot();
    let dir = create_temp_dir("float_semantics");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let src = r#"
        import <float>;

        fn main() -> i32 {
            // Arithmetic
            dec a: f32 = 2.5 as f32;
            dec b: f32 = 1.25 as f32;
            if (a + b) != (3.75 as f32) { return 1; }
            if (a - b) != (1.25 as f32) { return 2; }
            if (a * b) != (3.125 as f32) { return 3; }
            if (a / b) != (2.0 as f32) { return 4; }
            if (-a) != ((0.0 as f32) - (2.5 as f32)) { return 5; }

            // NaN / infinity factories and IEEE 754 classification
            dec nan = std::f32_nan();
            if nan == nan { return 6; }
            if (nan != nan) == false { return 7; }
            if nan < (0.0 as f32) { return 8; }
            if nan > (0.0 as f32) { return 9; }
            if nan.is_nan() == false { return 10; }
            if a.is_nan() { return 11; }

            // IEEE 754 Signed zero
            dec pos_zero: f32 = 0.0 as f32;
            dec neg_zero: f32 = -(0.0 as f32);
            if (pos_zero == neg_zero) == false { return 12; }
            if pos_zero.is_sign_positive() == false { return 13; }
            if pos_zero.is_sign_negative() { return 14; }
            if neg_zero.is_sign_negative() == false { return 15; }
            if neg_zero.is_sign_positive() { return 16; }
            if pos_zero.to_bits() == neg_zero.to_bits() { return 17; }

            // Foundation operations
            dec neg_val: f32 = (0.0 as f32) - (1.5 as f32);
            if neg_val.abs() != (1.5 as f32) { return 18; }
            if (2.0 as f32).min(1.0 as f32) != (1.0 as f32) { return 19; }
            if (2.0 as f32).max(1.0 as f32) != (2.0 as f32) { return 20; }
            dec clamped = (5.0 as f32).clamp(1.0 as f32, 4.0 as f32);
            if clamped != (4.0 as f32) { return 21; }

            if std::f32_infinity().is_infinite() == false { return 22; }
            if std::f32_neg_infinity().is_sign_negative() == false { return 23; }
            if std::f64_nan().is_nan() == false { return 24; }
            if std::f64_infinity().is_infinite() == false { return 25; }
            if std::f64_neg_infinity().is_sign_negative() == false { return 26; }
            if std::f64_to_bits(std::f64_from_bits(9221120237041090560u64)) != 9221120237041090560u64 { return 27; }

            return 0;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "test_float_semantics_and_operations failed with code {}", code);
        }
        Err(e) => {
            panic!("test_float_semantics_and_operations failed: {}", e);
        }
    }
}

#[test]
fn test_float_to_int_truncation_bounds_valid() {
    let sysroot = fresh_phase4c_sysroot();
    let dir = create_temp_dir("float_to_int_valid");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let src = r#"
        fn main() -> i32 {
            // Blocker 1: -128.5f32 truncates toward zero to -128, which is valid i8!
            dec v1: i8 = (-128.5 as f32) as i8;
            if v1 != ((0 as i8) - (128 as i8)) { return 1; }

            dec v2: i8 = (127.99 as f32) as i8;
            if v2 != (127 as i8) { return 2; }

            dec v3: i32 = (-2147483648.75) as i32;
            if v3 != (-2147483647 - 1) { return 3; }

            dec v4: i32 = (2147483647.9) as i32;
            if v4 != 2147483647 { return 4; }

            dec v5: u8 = (255.99 as f32) as u8;
            if v5 != (255 as u8) { return 5; }

            dec v6: u8 = (0.99 as f32) as u8;
            if v6 != (0 as u8) { return 6; }

            return 0;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "test_float_to_int_truncation_bounds_valid failed with code {}", code);
        }
        Err(e) => {
            panic!("test_float_to_int_truncation_bounds_valid failed: {}", e);
        }
    }
}

fn assert_traps(name: &str, src: &str) {
    let sysroot = fresh_phase4c_sysroot();
    let dir = create_temp_dir(name);
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, stdout, stderr)) => {
            assert_ne!(
                code, 0,
                "Expected program to trap, but it exited with 0. stdout: {}, stderr: {}",
                stdout, stderr
            );
        }
        Err(_) => {} // Compilation or link error is also non-zero/failure
    }
}

#[test]
fn test_float_to_int_traps() {
    // 128.0f32 exceeds i8::MAX (127) -> traps
    assert_traps(
        "trap_upper_i8",
        r#"fn main() -> i32 { dec x: i8 = (128.0 as f32) as i8; return 0; }"#,
    );

    // -129.0f32 exceeds i8::MIN (-128) -> traps
    assert_traps(
        "trap_lower_i8",
        r#"fn main() -> i32 { dec x: i8 = (-129.0 as f32) as i8; return 0; }"#,
    );

    // -0.5f32 to unsigned u8 traps because x < 0.0
    assert_traps(
        "trap_neg_u8",
        r#"fn main() -> i32 { dec x: u8 = (-0.5 as f32) as u8; return 0; }"#,
    );

    // 256.0f32 to u8 traps because exceeds 255
    assert_traps(
        "trap_upper_u8",
        r#"fn main() -> i32 { dec x: u8 = (256.0 as f32) as u8; return 0; }"#,
    );

    // NaN to int traps
    assert_traps(
        "trap_nan_i32",
        r#"
            import <float>;
            fn main() -> i32 {
                dec nan = std::f32_from_bits(2143289344 as u32);
                dec x: i32 = nan as i32;
                return 0;
            }
        "#,
    );

    // Infinity to int traps
    assert_traps(
        "trap_inf_i32",
        r#"
            import <float>;
            fn main() -> i32 {
                dec inf = std::f32_from_bits(2139095040 as u32);
                dec x: i32 = inf as i32;
                return 0;
            }
        "#,
    );

    // clamp with min > max traps via panic()
    assert_traps(
        "trap_clamp_panic",
        r#"
            import <float>;
            fn main() -> i32 {
                dec c = (1.5 as f32).clamp(2.0 as f32, 1.0 as f32);
                return 0;
            }
        "#,
    );
}

#[test]
fn test_float_to_int_comptime_casts() {
    let sysroot = fresh_phase4c_sysroot();
    let dir = create_temp_dir("float_to_int_comptime");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let invalid_src = r#"
        const VALUE: i8 = 128.0 as i8;
        fn main() -> i32 { return VALUE as i32; }
    "#;
    let invalid_path = dir.join("invalid.ln");
    fs::write(&invalid_path, invalid_src).expect("write invalid comptime fixture");
    let result = check(invalid_path.to_str().unwrap(), invalid_src.to_string(), &opts);
    let diagnostics = result.expect_err("out-of-range comptime float-to-int cast must fail");
    assert!(
        diagnostics.iter().any(|d| {
            let message = d.message.to_lowercase();
            message.contains("cast") || message.contains("conversion") || message.contains("range")
        }),
        "expected a numeric conversion diagnostic, got: {:?}",
        diagnostics
    );
}

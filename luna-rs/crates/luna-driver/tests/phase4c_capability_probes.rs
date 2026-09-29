use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_probe_4c_{}_{}_{}",
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

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
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
fn probe_1_generic_trait_conversion_dispatch() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("probe_1_convert");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let src = r#"
        trait Convert<T> {
            fn convert(self: Self) -> T;
        }

        impl Convert<u16> for u8 {
            fn convert(self: Self) -> u16 {
                return self as u16;
            }
        }

        impl<T> Convert<T> for T {
            fn convert(self: Self) -> T {
                return self;
            }
        }

        fn test_convert<From: Convert<To>, To>(val: From) -> To {
            return val.convert();
        }

        fn main() -> i32 {
            dec x: u8 = 42 as u8;
            dec z: u16 = test_convert<u8, u16>(x);
            dec w: u8 = test_convert<u8, u8>(x);
            if z == (42 as u16) && w == (42 as u8) {
                return 0;
            }
            return 1;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "Probe 1 binary returned non-zero code");
            println!("Probe 1 (Generic trait conversion dispatch): PASSED");
        }
        Err(e) => {
            panic!("Probe 1 failed: {}", e);
        }
    }
}

#[test]
fn probe_2_static_trait_function_dispatch() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("probe_2_static_default");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let src = r#"
        trait Default {
            fn default() -> Self;
        }

        impl Default for i32 {
            fn default() -> Self {
                return 123;
            }
        }

        fn make_default_direct() -> i32 {
            return Default::default();
        }

        fn main() -> i32 {
            dec val: i32 = make_default_direct();
            if val == 123 {
                return 0;
            }
            return 1;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "Probe 2 binary returned non-zero code");
            println!("Probe 2 (Static trait function dispatch T::default()): PASSED");
        }
        Err(e) => {
            println!("Probe 2 failed: {}", e);
            panic!("Probe 2 static trait dispatch failed: {}", e);
        }
    }
}

#[test]
fn probe_3_bitcast_pointer_reinterpretation() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("probe_3_bitcast");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    // Tests whether raw pointer reinterpretation works between integers of equal width (e.g. u32 and i32)
    // and whether std::ptr::read is viable
    let src = r#"
        import <ptr>;

        fn u32_to_i32_bits(x: u32) -> i32 {
            unsafe {
                return std::ptr::read<i32>((&x as *u32) as *i32);
            }
        }

        fn main() -> i32 {
            dec u: u32 = 4294967295 as u32; // 0xFFFFFFFF
            dec i: i32 = u32_to_i32_bits(u);
            if i == -1 {
                return 0;
            }
            return 1;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            assert_eq!(code, 0, "Probe 3 binary returned non-zero code");
            println!("Probe 3 (Pointer representation reinterpretation): PASSED");
        }
        Err(e) => {
            panic!("Probe 3 failed: {}", e);
        }
    }
}

#[test]
fn probe_4_and_5_float_mvir_lowering_status() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("probe_4_5_float");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    // This probe tests what happens when float literal and negation are compiled today
    let src = r#"
        fn main() -> i32 {
            dec x: f32 = 1.5;
            dec y: f32 = -x;
            return 0;
        }
    "#;

    let res = run_binary_with_output(&dir, src, &opts);
    match res {
        Ok((code, _, _)) => {
            println!("Probe 4 & 5 compiled with code: {}", code);
        }
        Err(e) => {
            println!("Probe 4 & 5 (as expected before C-GAP-09): {}", e);
        }
    }
}

#[test]
fn probe_6_panic_provider_identity() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("probe_6_panic");
    let opts = CompilerOptions {
        search_paths: vec![sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    // Test import <core/panic> vs import <panic>
    let src_core_panic = r#"
        import <core/panic>;
        fn main() -> i32 {
            return 0;
        }
    "#;
    let res_core = run_binary_with_output(&dir, src_core_panic, &opts);
    assert!(res_core.is_ok(), "import <core/panic> should succeed");

    let dir2 = create_temp_dir("probe_6_panic2");
    let src_panic = r#"
        import <panic>;
        fn main() -> i32 {
            return 0;
        }
    "#;
    let res_panic = run_binary_with_output(&dir2, src_panic, &opts);
    match res_panic {
        Ok(_) => println!("import <panic> is already supported!"),
        Err(e) => println!("import <panic> currently fails (needs alias/manifest entry): {}", e),
    }
}

#[test]
fn rebuild_sysroot_in_isolated_directory() {
    let source_sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let temp_root = create_temp_dir("rebuild_sysroot");
    let isolated_external = temp_root.join("libs").join("external");
    copy_dir_all(&source_sysroot.external_dir(), &isolated_external)
        .expect("copy source sysroot to isolated directory");

    let isolated_sysroot = Sysroot::discover(Some(temp_root.to_str().unwrap()))
        .expect("discover isolated sysroot");
    SysrootBuilder::new(isolated_sysroot)
        .build_all(true)
        .expect("Failed to rebuild isolated sysroot");
}

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_builder::SysrootBuilder;
use luna_driver::{compile, CompilerOptions};

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}

fn remove_files_by_ext(dir: &Path, ext: &str) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                remove_files_by_ext(&p, ext);
            } else if p.extension().map_or(false, |e| e == ext) {
                let _ = fs::remove_file(&p);
            }
        }
    }
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_p4b_parity_{}_{}_{}",
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

fn locate_external_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("libs")
        .join("external")
}

fn fresh_external_artifacts() -> PathBuf {
    static BUILT_EXTERNAL: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BUILT_EXTERNAL.get_or_init(|| {
        let external_dir = locate_external_dir();
        let artifact_root = create_temp_dir("fresh_artifacts");
        let artifact_external = artifact_root.join("libs").join("external");
        copy_dir_all(&external_dir, &artifact_external).expect("copy current provider sources");
        SysrootBuilder::new(Sysroot::from_root(artifact_root.clone()).expect("fresh artifact sysroot"))
            .build_all(true)
            .expect("build fresh canonical provider artifacts from current sources");
        artifact_external
    }).clone()
}

struct ParityHarness {
    source_sysroot: PathBuf,
    llib_sysroot: PathBuf,
}

impl ParityHarness {
    fn new(prefix: &str) -> Self {
        let external_dir = fresh_external_artifacts();
        assert!(external_dir.join("sysroot.toml").exists(), "sysroot.toml required");

        let base_dir = create_temp_dir(prefix);
        let source_sysroot = base_dir.join("source_mode");
        let llib_sysroot = base_dir.join("llib_mode");

        let source_external = source_sysroot.join("libs").join("external");
        let artifact_external = llib_sysroot.join("libs").join("external");
        copy_dir_all(&external_dir, &source_external).expect("copy to source_sysroot");
        copy_dir_all(&external_dir, &artifact_external).expect("copy to llib_sysroot");

        // Source mode: keep ONLY .ln and sysroot.toml (strip all .llib and .obj)
        remove_files_by_ext(&source_external, "llib");
        remove_files_by_ext(&source_external, "obj");

        // Llib mode: keep ONLY .llib, .obj, and sysroot.toml (strip all .ln source files)
        remove_files_by_ext(&artifact_external, "ln");

        Self {
            source_sysroot,
            llib_sysroot,
        }
    }

    fn run_in_mode(&self, test_name: &str, is_source: bool, src: &str) -> Result<(i32, String, String), String> {
        let mode_name = if is_source { "source" } else { "llib" };
        let run_dir = create_temp_dir(&format!("{}_{}", test_name, mode_name));
        let src_path = run_dir.join("main.ln");
        let exe_path = run_dir.join(if cfg!(windows) { "main.exe" } else { "main" });
        fs::write(&src_path, src).map_err(|e| e.to_string())?;

        let sysroot_path = if is_source {
            &self.source_sysroot
        } else {
            &self.llib_sysroot
        };

        let opts = CompilerOptions {
            output_path: Some(exe_path.to_str().unwrap().to_string()),
            search_paths: vec![sysroot_path.to_str().unwrap().to_string()],
            no_link: false,
            quiet: true,
            ..Default::default()
        };

        let compile_res = compile(src_path.to_str().unwrap(), src.to_string(), &opts);
        if let Err(err) = compile_res {
            return Err(format!("Compilation failed in {} mode: {:?}", mode_name, err));
        }

        let output = Command::new(&exe_path)
            .output()
            .map_err(|e| format!("Execution failed in {} mode: {}", mode_name, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        Ok((code, stdout, stderr))
    }

    fn assert_parity(&self, test_name: &str, src: &str) {
        let (code_src, out_src, err_src) = self.run_in_mode(test_name, true, src)
            .expect("source-mode execution should succeed");
        assert_eq!(code_src, 0, "source-mode non-zero exit: {}\nstderr: {}", code_src, err_src);

        let (code_lib, out_lib, err_lib) = self.run_in_mode(test_name, false, src)
            .expect("llib-mode execution should succeed");
        assert_eq!(code_lib, 0, "llib-mode non-zero exit: {}\nstderr: {}", code_lib, err_lib);

        assert_eq!(
            code_src, code_lib,
            "Exit code disparity between source ({}) and llib ({})",
            code_src, code_lib
        );
        assert_eq!(
            out_src, out_lib,
            "Stdout disparity between source and llib modes:\nSOURCE:\n{}\nLLIB:\n{}",
            out_src, out_lib
        );
    }
}

#[test]
fn test_parity_vec_and_slices() {
    let harness = ParityHarness::new("vec_slice");

    let src = r#"
        import <vec>;
        import <slice>;
        import <cmp>;

        fn main() -> i32 {
            dec rw v = std::vec_new<i32>();
            v.push(40);
            v.push(10);
            v.push(30);
            v.push(20);
            v.push(50);

            // 1. swap_remove
            dec rem = v.swap_remove(0 as u64);
            if rem != 40 { return 1; }
            if v.len() != (4 as u64) { return 2; }

            // 2. slice_sort
            std::slice::slice_sort<i32>(v.as_mut_slice());
            dec s = v.as_slice();
            if s[0 as u64] != 10 { return 3; }
            if s[1 as u64] != 20 { return 4; }
            if s[2 as u64] != 30 { return 5; }
            if s[3 as u64] != 50 { return 6; }

            // 3. split_at
            dec (left, right) = std::slice::split_at<i32>(s, 2 as u64);
            if (left.len as u64) != (2 as u64) { return 7; }
            if (right.len as u64) != (2 as u64) { return 8; }
            if left[0 as u64] != 10 || left[1 as u64] != 20 { return 9; }
            if right[0 as u64] != 30 || right[1 as u64] != 50 { return 10; }

            // 4. clone and resize
            dec rw v2 = v.clone();
            if v2.len() != (4 as u64) { return 11; }
            v2.resize<i32>(2 as u64, 0);
            if v2.len() != (2 as u64) { return 12; }

            return 0;
        }
    "#;

    harness.assert_parity("vec_and_slices", src);
}

#[test]
fn test_parity_hashmap_and_hashset() {
    let harness = ParityHarness::new("map_set");

    let src = r#"
        import <hashmap>;
        import <hashset>;

        fn main() -> i32 {
            // 1. HashMap operations
            dec rw map = std::hashmap_new<i32, i32>();
            map.insert(1, 100);
            map.insert(2, 200);

            dec ok1 = map.insert_if_absent(1, 999);
            if ok1 != false { return 1; }

            dec ok2 = map.insert_if_absent(3, 300);
            if ok2 != true { return 2; }

            dec r1 = map.get_or_insert(3, 888);
            if *r1 != 300 { return 3; }

            dec r2 = map.get_or_insert(4, 400);
            if *r2 != 400 { return 4; }

            if map.len() != (4 as u64) { return 5; }

            // 2. HashSet operations
            dec rw set = std::hashset_new<i32>();
            set.insert(10);
            set.insert(20);
            set.insert(30);

            if set.contains(&10) == false { return 6; }
            if set.contains(&99) == true { return 7; }
            if set.len() != (3 as u64) { return 8; }

            dec removed = set.remove(&20);
            if removed == false { return 9; }
            if set.contains(&20) == true { return 10; }
            if set.len() != (2 as u64) { return 11; }

            return 0;
        }
    "#;

    harness.assert_parity("hashmap_and_hashset", src);
}

#[test]
fn test_parity_iter_range_combinators() {
    let harness = ParityHarness::new("iter_comb");

    let src = r#"
        import <iter_adapters>;
        import <iter_consumers>;
        import <result>;

        fn is_even(x: &i32) -> bool {
            return *x % 2 == 0;
        }

        fn extract_even_half(x: i32) -> std::Option<i32> {
            if x % 2 == 0 {
                return std::Option::Some(x / 2);
            }
            return std::Option::None;
        }

        fn main() -> i32 {
            // 1. Range sum
            dec rw it = std::iter::range(0, 5);
            dec sum = std::iter::iter_sum_i32(it);
            // 0 + 1 + 2 + 3 + 4 = 10
            if sum != 10 { return 1; }

            // 2. Option & Result combinators
            dec o1: std::Option<std::Option<i32>> = std::Option::Some(std::Option::Some(42));
            dec o_flat = std::option_flatten<i32>(o1);
            if o_flat.unwrap() != 42 { return 2; }

            dec r1: std::Result<std::Result<i32, i32>, i32> = std::Result::Ok(std::Result::Ok(99));
            dec r_flat = std::result_flatten<i32, i32>(r1);
            if r_flat.unwrap() != 99 { return 3; }

            // 3. Transpose
            dec ot: std::Option<std::Result<i32, i32>> = std::Option::Some(std::Result::Ok(77));
            dec res_trans = std::option_transpose<i32, i32>(ot);
            if res_trans.is_ok() == false { return 4; }

            return 0;
        }
    "#;

    harness.assert_parity("iter_range_combinators", src);
}

#[test]
fn test_parity_numerics_and_cmp() {
    let harness = ParityHarness::new("num_cmp");

    let src = r#"
        import <num>;
        import <cmp>;

        fn main() -> i32 {
            // 1. Checked arithmetic
            dec a1 = std::checked_add_i32(2147483647, 1);
            if a1.is_some() { return 1; }

            dec a2 = std::checked_div_i32(10, 0);
            if a2.is_some() { return 2; }

            dec a3 = std::checked_div_i32((0 - 2147483647) - 1, 0 - 1);
            if a3.is_some() { return 3; }

            dec a4 = std::checked_abs_i32((0 - 2147483647) - 1);
            if a4.is_some() { return 4; }

            // 2. Saturating arithmetic
            dec s1 = std::saturating_add_i32(2147483647, 100);
            if s1 != 2147483647 { return 5; }

            dec s2 = std::saturating_sub_u8(5 as u8, 10 as u8);
            if s2 != (0 as u8) { return 6; }

            // 3. Cmp min/max/clamp
            if std::min<i32>(10, 20) != 10 { return 7; }
            if std::max<i32>(10, 20) != 20 { return 8; }
            if std::clamp<i32>(5, 10, 20) != 10 { return 9; }
            if std::clamp<i32>(25, 10, 20) != 20 { return 10; }
            if std::clamp<i32>(15, 10, 20) != 15 { return 11; }

            return 0;
        }
    "#;

    harness.assert_parity("numerics_and_cmp", src);
}

#[test]
fn test_parity_memory_and_sorting() {
    let harness = ParityHarness::new("mem_sort");

    let src = r#"
        import <mem>;
        import <ptr>;
        import <vec>;
        import <slice>;
        import <cmp>;

        struct Point {
            x: i32,
            y: i32,
        };

        fn main() -> i32 {
            // 1. std::mem::swap
            dec rw a: i32 = 100;
            dec rw b: i32 = 200;
            {
                std::mem::swap<i32>(&rw a, &rw b);
            }
            if a != 200 { return 1; }
            if b != 100 { return 2; }

            dec rw p1 = Point { x: 10, y: 20 };
            dec rw p2 = Point { x: 30, y: 40 };
            {
                std::mem::swap<Point>(&rw p1, &rw p2);
            }
            if p1.x != 30 || p1.y != 40 { return 3; }
            if p2.x != 10 || p2.y != 20 { return 4; }

            // 2. std::mem::replace
            dec rw dest: i32 = 777;
            dec old = std::mem::replace<i32>(&rw dest, 888);
            if old != 777 { return 5; }
            if dest != 888 { return 6; }

            // 3. std::ptr::swap (distinct and identical)
            unsafe {
                dec pa = &rw a as *rw i32;
                dec pb = &rw b as *rw i32;
                std::ptr::swap<i32>(pa, pb);
                // a should be 100, b should be 200 again
                if *pa != 100 || *pb != 200 { return 7; }

                // identical pointer is valid no-op
                std::ptr::swap<i32>(pa, pa);
                if *pa != 100 { return 8; }
            }

            // 4. slice_sort
            dec rw v = std::vec_new<i32>();
            v.push(50);
            v.push(10);
            v.push(40);
            v.push(20);
            v.push(30);

            std::slice::slice_sort<i32>(v.as_mut_slice());
            dec s = v.as_slice();
            if s[0 as u64] != 10 { return 9; }
            if s[1 as u64] != 20 { return 10; }
            if s[2 as u64] != 30 { return 11; }
            if s[3 as u64] != 40 { return 12; }
            if s[4 as u64] != 50 { return 13; }

            return 0;
        }
    "#;

    harness.assert_parity("memory_and_sorting", src);
}

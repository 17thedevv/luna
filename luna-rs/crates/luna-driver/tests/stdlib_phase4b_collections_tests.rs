use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_col_{}_{}_{}",
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
fn test_hashmap_insert_if_absent() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("hm_iia");

    let src = r#"
        import <hashmap>;

        fn main() -> i32 {
            dec rw m = std::hashmap_new<i32, i32>();
            m.insert(1, 100);

            // key 1 exists, insert_if_absent must return false and not overwrite
            dec res1 = m.insert_if_absent(1, 999);
            if res1 { return 1; }
            match m.get(&1) {
                std::Option::Some(v) -> {
                    if *v != 100 { return 2; }
                },
                std::Option::None -> { return 3; },
            }

            // key 2 is absent, insert_if_absent must return true and insert
            dec res2 = m.insert_if_absent(2, 200);
            if res2 == false { return 4; }
            match m.get(&2) {
                std::Option::Some(v) -> {
                    if *v != 200 { return 5; }
                },
                std::Option::None -> { return 6; },
            }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("Execution must succeed");
    assert_eq!(code, 0, "test_hashmap_insert_if_absent failed with code {}: {}", code, stderr);
}

#[test]
fn test_hashmap_get_or_insert() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("hm_goi");

    let src = r#"
        import <hashmap>;

        fn main() -> i32 {
            dec rw m = std::hashmap_new<i32, i32>();

            // key 10 absent -> inserts 500 and returns &rw
            dec rw ref1 = m.get_or_insert(10, 500);
            if *ref1 != 500 { return 1; }

            // mutate via returned reference
            *ref1 = 555;

            // key 10 present -> returns existing value (555), default (999) dropped
            dec rw ref2 = m.get_or_insert(10, 999);
            if *ref2 != 555 { return 2; }

            // verify in map directly
            match m.get(&10) {
                std::Option::Some(v) -> {
                    if *v != 555 { return 3; }
                },
                std::Option::None -> { return 4; },
            }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("Execution must succeed");
    assert_eq!(code, 0, "test_hashmap_get_or_insert failed with code {}: {}", code, stderr);
}

#[test]
fn test_hashset_algebra_borrowed() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("hs_algebra");

    let src = r#"
        import <hashset>;
        import <clone>;

        fn main() -> i32 {
            dec rw s1 = std::hashset_new<i32>();
            s1.insert(1);
            s1.insert(2);
            s1.insert(3);

            dec rw s2 = std::hashset_new<i32>();
            s2.insert(2);
            s2.insert(3);
            s2.insert(4);

            // Phase 4A contract: union_owned borrows both sets
            dec u = s1.union_owned(&s2);
            // s1 and s2 must still be intact and valid!
            if s1.len() != (3 as u64) { return 1; }
            if s2.len() != (3 as u64) { return 2; }
            if u.len() != (4 as u64) { return 3; }
            if u.contains(&1) == false { return 4; }
            if u.contains(&2) == false { return 5; }
            if u.contains(&3) == false { return 6; }
            if u.contains(&4) == false { return 7; }

            // intersection_owned borrows both sets
            dec inter = s1.intersection_owned(&s2);
            if s1.len() != (3 as u64) { return 8; }
            if s2.len() != (3 as u64) { return 9; }
            if inter.len() != (2 as u64) { return 10; }
            if inter.contains(&2) == false { return 11; }
            if inter.contains(&3) == false { return 12; }
            if inter.contains(&1) { return 13; }
            if inter.contains(&4) { return 14; }

            // difference_owned borrows both sets
            dec diff = s1.difference_owned(&s2);
            if s1.len() != (3 as u64) { return 15; }
            if s2.len() != (3 as u64) { return 16; }
            if diff.len() != (1 as u64) { return 17; }
            if diff.contains(&1) == false { return 18; }
            if diff.contains(&2) { return 19; }

            // free-function form: std::union_owned(&s1, &s2)
            dec u2 = std::union_owned(&s1, &s2);
            if u2.len() != (4 as u64) { return 20; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("Execution must succeed");
    assert_eq!(code, 0, "test_hashset_algebra_borrowed failed with code {}: {}", code, stderr);
}

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_sort_{}_{}_{}",
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
fn test_sorting_adversarial_and_inconsistent() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("adv_and_inconsistent");

    let src = r#"
        import <vec>;
        import <slice>;
        import <cmp>;

        // Non-transitive / inconsistent comparator
        // Always claims every element is less than every other element (-1)
        fn inconsistent_cmp(a: &i32, b: &i32) -> i32 {
            if *a == *b {
                return 0;
            }
            return 0 - 1;
        }

        fn main() -> i32 {
            // 1. Adversarial input with duplicates: verify O(n log n) termination
            dec rw v = std::vec_new<i32>();
            dec rw i: i32 = 100;
            while i > 0 {
                v.push(i % 7);
                i = i - 1;
            }

            // Generic sort via Ord
            std::slice::slice_sort<i32>(v.as_mut_slice());

            // Verify monotonic non-decreasing order
            dec s = v.as_slice();
            dec rw j: u64 = 0 as u64;
            while j < (v.len() - (1 as u64)) {
                if s[j] > s[j + (1 as u64)] {
                    return 1;
                }
                j = j + (1 as u64);
            }

            // 2. Inconsistent comparator: MUST terminate without infinite loop
            // and must preserve all elements (multiset preservation)
            dec rw v_incon = std::vec_new<i32>();
            v_incon.push(4);
            v_incon.push(2);
            v_incon.push(9);
            v_incon.push(1);
            v_incon.push(7);

            std::slice::slice_sort_by<i32>(v_incon.as_mut_slice(), inconsistent_cmp);

            // Multiset check: length unchanged, sum unchanged
            if v_incon.len() != (5 as u64) {
                return 2;
            }
            dec s2 = v_incon.as_slice();
            dec sum = s2[0] + s2[1] + s2[2] + s2[3] + s2[4];
            if sum != (4 + 2 + 9 + 1 + 7) {
                return 3;
            }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("Execution must succeed");
    assert_eq!(code, 0, "test_sorting_adversarial_and_inconsistent failed with code {}: {}", code, stderr);
}

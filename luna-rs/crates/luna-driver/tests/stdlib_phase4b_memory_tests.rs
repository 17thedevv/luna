use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_mem_{}_{}_{}",
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
fn test_mem_swap_and_replace() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("swap_replace");

    let src = r#"
        import <mem>;
        import <ptr>;

        struct Point {
            x: i32,
            y: i32,
        };

        fn main() -> i32 {
            // 1. Primitive mem::swap
            dec rw a: i32 = 100;
            dec rw b: i32 = 200;
            mem::swap<i32>(&rw a, &rw b);
            if a != 200 { return 1; }
            if b != 100 { return 2; }

            // 2. Aggregate struct mem::swap
            dec rw p1 = Point { x: 1, y: 2 };
            dec rw p2 = Point { x: 8, y: 9 };
            mem::swap<Point>(&rw p1, &rw p2);
            if p1.x != 8 { return 3; }
            if p1.y != 9 { return 4; }
            if p2.x != 1 { return 5; }
            if p2.y != 2 { return 6; }

            // 3. mem::replace
            dec rw dest: i32 = 500;
            dec old = mem::replace<i32>(&rw dest, 999);
            if old != 500 { return 7; }
            if dest != 999 { return 8; }

            // 4. ptr::swap with identical pointers (no-op precondition)
            unsafe {
                dec ptr_a = &rw a as *rw i32;
                ptr::swap<i32>(ptr_a, ptr_a);
                if a != 200 { return 9; }
            }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("Execution must succeed");
    assert_eq!(code, 0, "test_mem_swap_and_replace failed with code {}: {}", code, stderr);
}

#[test]
fn test_ptr_swap_direct() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("ptr_swap");

    let src = r#"
        import <ptr>;

        struct DropProbe {
            id: i32,
            counter: *rw i32,
        };

        impl Drop for DropProbe {
            fn drop(self: &rw Self) {
                if (self.counter as u64) != (0 as u64) {
                    unsafe {
                        *self.counter = *self.counter + 1;
                    }
                }
            }
        }

        fn test_dropprobe_swap(drops: *rw i32) -> i32 {
            {
                dec rw p1 = DropProbe { id: 10, counter: drops };
                dec rw p2 = DropProbe { id: 20, counter: drops };

                unsafe {
                    dec ptr1 = &rw p1 as *rw DropProbe;
                    dec ptr2 = &rw p2 as *rw DropProbe;
                    ptr::swap<DropProbe>(ptr1, ptr2);
                }

                // During and immediately after swap, 0 drops have occurred
                unsafe {
                    if *drops != 0 { return 4; }
                }
                if p1.id != 20 { return 5; }
                if p2.id != 10 { return 6; }
            }
            // After exiting block scope, exactly 2 drops have occurred (p1 and p2 once each)
            unsafe {
                if *drops != 2 { return 7; }
            }
            return 0;
        }

        fn main() -> i32 {
            // Case 1: two distinct valid *rw T -> values swapped
            dec rw x: i32 = 111;
            dec rw y: i32 = 222;
            unsafe {
                dec px = &rw x as *rw i32;
                dec py = &rw y as *rw i32;
                ptr::swap<i32>(px, py);
            }
            if x != 222 { return 1; }
            if y != 111 { return 2; }

            // Case 2: a == b -> valid no-op
            unsafe {
                dec px2 = &rw x as *rw i32;
                ptr::swap<i32>(px2, px2);
            }
            if x != 222 { return 3; }

            // Case 3: non-Copy DropProbe -> no premature drops or drop duplication
            dec rw drop_count: i32 = 0;
            dec res = test_dropprobe_swap(&rw drop_count as *rw i32);
            if res != 0 { return res; }
            if drop_count != 2 { return 8; }

            return 0;
        }
    "#;

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];

    let (code, _stdout, stderr) = run_binary(&dir, src, &opts).expect("Execution must succeed");
    assert_eq!(code, 0, "test_ptr_swap_direct failed with code {}: {}", code, stderr);
}

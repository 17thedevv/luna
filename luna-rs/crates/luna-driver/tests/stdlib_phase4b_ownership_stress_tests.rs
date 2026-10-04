use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use luna_driver::{compile, CompilerOptions};
use luna_driver::sysroot::Sysroot;

fn create_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "luna_test_p4b_own_{}_{}_{}",
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

const PROBE_HEADER: &str = r#"
import <vec>;
import <hashmap>;
import <slice>;
import <mem>;
import <cmp>;
import <clone>;
import <hash>;

struct DropProbe {
    id: i32,
    counter: *rw i32,
};

impl std::Drop for DropProbe {
    fn drop(self: &rw Self) {
        if (self.counter as u64) != (0 as u64) {
            unsafe {
                *self.counter = *self.counter + 1;
            }
        }
    }
}

impl std::Clone for DropProbe {
    fn clone(self: &Self) -> Self {
        return DropProbe {
            id: self.id,
            counter: self.counter,
        };
    }
}

impl std::Eq for DropProbe {
    fn eq(self: &Self, other: &Self) -> bool {
        return self.id == other.id;
    }
}

impl std::Ord for DropProbe {
    fn cmp(self: &Self, other: &Self) -> i32 {
        if self.id < other.id { return 0 - 1; }
        if self.id > other.id { return 1; }
        return 0;
    }
}

impl std::Hash for DropProbe {
    fn hash(self: &Self) -> u64 {
        return self.id as u64;
    }
}
"#;

#[test]
fn test_ownership_stress_vec_swap_remove() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("swap_remove");

    let src = format!(r#"
        {}

        fn run_test(drops: *rw i32) -> i32 {{
            dec rw v = std::vec_new<DropProbe>();
            v.push(DropProbe {{ id: 10, counter: drops }});
            v.push(DropProbe {{ id: 20, counter: drops }});
            v.push(DropProbe {{ id: 30, counter: drops }});
            v.push(DropProbe {{ id: 40, counter: drops }});

            unsafe {{
                if *drops != 0 {{ return 1; }}
            }}

            // swap_remove element at index 1 (id: 20)
            // It moves the last element (40) into index 1
            {{
                dec removed = v.swap_remove(1 as u64);
                if removed.id != 20 {{ return 2; }}
                unsafe {{
                    // No drops during swap_remove!
                    if *drops != 0 {{ return 3; }}
                }}
            }}
            // `removed` dropped at closing brace of scope
            unsafe {{
                if *drops != 1 {{ return 4; }}
            }}

            if v.len() != (3 as u64) {{ return 5; }}
            dec s = v.as_slice();
            if s[0 as u64].id != 10 {{ return 6; }}
            if s[1 as u64].id != 40 {{ return 7; }}
            if s[2 as u64].id != 30 {{ return 8; }}

            return 0;
        }}

        fn main() -> i32 {{
            dec rw drop_count: i32 = 0;
            dec res = run_test(&rw drop_count as *rw i32);
            if res != 0 {{ return res; }}
            // When v exits run_test, all remaining 3 elements are dropped: total = 4
            if drop_count != 4 {{ return 9; }}
            return 0;
        }}
    "#, PROBE_HEADER);

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, &src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_ownership_stress_vec_retain() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("retain");

    let src = format!(r#"
        {}

        fn is_even(item: &DropProbe) -> bool {{
            return item.id % 2 == 0;
        }}

        fn run_test(drops: *rw i32) -> i32 {{
            dec rw v = std::vec_new<DropProbe>();
            v.push(DropProbe {{ id: 1, counter: drops }});
            v.push(DropProbe {{ id: 2, counter: drops }});
            v.push(DropProbe {{ id: 3, counter: drops }});
            v.push(DropProbe {{ id: 4, counter: drops }});
            v.push(DropProbe {{ id: 5, counter: drops }});
            v.push(DropProbe {{ id: 6, counter: drops }});

            unsafe {{
                if *drops != 0 {{ return 1; }}
            }}

            v.retain(is_even);

            // Odd elements 1, 3, 5 dropped during retain
            unsafe {{
                if *drops != 3 {{ return 2; }}
            }}

            if v.len() != (3 as u64) {{ return 3; }}
            dec s = v.as_slice();
            if s[0 as u64].id != 2 {{ return 4; }}
            if s[1 as u64].id != 4 {{ return 5; }}
            if s[2 as u64].id != 6 {{ return 6; }}

            return 0;
        }}

        fn main() -> i32 {{
            dec rw drop_count: i32 = 0;
            dec res = run_test(&rw drop_count as *rw i32);
            if res != 0 {{ return res; }}
            // When v exits, remaining 3 elements are dropped: total = 6
            if drop_count != 6 {{ return 7; }}
            return 0;
        }}
    "#, PROBE_HEADER);

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, &src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_ownership_stress_vec_dedup() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("dedup");

    let src = format!(r#"
        {}

        fn run_test(drops: *rw i32) -> i32 {{
            dec rw v = std::vec_new<DropProbe>();
            v.push(DropProbe {{ id: 1, counter: drops }});
            v.push(DropProbe {{ id: 1, counter: drops }});
            v.push(DropProbe {{ id: 2, counter: drops }});
            v.push(DropProbe {{ id: 2, counter: drops }});
            v.push(DropProbe {{ id: 2, counter: drops }});
            v.push(DropProbe {{ id: 3, counter: drops }});
            v.push(DropProbe {{ id: 4, counter: drops }});
            v.push(DropProbe {{ id: 4, counter: drops }});

            unsafe {{
                if *drops != 0 {{ return 1; }}
            }}

            v.dedup();

            // 4 duplicates dropped during dedup
            unsafe {{
                if *drops != 4 {{ return 2; }}
            }}

            if v.len() != (4 as u64) {{ return 3; }}
            dec s = v.as_slice();
            if s[0 as u64].id != 1 {{ return 4; }}
            if s[1 as u64].id != 2 {{ return 5; }}
            if s[2 as u64].id != 3 {{ return 6; }}
            if s[3 as u64].id != 4 {{ return 7; }}

            return 0;
        }}

        fn main() -> i32 {{
            dec rw drop_count: i32 = 0;
            dec res = run_test(&rw drop_count as *rw i32);
            if res != 0 {{ return res; }}
            // When v exits, remaining 4 elements are dropped: total = 8
            if drop_count != 8 {{ return 8; }}
            return 0;
        }}
    "#, PROBE_HEADER);

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, &src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_ownership_stress_vec_resize_and_clone() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("resize_clone");

    let src = format!(r#"
        {}

        fn run_shrink_test(drops: *rw i32) -> i32 {{
            dec rw v = std::vec_new<DropProbe>();
            v.push(DropProbe {{ id: 10, counter: drops }});
            v.push(DropProbe {{ id: 20, counter: drops }});
            v.push(DropProbe {{ id: 30, counter: drops }});
            v.push(DropProbe {{ id: 40, counter: drops }});
            v.push(DropProbe {{ id: 50, counter: drops }});

            // Shrink from 5 to 2: 3 elements dropped + 1 template value dropped
            v.resize(2 as u64, DropProbe {{ id: 99, counter: drops }});

            unsafe {{
                if *drops != 4 {{ return 1; }}
            }}
            if v.len() != (2 as u64) {{ return 2; }}
            dec s = v.as_slice();
            if s[0 as u64].id != 10 {{ return 3; }}
            if s[1 as u64].id != 20 {{ return 4; }}

            return 0;
        }}

        fn run_grow_and_clone_test(drops: *rw i32) -> i32 {{
            dec rw v = std::vec_new<DropProbe>();
            v.push(DropProbe {{ id: 1, counter: drops }});
            v.push(DropProbe {{ id: 2, counter: drops }});

            // Grow from 2 to 4 with id: 100: 2 clones pushed + 1 template value dropped
            v.resize(4 as u64, DropProbe {{ id: 100, counter: drops }});
            unsafe {{
                // Only the template argument dropped so far
                if *drops != 1 {{ return 5; }}
            }}
            if v.len() != (4 as u64) {{ return 6; }}

            // Clone vector: 4 elements cloned into v_clone
            {{
                dec v_clone = v.clone();
                if v_clone.len() != (4 as u64) {{ return 7; }}
                unsafe {{
                    // Still 1 drop! Clones are alive
                    if *drops != 1 {{ return 8; }}
                }}
            }}
            // v_clone drops at closing brace: 4 drops
            unsafe {{
                if *drops != 5 {{ return 9; }}
            }}

            return 0;
        }}

        fn main() -> i32 {{
            // Test shrink
            dec rw d1: i32 = 0;
            dec res1 = run_shrink_test(&rw d1 as *rw i32);
            if res1 != 0 {{ return res1; }}
            // When v drops: 2 remaining elements drop -> total = 6
            if d1 != 6 {{ return 10; }}

            // Test grow and clone
            dec rw d2: i32 = 0;
            dec res2 = run_grow_and_clone_test(&rw d2 as *rw i32);
            if res2 != 0 {{ return res2; }}
            // When v drops: 4 elements drop -> total = 9 (1 template + 4 clone + 4 original)
            if d2 != 9 {{ return 11; }}

            return 0;
        }}
    "#, PROBE_HEADER);

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, &src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_ownership_stress_hashmap_insert_if_absent_and_get_or_insert() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("map_stress");

    let src = format!(r#"
        {}

        fn run_insert_if_absent_test(drops: *rw i32) -> i32 {{
            dec rw map = std::hashmap_new<DropProbe, DropProbe>();

            // 1. First insert: key absent -> inserted -> 0 drops
            dec inserted = map.insert_if_absent(
                DropProbe {{ id: 1, counter: drops }},
                DropProbe {{ id: 10, counter: drops }}
            );
            if inserted == false {{ return 1; }}
            unsafe {{
                if *drops != 0 {{ return 2; }}
            }}

            // 2. Second insert with same key: key present -> not inserted
            // Both passed key and val arguments must be dropped!
            dec inserted2 = map.insert_if_absent(
                DropProbe {{ id: 1, counter: drops }},
                DropProbe {{ id: 99, counter: drops }}
            );
            if inserted2 == true {{ return 3; }}
            unsafe {{
                if *drops != 2 {{ return 4; }}
            }}

            if map.len() != (1 as u64) {{ return 5; }}

            return 0;
        }}

        fn run_get_or_insert_test(drops: *rw i32) -> i32 {{
            dec rw map = std::hashmap_new<DropProbe, DropProbe>();

            // 1. Key absent: inserted -> 0 drops
            {{
                dec r1 = map.get_or_insert(
                    DropProbe {{ id: 2, counter: drops }},
                    DropProbe {{ id: 20, counter: drops }}
                );
                if r1.id != 20 {{ return 6; }}
            }}
            unsafe {{
                if *drops != 0 {{ return 7; }}
            }}

            // 2. Key present: default_val dropped, key dropped -> 2 drops
            {{
                dec r2 = map.get_or_insert(
                    DropProbe {{ id: 2, counter: drops }},
                    DropProbe {{ id: 200, counter: drops }}
                );
                if r2.id != 20 {{ return 8; }}
            }}
            unsafe {{
                if *drops != 2 {{ return 9; }}
            }}

            if map.len() != (1 as u64) {{ return 10; }}

            return 0;
        }}

        fn main() -> i32 {{
            // Test insert_if_absent
            dec rw d1: i32 = 0;
            dec res1 = run_insert_if_absent_test(&rw d1 as *rw i32);
            if res1 != 0 {{ return res1; }}
            // Map exits: 1 key + 1 value dropped -> total = 4
            if d1 != 4 {{ return 11; }}

            // Test get_or_insert
            dec rw d2: i32 = 0;
            dec res2 = run_get_or_insert_test(&rw d2 as *rw i32);
            if res2 != 0 {{ return res2; }}
            // Map exits: 1 key + 1 value dropped -> total = 4
            if d2 != 4 {{ return 12; }}

            return 0;
        }}
    "#, PROBE_HEADER);

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, &src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

#[test]
fn test_ownership_stress_mem_swap_replace_and_slice_sort() {
    let sysroot = Sysroot::discover_for_test().expect("sysroot required");
    let dir = create_temp_dir("swap_replace_sort");

    let src = format!(r#"
        {}

        fn run_swap_test(drops: *rw i32) -> i32 {{
            dec rw p1 = DropProbe {{ id: 10, counter: drops }};
            dec rw p2 = DropProbe {{ id: 20, counter: drops }};

            {{
                std::mem::swap<DropProbe>(&rw p1, &rw p2);
            }}

            if p1.id != 20 {{ return 1; }}
            if p2.id != 10 {{ return 2; }}
            unsafe {{
                // std::mem::swap does not drop either element
                if *drops != 0 {{ return 3; }}
            }}

            return 0;
        }}

        fn run_replace_test(drops: *rw i32) -> i32 {{
            dec rw dest = DropProbe {{ id: 100, counter: drops }};
            {{
                dec old = std::mem::replace<DropProbe>(&rw dest, DropProbe {{ id: 200, counter: drops }});
                if old.id != 100 {{ return 4; }}
                unsafe {{
                    // Neither is dropped yet
                    if *drops != 0 {{ return 6; }}
                }}
            }}
            // old dropped at scope exit
            unsafe {{
                if *drops != 1 {{ return 7; }}
            }}
            if dest.id != 200 {{ return 5; }}

            return 0;
        }}

        fn run_sort_test(drops: *rw i32) -> i32 {{
            dec rw v = std::vec_new<DropProbe>();
            v.push(DropProbe {{ id: 50, counter: drops }});
            v.push(DropProbe {{ id: 10, counter: drops }});
            v.push(DropProbe {{ id: 40, counter: drops }});
            v.push(DropProbe {{ id: 20, counter: drops }});
            v.push(DropProbe {{ id: 30, counter: drops }});

            unsafe {{
                if *drops != 0 {{ return 8; }}
            }}

            {{
                std::slice::slice_sort<DropProbe>(v.as_mut_slice());
            }}

            // slice_sort moves elements via in-place swap, NO drops occur during sorting!
            unsafe {{
                if *drops != 0 {{ return 9; }}
            }}

            dec s = v.as_slice();
            if s[0 as u64].id != 10 {{ return 10; }}
            if s[1 as u64].id != 20 {{ return 11; }}
            if s[2 as u64].id != 30 {{ return 12; }}
            if s[3 as u64].id != 40 {{ return 13; }}
            if s[4 as u64].id != 50 {{ return 14; }}

            return 0;
        }}

        fn main() -> i32 {{
            // Test std::mem::swap
            dec rw d1: i32 = 0;
            dec res1 = run_swap_test(&rw d1 as *rw i32);
            if res1 != 0 {{ return res1; }}
            // When p1 and p2 drop -> total = 2
            if d1 != 2 {{ return 15; }}

            // Test std::mem::replace
            dec rw d2: i32 = 0;
            dec res2 = run_replace_test(&rw d2 as *rw i32);
            if res2 != 0 {{ return res2; }}
            // When dest drops -> total = 2 (old + dest)
            if d2 != 2 {{ return 16; }}

            // Test slice_sort
            dec rw d3: i32 = 0;
            dec res3 = run_sort_test(&rw d3 as *rw i32);
            if res3 != 0 {{ return res3; }}
            // When v drops -> all 5 elements drop -> total = 5
            if d3 != 5 {{ return 17; }}

            return 0;
        }}
    "#, PROBE_HEADER);

    let mut opts = CompilerOptions::default();
    opts.search_paths = vec![sysroot.root().to_string_lossy().to_string()];
    let (code, _stdout, stderr) = run_binary(&dir, &src, &opts).expect("execution should succeed");
    assert_eq!(code, 0, "stderr: {}", stderr);
}

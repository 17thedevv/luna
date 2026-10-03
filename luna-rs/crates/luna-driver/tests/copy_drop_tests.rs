use luna_driver::{check, CompilerOptions};
use luna_driver::sysroot::Sysroot;
use std::fs;
use std::path::PathBuf;

fn create_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("luna_copy_drop_tests").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
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

fn remove_artifacts(dir: &std::path::Path) {
    for entry in fs::read_dir(dir).expect("read source sysroot directory") {
        let path = entry.expect("read sysroot entry").path();
        if path.is_dir() {
            remove_artifacts(&path);
        } else if matches!(path.extension().and_then(|ext| ext.to_str()), Some("llib" | "obj")) {
            fs::remove_file(path).expect("remove stale provider artifact");
        }
    }
}

fn source_sysroot(name: &str) -> Sysroot {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let external_dir = manifest_dir.parent().expect("crates directory")
        .parent().expect("workspace root").join("libs").join("external");
    let root = create_temp_dir(&format!("source_sysroot_{name}"));
    let source_external = root.join("libs").join("external");
    copy_dir_all(&external_dir, &source_external);
    remove_artifacts(&source_external);
    Sysroot::from_root(root).expect("source-only sysroot")
}

fn run_compiler(name: &str, src: &str) -> (bool, Vec<luna_common::Diagnostic>) {
    let test_sysroot = source_sysroot(name);
    let temp = create_temp_dir(name);
    let main_path = temp.join("main.ln");
    fs::write(&main_path, src).unwrap();

    let options = CompilerOptions {
        search_paths: vec![test_sysroot.root().to_string_lossy().to_string()],
        quiet: true,
        ..Default::default()
    };

    let res = check(main_path.to_str().unwrap(), src.to_string(), &options);
    match res {
        Ok(_) => (true, vec![]),
        Err(diags) => (false, diags),
    }
}

#[test]
fn test_mutual_exclusivity_copy_drop() {
    let src = r#"
        import <core/panic>;
        import <mem>;
        import <slice>;
        import <copy>;
        import <clone>;
        import <ptr>;
        import <iter_adapters>;
        import <iter_consumers>;
        struct MyStruct {
            x: i32,
        };

        impl std::Drop for MyStruct {
            fn drop(self: &rw Self) {}
        }

        impl std::Copy for MyStruct {}

        fn main() -> i32 { return 0; }
    "#;

    let (success, diagnostics) = run_compiler("mutual_exclusivity", src);
    assert!(!success, "Compilation should fail when implementing both Copy and Drop");
    
    let has_conflict_error = diagnostics.iter().any(|d| 
        d.message.contains("E_COPY_DROP_CONFLICT")
    );
    assert!(has_conflict_error, "Expected E_COPY_DROP_CONFLICT error, found: {:?}", diagnostics);
}

#[test]
fn test_transitive_copy_drop_conflict() {
    let src = r#"
        import <core/panic>;
        import <mem>;
        import <slice>;
        import <copy>;
        import <clone>;
        import <ptr>;
        import <iter_adapters>;
        import <iter_consumers>;
        struct Inner {
            x: i32,
        };

        impl std::Drop for Inner {
            fn drop(self: &rw Self) {}
        }

        struct Outer {
            inner: Inner,
        };

        // Implementing std::Copy for Outer should fail because its field `inner` needs drop
        impl std::Copy for Outer {}

        fn main() -> i32 { return 0; }
    "#;

    let (success, diagnostics) = run_compiler("transitive_conflict", src);
    assert!(!success, "Compilation should fail due to transitive drop requirement");
    
    let has_conflict_error = diagnostics.iter().any(|d| 
        d.message.contains("E_COPY_DROP_CONFLICT")
    );
    assert!(has_conflict_error, "Expected E_COPY_DROP_CONFLICT error, found: {:?}", diagnostics);
}

#[test]
fn test_valid_copy() {
    let src = r#"
        import <core/panic>;
        import <mem>;
        import <slice>;
        import <copy>;
        import <clone>;
        import <ptr>;
        import <iter_adapters>;
        import <iter_consumers>;
        struct ValidStruct {
            x: i32,
        };

        impl std::Copy for ValidStruct {}

        fn main() -> i32 { return 0; }
    "#;

    let (success, diagnostics) = run_compiler("valid_copy", src);
    assert!(success, "Compilation should succeed for valid Copy impl. Found diagnostics: {:?}", diagnostics);
}

#[test]
fn test_derive_copy_drop_conflict() {
    let src = r#"
        import <core/panic>;
        import <mem>;
        import <slice>;
        import <copy>;
        import <clone>;
        import <ptr>;
        import <iter_adapters>;
        import <iter_consumers>;
        struct Resource {
            x: i32,
        };

        impl std::Drop for Resource {
            fn drop(self: &rw Self) {}
        }

        #[derive(Copy)]
        struct Bad {
            resource: Resource,
        };

        fn main() -> i32 { return 0; }
    "#;

    let (success, diagnostics) = run_compiler("derive_conflict", src);
    assert!(!success, "Compilation should fail when deriving Copy on struct with Drop field");
    
    let has_conflict_error = diagnostics.iter().any(|d| 
        d.message.contains("E_COPY_DROP_CONFLICT")
    );
    assert!(has_conflict_error, "Expected E_COPY_DROP_CONFLICT error, found: {:?}", diagnostics);
}

#[test]
fn test_generic_instantiation_copy_drop() {
    let src = r#"
        import <core/panic>;
        import <mem>;
        import <slice>;
        import <copy>;
        import <clone>;
        import <ptr>;
        import <iter_adapters>;
        import <iter_consumers>;
        struct Resource {
            x: i32,
        };

        impl std::Drop for Resource {
            fn drop(self: &rw Self) {}
        }

        struct Wrapper<T> {
            value: T,
        };

        impl<T> std::Copy for Wrapper<T> {}

        fn main() -> i32 { 
            dec r = Resource { x: 42 };
            dec w = Wrapper<Resource> { value: r };
            return 0; 
        }
    "#;

    let (success, diagnostics) = run_compiler("generic_conflict", src);
    assert!(!success, "Compilation should fail when instantiating Copy generic with Drop type");
    
    let has_conflict_error = diagnostics.iter().any(|d| 
        d.message.contains("E_COPY_DROP_CONFLICT")
    );
    assert!(has_conflict_error, "Expected E_COPY_DROP_CONFLICT error, found: {:?}", diagnostics);
}

#[path = "support/stdlib.rs"]
mod support;

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use support::{render, workspace_root, ProviderModes};

fn publish(root: &Path, dir: &Path, name: &str, source: &str) -> Output {
    let input = dir.join(format!("{name}.ln"));
    fs::write(&input, source).unwrap();
    Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(input)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(dir.join(format!("{name}.llib")))
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap()
}

#[test]
fn native_calls_relink_but_materialized_bodies_invalidate() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/artifacts");
    let consumer = fs::read_to_string(fixtures.join("consumer.ln")).unwrap();
    for (case, dependency, wrapper, source_dependency, invalidate) in [
        (
            "native",
            "native_dependency",
            "native_wrapper",
            false,
            false,
        ),
        (
            "generic",
            "generic_dependency",
            "generic_wrapper",
            false,
            true,
        ),
        (
            "comptime",
            "native_dependency",
            "comptime_wrapper",
            false,
            true,
        ),
        (
            "bundled_source",
            "native_dependency",
            "native_wrapper",
            true,
            true,
        ),
    ] {
        let dir = modes.artifact.join(case);
        fs::create_dir(&dir).unwrap();
        let a = fs::read_to_string(fixtures.join(format!("{dependency}.ln"))).unwrap();
        let b = fs::read_to_string(fixtures.join(format!("{wrapper}.ln"))).unwrap();
        if source_dependency {
            fs::write(dir.join("a.ln"), &a).unwrap();
        } else {
            let result = publish(&modes.artifact, &dir, "a", &a);
            assert!(result.status.success(), "{case} A: {}", render(&result));
            fs::remove_file(dir.join("a.ln")).unwrap();
        }
        let result = publish(&modes.artifact, &dir, "b", &b);
        assert!(result.status.success(), "{case} B: {}", render(&result));
        fs::remove_file(dir.join("b.ln")).unwrap();
        let input = dir.join("main.ln");
        fs::write(&input, &consumer).unwrap();
        let (exe, result) = modes.build(&modes.artifact, &input, &format!("fresh_{case}"));
        assert!(result.status.success(), "{case} fresh: {}", render(&result));
        assert_eq!(Command::new(exe).status().unwrap().code(), Some(1));

        let changed = a.replace("return 1;", "return 2;");
        if source_dependency {
            fs::write(dir.join("a.ln"), changed).unwrap();
        } else {
            let result = publish(&modes.artifact, &dir, "a", &changed);
            assert!(
                result.status.success(),
                "{case} changed A: {}",
                render(&result)
            );
            fs::remove_file(dir.join("a.ln")).unwrap();
        }
        let (exe, result) = modes.build(&modes.artifact, &input, &format!("changed_{case}"));
        if invalidate {
            assert!(!result.status.success(), "{case} stale body was accepted");
            assert!(
                String::from_utf8_lossy(&result.stderr)
                    .contains("dependency execution fingerprint mismatch for provider a"),
                "{case} incorrect rejection: {}",
                render(&result)
            );
        } else {
            assert!(
                result.status.success(),
                "native call should relink: {}",
                render(&result)
            );
            assert_eq!(Command::new(exe).status().unwrap().code(), Some(2));
        }
    }
}

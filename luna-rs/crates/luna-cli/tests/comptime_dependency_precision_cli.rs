//! C4 freeze — comptime execution-dependency precision.
//!
//! In the tested scope the behaviour must be sound and sufficiently precise: a
//! changed execution dependency must invalidate the dependent artifact, while a
//! changed provider that was only imported (never executed at comptime) must not.
//! Branch-sensitive selection is not asserted here (see C4-FU1). `interface
//! fingerprint != execution fingerprint` is preserved throughout.
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
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&input)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(dir.join(format!("{name}.llib")))
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    let _ = fs::remove_file(&input);
    output
}

#[test]
fn comptime_execution_dependencies_are_precise() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/comptime_dependencies");
    let read = |name: &str| fs::read_to_string(fixtures.join(format!("{name}.ln"))).unwrap();
    let mut failures = Vec::new();

    // C4-1 direct execution dependency (+ C4-5 same interface, changed body;
    // + C4-7 typed stale diagnostic).
    {
        let dir = modes.artifact.join("direct");
        fs::create_dir_all(&dir).unwrap();
        let dep = publish(&modes.artifact, &dir, "c4_dep", &read("c4_dep"));
        let direct = publish(&modes.artifact, &dir, "c4_direct", &read("c4_direct"));
        if !dep.status.success() || !direct.status.success() {
            failures.push(format!("C4-1 publish: dep {} direct {}", render(&dep), render(&direct)));
        }
        let consumer = dir.join("main.ln");
        fs::write(&consumer, read("c4_consumer_direct")).unwrap();
        let (exe, build) = modes.build(&modes.artifact, &consumer, "c4_direct_fresh");
        if !build.status.success() || Command::new(exe).status().unwrap().code() != Some(0) {
            failures.push(format!("C4-1 fresh: {}", render(&build)));
        }
        let changed = read("c4_dep").replace("return 1;", "return 2;");
        assert!(publish(&modes.artifact, &dir, "c4_dep", &changed).status.success());
        let (exe, build) = modes.build(&modes.artifact, &consumer, "c4_direct_changed");
        if build.status.success() {
            failures.push("C4-1/5/7: changed execution dependency was accepted".to_string());
        } else if !String::from_utf8_lossy(&build.stderr)
            .contains("dependency execution fingerprint mismatch")
        {
            failures.push(format!("C4-7 diagnostic: {}", render(&build)));
        }
        if exe.exists() && build.status.success() {
            failures.push("C4-7: published a stale executable".to_string());
        }
    }

    // C4-2 transitive execution dependency.
    {
        let dir = modes.artifact.join("transitive");
        fs::create_dir_all(&dir).unwrap();
        let leaf = publish(&modes.artifact, &dir, "c4_leaf", &read("c4_leaf"));
        let mid = publish(&modes.artifact, &dir, "c4_mid", &read("c4_mid"));
        let top = publish(&modes.artifact, &dir, "c4_top", &read("c4_top"));
        if !leaf.status.success() || !mid.status.success() || !top.status.success() {
            failures.push(format!(
                "C4-2 publish: leaf {} mid {} top {}",
                render(&leaf),
                render(&mid),
                render(&top)
            ));
        }
        let consumer = dir.join("main.ln");
        fs::write(&consumer, read("c4_consumer_top")).unwrap();
        let (exe, build) = modes.build(&modes.artifact, &consumer, "c4_top_fresh");
        if !build.status.success() || Command::new(exe).status().unwrap().code() != Some(0) {
            failures.push(format!("C4-2 fresh: {}", render(&build)));
        }
        let changed = read("c4_leaf").replace("return 3;", "return 5;");
        assert!(publish(&modes.artifact, &dir, "c4_leaf", &changed).status.success());
        let (_, build) = modes.build(&modes.artifact, &consumer, "c4_top_changed");
        if build.status.success() {
            failures.push("C4-2: transitive execution dependency change was accepted".to_string());
        }
    }

    // C4-3 unused imported provider must not become an execution dependency.
    {
        let dir = modes.artifact.join("unused");
        fs::create_dir_all(&dir).unwrap();
        assert!(publish(&modes.artifact, &dir, "c4_dep", &read("c4_dep")).status.success());
        assert!(publish(&modes.artifact, &dir, "c4_used", &read("c4_used")).status.success());
        assert!(publish(&modes.artifact, &dir, "c4_unused", &read("c4_unused")).status.success());
        let consumer = dir.join("main.ln");
        fs::write(&consumer, read("c4_consumer_unused")).unwrap();
        let (exe, build) = modes.build(&modes.artifact, &consumer, "c4_unused_fresh");
        if !build.status.success() || Command::new(exe).status().unwrap().code() != Some(0) {
            failures.push(format!("C4-3 fresh: {}", render(&build)));
        }
        let changed = read("c4_unused").replace("return 9;", "return 11;");
        assert!(publish(&modes.artifact, &dir, "c4_unused", &changed).status.success());
        let (_, build) = modes.build(&modes.artifact, &consumer, "c4_unused_changed");
        if !build.status.success() {
            failures.push(format!(
                "C4-3: unused imported provider invalidated the consumer (false positive): {}",
                render(&build)
            ));
        }
    }

    // C4-6 source/fresh-artifact parity for a direct execution dependency.
    {
        let dir = modes.source.join("direct_source");
        fs::create_dir_all(&dir).unwrap();
        fs::copy(fixtures.join("c4_dep.ln"), dir.join("c4_dep.ln")).unwrap();
        fs::copy(fixtures.join("c4_direct.ln"), dir.join("c4_direct.ln")).unwrap();
        let consumer = dir.join("main.ln");
        fs::write(&consumer, read("c4_consumer_direct")).unwrap();
        let (exe, build) = modes.build(&modes.source, &consumer, "c4_direct_source");
        if !build.status.success() || Command::new(exe).status().unwrap().code() != Some(0) {
            failures.push(format!("C4-6 source: {}", render(&build)));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

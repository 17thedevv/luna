//! D3 freeze — provider Tier-2 / Tier-3 matrix.
//!
//! Lock the boundaries that were historically fragile: public interface ->
//! serialized metadata -> dependency reconstruction -> consumer resolution, and
//! a three-provider chain `consumer -> a -> b -> c` across all-source,
//! partially-artifact, all-artifact and relocated-artifact graphs.
#[path = "support/stdlib.rs"]
mod support;

use std::{fs, path::Path, process::Command};
use support::{render, workspace_root, ProviderModes};

fn publish(root: &Path, dir: &Path, name: &str, source: &str) -> std::process::Output {
    fs::create_dir_all(dir).unwrap();
    let input = dir.join(format!("{name}.ln"));
    fs::write(&input, source).unwrap();
    Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&input)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(dir.join(format!("{name}.llib")))
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap()
}

fn run_consumer(
    modes: &ProviderModes,
    root: &Path,
    dir: &Path,
    consumer: &str,
    tag: &str,
    failures: &mut Vec<String>,
) {
    let input = dir.join("main.ln");
    fs::write(&input, consumer).unwrap();
    let (exe, build) = modes.build(root, &input, tag);
    if !build.status.success() {
        failures.push(format!("{tag} build: {}", render(&build)));
        return;
    }
    let run = Command::new(exe).current_dir(std::env::temp_dir()).output().unwrap();
    if run.status.code() != Some(0) {
        failures.push(format!("{tag} run: {}", render(&run)));
    }
}

fn fixture(name: &str) -> String {
    fs::read_to_string(
        workspace_root()
            .parent()
            .unwrap()
            .join(format!("tests/luna/language/provider_tiers/{name}.ln")),
    )
    .unwrap()
}

fn setup_tier3(root: &Path, dir: &Path, mode: &str) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    let (a, b, c) = (fixture("a"), fixture("b"), fixture("c"));
    let c_source = matches!(mode, "all-source" | "a-artifact" | "ab-artifact");
    let b_source = matches!(mode, "all-source" | "a-artifact");
    let a_source = mode == "all-source";
    if c_source {
        fs::write(dir.join("c.ln"), &c).unwrap();
    } else {
        assert!(publish(root, dir, "c", &c).status.success());
    }
    if b_source {
        fs::write(dir.join("b.ln"), &b).unwrap();
    } else {
        assert!(publish(root, dir, "b", &b).status.success());
    }
    if a_source {
        fs::write(dir.join("a.ln"), &a).unwrap();
    } else {
        assert!(publish(root, dir, "a", &a).status.success());
    }
    if mode == "relocated" {
        let moved = dir.with_file_name(format!(
            "{}_moved",
            dir.file_name().unwrap().to_string_lossy()
        ));
        let _ = fs::remove_dir_all(&moved);
        fs::rename(dir, &moved).unwrap();
    }
}

fn relocated_dir(dir: &Path) -> std::path::PathBuf {
    dir.with_file_name(format!("{}_moved", dir.file_name().unwrap().to_string_lossy()))
}

#[test]
fn tier3_graph_resolves_in_every_provider_mode() {
    let modes = ProviderModes::fresh();
    let mut failures = Vec::new();
    for mode in ["all-source", "a-artifact", "ab-artifact", "all-artifact", "relocated"] {
        let dir = modes.artifact.join(format!("tier3_{mode}"));
        setup_tier3(&modes.artifact, &dir, mode);
        let graph = if mode == "relocated" {
            relocated_dir(&dir)
        } else {
            dir
        };
        run_consumer(
            &modes,
            &modes.artifact,
            &graph,
            &fixture("consumer_tier3"),
            &format!("d3_tier3_{mode}"),
            &mut failures,
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn tier2_feature_surface_survives_the_artifact_boundary() {
    let modes = ProviderModes::fresh();
    let mut failures = Vec::new();
    // Source: provider .ln alongside the consumer.
    let dir = modes.source.join("tier2_source");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("feature_provider.ln"), fixture("feature_provider")).unwrap();
    run_consumer(
        &modes,
        &modes.source,
        &dir,
        &fixture("consumer_tier2"),
        "d3_tier2_source",
        &mut failures,
    );
    // Artifact: provider .llib only (private helper/calls must still materialize).
    let dir = modes.artifact.join("tier2_artifact");
    assert!(publish(&modes.artifact, &dir, "feature_provider", &fixture("feature_provider")).status.success());
    run_consumer(
        &modes,
        &modes.artifact,
        &dir,
        &fixture("consumer_tier2"),
        "d3_tier2_artifact",
        &mut failures,
    );
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn missing_dependency_fails_closed_without_source_fallback() {
    let modes = ProviderModes::fresh();
    let dir = modes.artifact.join("missing");
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("main.ln");
    fs::write(&input, fixture("consumer_missing")).unwrap();
    let check = modes.check(&modes.artifact, &input);
    let (exe, build) = modes.build(&modes.artifact, &input, "d3_missing");
    assert!(!exe.exists(), "missing dependency published an executable");
    for (command, output) in [("check", check), ("build", build)] {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.code() == Some(1) && stderr.contains("error[E1004]"),
            "{command}: expected E1004: {}",
            render(&output)
        );
    }
}

#[test]
fn zero_parameter_async_export_probe() {
    // FIND-ASYNC-PROVIDER-01 probe: record (do not fix here).
    let modes = ProviderModes::fresh();
    let dir = modes.artifact.join("async_probe");
    let published = publish(&modes.artifact, &dir, "async_provider", &fixture("async_provider"));
    assert!(
        published.status.success(),
        "async provider publish: {}",
        render(&published)
    );
    fs::remove_file(dir.join("async_provider.ln")).unwrap();
    let input = dir.join("main.ln");
    fs::write(&input, "import \"async_provider\";\nfn main() -> i32 { return 0; }\n").unwrap();
    let (_, build) = modes.build(&modes.artifact, &input, "d3_async_probe");
    let stderr = String::from_utf8_lossy(&build.stderr);
    if build.status.success() {
        eprintln!("D3 note: FIND-ASYNC-PROVIDER-01 no longer reproduces");
    } else if stderr.contains("CorruptedData") {
        eprintln!("D3 external finding reproduced: FIND-ASYNC-PROVIDER-01");
    } else {
        eprintln!("D3 note: async probe rejected differently: {}", render(&build));
    }
}

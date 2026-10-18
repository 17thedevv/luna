//! DIAG-3/6/9: joined move paths retain a deterministic structural antecedent.
#[path = "support/stdlib.rs"]
mod support;

use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use support::{render, workspace_root, ProviderModes};

fn marked_line(path: &Path, marker: &str) -> u32 {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .position(|line| line.contains(marker))
        .unwrap() as u32
        + 1
}

fn locations(output: &Output) -> Vec<(String, u32, u32)> {
    String::from_utf8_lossy(&output.stderr)
        .lines()
        .filter_map(|line| {
            let location = line.trim_start().strip_prefix("-->")?.trim();
            let mut parts = location.rsplitn(3, ':');
            let col = parts.next()?.parse().ok()?;
            let row = parts.next()?.parse().ok()?;
            let file = parts.next()?.replace('\\', "/");
            Some((file, row, col))
        })
        .collect()
}

fn assert_origin(
    output: &Output,
    file: &str,
    primary: u32,
    origin: u32,
) -> Vec<(String, u32, u32)> {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{}", render(output));
    assert_eq!(
        stderr
            .lines()
            .filter(|line| line.starts_with("error["))
            .count(),
        1,
        "{}",
        render(output)
    );
    assert!(stderr.contains("error[E3001]"), "{}", render(output));
    let sites = locations(output);
    assert_eq!(
        sites.len(),
        2,
        "primary plus one structural origin required: {}",
        render(output)
    );
    assert!(
        sites[0].0.ends_with(file) && sites[1].0.ends_with(file),
        "{sites:?}"
    );
    assert_eq!(sites[0].1, primary, "wrong invalid-event span: {sites:?}");
    assert_eq!(sites[1].1, origin, "wrong antecedent span: {sites:?}");
    sites
}

#[test]
fn move_origins_survive_nested_and_equal_state_joins_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/move_origin_join");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("move_origin_join");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let build = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(provider.with_extension("llib"))
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(build.status.success(), "{}", render(&build));
            fs::remove_file(provider).unwrap();
        }
        for name in [
            "nested_else",
            "nested_then",
            "equal_states",
            "loop_join",
            "field_reinitialized",
            "collapsed_fields",
            "imported_generic",
        ] {
            let fixture = project.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &fixture).unwrap();
            let subject = if name == "imported_generic" {
                "provider.ln".to_string()
            } else {
                format!("{name}.ln")
            };
            let primary = marked_line(&fixtures.join(&subject), "// invalid");
            let origin = marked_line(&fixtures.join(&subject), "// origin");
            let rendered_subject = if name == "imported_generic" && mode == "artifact" {
                "provider.llib"
            } else {
                &subject
            };
            let first = assert_origin(
                &modes.check(root, &fixture),
                rendered_subject,
                primary,
                origin,
            );
            let second = assert_origin(
                &modes.check(root, &fixture),
                rendered_subject,
                primary,
                origin,
            );
            assert_eq!(first, second, "nondeterministic origin: {mode}/{name}");
            let (exe, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            assert!(!exe.exists(), "invalid move published native output");
            let built = assert_origin(&build, rendered_subject, primary, origin);
            assert_eq!(first, built, "check/build origin differs: {mode}/{name}");
            eprintln!("PASS {mode}/{name}: E3001 primary {primary}, origin {origin}; repeated check/build");
        }
        let control = project.join("controls.ln");
        fs::copy(fixtures.join("controls.ln"), &control).unwrap();
        assert!(modes.check(root, &control).status.success());
        let (exe, build) = modes.build(root, &control, &format!("{mode}_controls"));
        assert!(build.status.success(), "{}", render(&build));
        let run = Command::new(exe).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/controls: reinitialization and valid cleanup execute natively");
    }
}

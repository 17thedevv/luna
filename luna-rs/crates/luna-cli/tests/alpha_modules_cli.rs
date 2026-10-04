#[path = "support/stdlib.rs"]
mod support;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{render, workspace_root, ProviderModes};

fn fixtures() -> PathBuf {
    workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/alpha_modules")
}
fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}
fn cli(root: &Path, cwd: &Path, command: &str, input: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg(command)
        .arg(input)
        .args(args)
        .arg("--quiet")
        .arg("-I")
        .arg(root)
        .env("LUNA_SYSROOT", root)
        .current_dir(cwd)
        .output()
        .unwrap()
}
fn accepted(output: &Output, case: &str) {
    assert!(output.status.success(), "{case}: {}", render(output));
}
fn rejected(output: &Output, code: &str, case: &str) {
    assert!(
        !output.status.success(),
        "{case} unexpectedly accepted: {}",
        render(output)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(code),
        "{case} lacks {code}: {}",
        render(output)
    );
}

#[test]
fn alpha_modules_source_and_fresh_artifact_contracts() {
    let modes = ProviderModes::fresh();
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        eprintln!("Starting alpha module matrix: {mode}");
        let work = root.join("project");
        fs::create_dir_all(&work).unwrap();
        for name in [
            "opening",
            "alias_dedup",
            "precedence",
            "nested_precedence",
            "child_namespace",
            "qualification",
            "macro_opening",
            "macro_hygiene",
            "macro_private_hygiene",
            "parameter_precedence",
            "macro_qualified_shadow",
            "macro_shadow_scope_exit",
        ] {
            let fixture = fixtures().join("using").join(format!("{name}.ln"));
            accepted(&cli(root, &work, "check", &fixture, &["--no-config"]), name);
            let (exe, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            accepted(&build, name);
            accepted(&Command::new(exe).output().unwrap(), name);
        }
        for (name, code) in [
            ("ambiguous_fn", "E1008"),
            ("ambiguous_reverse", "E1008"),
            ("ambiguous_type", "E1008"),
            ("ambiguous_signature", "E1008"),
            ("ambiguous_macro", "E1008"),
            ("macro_shadow_function", "E1001"),
            ("macro_shadow_type", "E1001"),
            ("macro_shadow_parameter", "E1001"),
            ("macro_shadow_local", "E1001"),
            ("macro_shadow_generic", "E1001"),
            ("macro_shadow_ambiguity", "E1001"),
            ("macro_shadow_loop", "E1001"),
            ("macro_shadow_match", "E1001"),
            ("macro_shadow_lambda", "E1001"),
            ("private_member", "E1003"),
            ("alias_collision", "E1002"),
            ("alias_duplicate", "E1002"),
            ("missing_namespace", "E1001"),
            ("leaf_target", "E1001"),
            ("block_directive", "E0003"),
            ("block_macro_directive", "E0003"),
            ("export_directive", "E0003"),
            ("nonrecursive", "E1001"),
            ("no_target_bootstrap", "E1001"),
            ("no_alias_reexport", "E1001"),
        ] {
            let fixture = fixtures().join("using").join(format!("{name}.ln"));
            let check = cli(root, &work, "check", &fixture, &["--no-config"]);
            rejected(&check, code, name);
            let (_, build) = modes.build(root, &fixture, &format!("{mode}_{name}"));
            rejected(&build, code, name);
            if name.starts_with("ambiguous_") {
                let errors = String::from_utf8_lossy(&build.stderr);
                assert!(
                    errors.contains("note:"),
                    "candidate locations missing: {}",
                    render(&build)
                );
            }
        }
        // Discovery aliases may cross another provider's genuine name. Identity
        // checks and serialized symbol references must never pick an alias key.
        let crossing = work.join("crossing");
        let crossing_providers = crossing.join("providers");
        let crossing_config = crossing.join("luna.toml");
        write(&crossing_config, "[providers]\nsecond = 'providers/first'\nother = 'providers/second'\nbridge = 'providers/bridge'\n");
        for name in ["first", "second", "bridge"] {
            write(
                &crossing_providers.join(format!("{name}.ln")),
                &fs::read_to_string(
                    fixtures()
                        .join("providers")
                        .join(format!("cross_{name}.ln")),
                )
                .unwrap(),
            );
            if mode == "artifact" {
                accepted(
                    &cli(
                        root,
                        &work,
                        "build",
                        &crossing_providers.join(format!("{name}.ln")),
                        &[
                            "--config",
                            crossing_config.to_str().unwrap(),
                            "--emit",
                            "llib",
                            "--lib",
                            "-o",
                            crossing_providers
                                .join(format!("{name}.llib"))
                                .to_str()
                                .unwrap(),
                        ],
                    ),
                    "crossing aliases fresh artifact",
                );
            }
        }
        if mode == "artifact" {
            for name in ["first", "second", "bridge"] {
                fs::remove_file(crossing_providers.join(format!("{name}.ln"))).unwrap();
            }
        }
        for name in ["cross_forward", "cross_reverse", "cross_portable"] {
            let input = crossing.join(format!("{name}.ln"));
            write(
                &input,
                &fs::read_to_string(fixtures().join(format!("{name}.ln"))).unwrap(),
            );
            accepted(&cli(root, &work, "check", &input, &[]), name);
            accepted(&cli(root, &work, "run", &input, &[]), name);
        }
        let app = work.join("app");
        let shared = work.join("shared");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&shared).unwrap();
        for name in ["geometry", "dependent"] {
            fs::copy(
                fixtures().join("providers").join(format!("{name}.ln")),
                shared.join(format!("{name}.ln")),
            )
            .unwrap();
        }
        let config = work.join("luna.toml");
        let config_text = "schema = 1\n[providers]\ngeo = 'shared/geometry'\nother = 'shared/geometry'\ndep = 'shared/dependent'\nunused = 'missing'\n";
        write(&config, config_text);
        // Deliberately invalid neighboring config must never replace invocation config.
        write(&shared.join("luna.toml"), "this is invalid TOML");
        for name in [
            "configured",
            "transitive",
            "dedup",
            "private",
            "old_local",
            "no_autoimport",
            "private_macro_escape",
            "exit_code",
        ] {
            fs::copy(
                fixtures().join(format!("{name}.ln")),
                app.join(format!("{name}.ln")),
            )
            .unwrap();
        }
        if mode == "artifact" {
            for name in ["geometry", "dependent"] {
                accepted(
                    &cli(
                        root,
                        &work,
                        "build",
                        &shared.join(format!("{name}.ln")),
                        &[
                            "--config",
                            config.to_str().unwrap(),
                            "--emit",
                            "llib",
                            "--lib",
                            "-o",
                            shared.join(format!("{name}.llib")).to_str().unwrap(),
                        ],
                    ),
                    "fresh custom provider",
                );
                assert!(shared.join(format!("{name}.llib")).is_file());
            }
            for name in ["geometry", "dependent"] {
                fs::remove_file(shared.join(format!("{name}.ln"))).unwrap();
            }
        }
        for name in ["configured", "transitive", "dedup", "old_local"] {
            let input = app.join(format!("{name}.ln"));
            accepted(
                &cli(root, &std::env::temp_dir(), "check", &input, &[]),
                name,
            );
            accepted(&cli(root, &std::env::temp_dir(), "run", &input, &[]), name);
        }
        rejected(
            &cli(root, &work, "check", &app.join("private.ln"), &[]),
            "E1001",
            "private provider member is not exported into the opened namespace",
        );
        rejected(
            &cli(root, &work, "check", &app.join("no_autoimport.ln"), &[]),
            "E1001",
            "no automatic import",
        );
        rejected(
            &cli(
                root,
                &work,
                "check",
                &app.join("private_macro_escape.ln"),
                &[],
            ),
            "E1001",
            "macro cannot grant access to a private sibling namespace member",
        );
        let input = app.join("configured.ln");
        rejected(
            &cli(root, &work, "check", &input, &["--no-config"]),
            "E1004",
            "disabled config",
        );
        accepted(
            &cli(
                root,
                &work,
                "run",
                &app.join("old_local.ln"),
                &["--no-config"],
            ),
            "legacy relative import",
        );
        write(&app.join("luna.toml"), "[providers]\ngeo = '../missing'\n");
        let missing = cli(root, &work, "check", &input, &[]);
        rejected(&missing, "E1004", "nearest config without merge");
        fs::copy(
            fixtures().join("providers/geometry.ln"),
            shared.join("geo.ln"),
        )
        .unwrap();
        fs::copy(
            fixtures().join("cached_binding.ln"),
            app.join("cached_binding.ln"),
        )
        .unwrap();
        rejected(
            &cli(root, &work, "check", &app.join("cached_binding.ln"), &[]),
            "E1004",
            "configured missing binding overrides an already loaded local provider",
        );
        let diagnostic = String::from_utf8_lossy(&missing.stderr);
        assert!(
            diagnostic.contains("luna.toml") && diagnostic.contains("note:"),
            "config provenance missing: {}",
            render(&missing)
        );
        accepted(
            &cli(
                root,
                &work,
                "run",
                &input,
                &["--config", config.to_str().unwrap()],
            ),
            "explicit overrides nearest",
        );
        fs::remove_file(app.join("luna.toml")).unwrap();
        for invalid in [
            "schema = 2",
            "unknown = 1",
            "[providers]\ngeo = ''",
            "[providers]\ngeo = 'x.ln'",
            "[providers]\n'bad-name' = 'x'",
            "[providers]\ngeo = 'a'\ngeo = 'b'",
            "[providers]\nslice = 'missing'",
            "[providers]\nrand = 'missing'",
        ] {
            write(&config, invalid);
            for command in ["check", "build", "run"] {
                rejected(&cli(root, &work, command, &input, &[]), "E6008", invalid);
            }
        }
        // Cycles cannot evade detection through different discovery aliases.
        fs::copy(
            fixtures().join("providers/cycle.ln"),
            shared.join("cycle.ln"),
        )
        .unwrap();
        fs::copy(
            fixtures().join("cycle_consumer.ln"),
            app.join("cycle_consumer.ln"),
        )
        .unwrap();
        write(
            &config,
            "[providers]\nloop_entry = 'shared/cycle'\nloop_alias = 'shared/cycle'\n",
        );
        rejected(
            &cli(root, &work, "check", &app.join("cycle_consumer.ln"), &[]),
            "E1005",
            "cycle via discovery aliases",
        );
        // Two physical providers sharing a logical identity must reject, never silently merge.
        for dir in ["left", "right"] {
            write(
                &work.join(dir).join("same.ln"),
                "export module different {}\n",
            );
        }
        fs::copy(
            fixtures().join("collision_consumer.ln"),
            app.join("collision_consumer.ln"),
        )
        .unwrap();
        write(
            &config,
            "[providers]\nleft = 'left/same'\nright = 'right/same'\n",
        );
        rejected(
            &cli(
                root,
                &work,
                "check",
                &app.join("collision_consumer.ln"),
                &[],
            ),
            "E1002",
            "distinct provider identity collision",
        );
        for command in ["check", "build", "run"] {
            rejected(
                &cli(
                    root,
                    &work,
                    command,
                    &input,
                    &["--config", work.join("absent.toml").to_str().unwrap()],
                ),
                "E6008",
                "missing explicit config",
            );
        }
        write(
            &config,
            &format!(
                "[providers]\ngeo = '{}'\n",
                shared.join("geometry").to_string_lossy().replace('\\', "/")
            ),
        );
        accepted(
            &cli(root, &work, "run", &input, &[]),
            "absolute provider stem",
        );
        write(&config, config_text);
        // Artifact precedence is strict even when valid source is beside corrupt binary.
        let broken = shared.join("geometry.llib");
        let original = fs::read(&broken).ok();
        write(&broken, "invalid artifact");
        rejected(
            &cli(root, &work, "check", &input, &[]),
            "E6005",
            "corrupt selected artifact must not fall back",
        );
        if let Some(bytes) = original {
            fs::write(&broken, bytes).unwrap();
        } else {
            fs::remove_file(&broken).unwrap();
        }
        if mode == "artifact" {
            let source = fs::read_to_string(fixtures().join("providers/geometry.ln")).unwrap();
            write(
                &shared.join("geometry.ln"),
                &(source + "\n// fingerprint changed\n"),
            );
            rejected(
                &cli(root, &work, "check", &input, &[]),
                "E6001",
                "stale configured artifact rejects without source fallback",
            );
            fs::remove_file(shared.join("geometry.ln")).unwrap();
            fs::rename(shared.join("geometry.llib"), shared.join("renamed.llib")).unwrap();
            write(
                &config,
                "[providers]\ngeo = 'shared/renamed'\ndep = 'shared/dependent'\n",
            );
            accepted(
                &cli(root, &work, "run", &input, &[]),
                "artifact filename does not rewrite provider identity",
            );
            accepted(
                &cli(root, &work, "run", &app.join("transitive.ln"), &[]),
                "renamed artifact retains genuine dependency fingerprints",
            );
            fs::rename(shared.join("renamed.llib"), shared.join("geometry.llib")).unwrap();
            write(&config, config_text);
        }
        let run = cli(root, &work, "run", &app.join("exit_code.ln"), &[]);
        assert_eq!(
            run.status.code(),
            Some(7),
            "run must execute: {}",
            render(&run)
        );
        let conflict = cli(
            root,
            &work,
            "check",
            &input,
            &["--config", config.to_str().unwrap(), "--no-config"],
        );
        assert!(!conflict.status.success());
        // Relocate the complete provider graph, then remove the original to rule out hidden paths.
        let moved = root.join("relocated");
        fs::rename(&work, &moved).unwrap();
        accepted(
            &cli(
                root,
                &std::env::temp_dir(),
                "run",
                &moved.join("app/configured.ln"),
                &[],
            ),
            "relocated provider graph",
        );
        accepted(
            &cli(
                root,
                &std::env::temp_dir(),
                "run",
                &moved.join("app/transitive.ln"),
                &[],
            ),
            "relocated transitive graph",
        );
        eprintln!("Completed alpha module matrix: {mode}");
    }
}

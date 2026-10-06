#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn default_declarations_are_checked_and_preserved_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/default_arguments");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("provider.ln"), &provider).unwrap();
        if mode == "artifact" {
            let output = Command::new(env!("CARGO_BIN_EXE_luna"))
                .arg("build")
                .arg(&provider)
                .args(["--lib", "--emit", "llib", "--quiet", "-o"])
                .arg(provider.with_extension("llib"))
                .env("LUNA_SYSROOT", root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{}", render(&output));
            let (_, _, _, metadata) = luna_llib::MlibReader::read_module(
                &mut std::io::Cursor::new(fs::read(provider.with_extension("llib")).unwrap()),
            )
            .unwrap();
            let signature = metadata.unwrap().interface.exported_symbols["defaults"].children
                ["combine"]
                .callable_signature
                .clone()
                .unwrap();
            assert_eq!(signature.default_contracts.len(), 2);
            assert!(signature.default_contracts[0].is_none());
            let contract = signature.default_contracts[1].as_ref().unwrap();
            assert!(
                contract.contains("helper") && contract.contains("$parameter0"),
                "{contract}"
            );
            fs::remove_file(provider).unwrap();
        }
        let relocated = root.join("relocated");
        fs::rename(project, &relocated).unwrap();
        for (name, code, message) in [
            ("required_after_default", "E2001", "Required parameters"),
            ("receiver_default", "E2001", "Receiver parameter"),
            ("later_parameter", "E1001", "second"),
            ("self_parameter", "E1001", "first"),
            ("rigid_generic", "E2026", "cast"),
            ("wrong_type", "E2001", "Default value"),
            ("unsafe_default", "E2025", "unsafe"),
            ("trait_new_default", "E2001", "trait implementation"),
            ("trait_changed_default", "E2001", "trait implementation"),
            ("trait_wrong_type", "E2001", "Default value"),
        ] {
            let source = relocated.join(format!("{name}.ln"));
            fs::copy(fixtures.join(format!("{name}.ln")), &source).unwrap();
            let check = modes.check(root, &source);
            let (executable, build) = modes.build(root, &source, &format!("{mode}_{name}"));
            assert!(!executable.exists());
            for (command, output) in [("check", check), ("build", build)] {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert_eq!(output.status.code(), Some(1), "{}", render(&output));
                assert!(
                    stderr.contains(&format!("error[{code}]")) && stderr.contains(message),
                    "{}",
                    render(&output)
                );
                assert!(
                    stderr.contains(&format!("{name}.ln:")) && stderr.contains('^'),
                    "{}",
                    render(&output)
                );
                assert!(!stderr.contains("error[E6001]"), "{}", render(&output));
                eprintln!("PASS {mode}/{command}/{name}: {code} {message}");
            }
        }
        let source = relocated.join("explicit_controls.ln");
        fs::copy(fixtures.join("explicit_controls.ln"), &source).unwrap();
        let check = modes.check(root, &source);
        assert!(check.status.success(), "{}", render(&check));
        let (executable, build) = modes.build(root, &source, &format!("{mode}_explicit_controls"));
        assert!(build.status.success(), "{}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/explicit_controls: full arity native exit0; definition-site defaults retained");
    }
    verify_default_interface_identity(&modes, &fixtures);
}

fn publish(
    root: &std::path::Path,
    project: &std::path::Path,
    source: &str,
) -> luna_llib::format::Manifest {
    fs::create_dir_all(project).unwrap();
    let provider = project.join("provider.ln");
    fs::write(&provider, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build")
        .arg(&provider)
        .args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(provider.with_extension("llib"))
        .env("LUNA_SYSROOT", root)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", render(&output));
    let (_, manifest, _, metadata) = luna_llib::MlibReader::read_module(&mut std::io::Cursor::new(
        fs::read(provider.with_extension("llib")).unwrap(),
    ))
    .unwrap();
    let metadata = metadata.unwrap();
    let method = metadata
        .interface
        .impl_headers
        .iter()
        .find(|header| header.trait_id.is_none() && header.method_contracts.contains_key("add"))
        .unwrap();
    let default = method.method_contracts["add"]
        .callable_signature
        .as_ref()
        .unwrap()
        .default_contracts[1]
        .as_ref()
        .unwrap();
    assert!(
        default.contains("$parameter0") && default.contains("Identifier:4:base|"),
        "field name must retain field identity: {default}"
    );
    fs::remove_file(provider).unwrap();
    manifest.unwrap()
}

fn verify_default_interface_identity(modes: &ProviderModes, fixtures: &std::path::Path) {
    let source = fs::read_to_string(fixtures.join("provider.ln")).unwrap();
    let root = &modes.artifact;
    let baseline = publish(root, &root.join("identity_baseline"), &source);
    for (name, changed, should_differ) in [
        (
            "default_changed",
            source.replace(
                "second: i32 = helper(first)",
                "second: i32 = helper(first) + 1",
            ),
            true,
        ),
        (
            "default_format",
            source.replace(
                "second: i32 = helper(first)",
                "second: i32 = helper( /* contract whitespace */ first )",
            ),
            false,
        ),
        (
            "generic_rename",
            source.replace(
                "identity<T>(first: T, second: T = first as T) -> T",
                "identity<U>(first: U, second: U = first as U) -> U",
            ),
            false,
        ),
        (
            "helper_body",
            source.replace("return value + 20", "return value + 21"),
            false,
        ),
        (
            "alias_rename",
            source
                .replace("as selected;", "as chosen;")
                .replace("selected::make", "chosen::make"),
            false,
        ),
        (
            "alias_retarget",
            source.replace("using first as selected;", "using second as selected;"),
            true,
        ),
        (
            "constant_alias_rename",
            source
                .replace("as selected_value;", "as chosen_value;")
                .replace("selected_value::VALUE", "chosen_value::VALUE"),
            false,
        ),
        (
            "constant_alias_retarget",
            source.replace(
                "using first as selected_value",
                "using second as selected_value",
            ),
            true,
        ),
        (
            "generic_helper_binder_rename",
            source.replace(
                "generic_default<T>(first: T, second: T = generic_helper<T>(first)) -> T",
                "generic_default<V>(first: V, second: V = generic_helper<V>(first)) -> V",
            ),
            false,
        ),
        (
            "impl_binder_rename",
            source
                .replace("Cell<T>", "Cell<V>")
                .replace("impl<T> Cell<V>", "impl<V> Cell<V>")
                .replace("value: T };", "value: V };")
                .replace(
                    "value: &T = &self.value as &T",
                    "value: &V = &self.value as &V",
                ),
            false,
        ),
        (
            "trait_binder_rename",
            source.replace("Processor<U>", "Processor<V>").replace(
                "first: U, second: U = first as U) -> U",
                "first: V, second: V = first as V) -> V",
            ),
            false,
        ),
    ] {
        let candidate = publish(root, &root.join(name), &changed);
        assert_eq!(
            baseline.provenance.interface_fingerprint != candidate.provenance.interface_fingerprint,
            should_differ,
            "{name}"
        );
        eprintln!("PASS interface/{name}: default contract and symbol-role identity");
    }
    for candidate in [
        "default_changed",
        "alias_retarget",
        "constant_alias_retarget",
    ] {
        let project = root.join(format!("stale_{candidate}_dependency"));
        fs::create_dir(&project).unwrap();
        copy_provider_bundle(&root.join("identity_baseline"), &project);
        let dependent = project.join("dependent.ln");
        fs::write(&dependent, "import \"provider\"; module dependent { export fn run() -> i32 { return defaults::combine(2, 9); } }").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_luna"))
            .arg("build")
            .arg(&dependent)
            .args(["--lib", "--emit", "llib", "--quiet", "-o"])
            .arg(dependent.with_extension("llib"))
            .env("LUNA_SYSROOT", root)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", render(&output));
        fs::remove_file(dependent).unwrap();
        let consumer = project.join("main.ln");
        fs::write(
            &consumer,
            "import \"dependent\"; fn main() -> i32 { return dependent::run() - 11; }",
        )
        .unwrap();
        let (executable, build) =
            modes.build(root, &consumer, &format!("{candidate}_dependency_fresh"));
        assert!(build.status.success(), "{}", render(&build));
        assert_eq!(
            Command::new(executable).output().unwrap().status.code(),
            Some(0)
        );
        copy_provider_bundle(&root.join(candidate), &project);
        let check = modes.check(root, &consumer);
        let (executable, build) =
            modes.build(root, &consumer, &format!("{candidate}_dependency_stale"));
        assert!(!executable.exists());
        for output in [check, build] {
            assert_eq!(output.status.code(), Some(1), "{}", render(&output));
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("dependency interface fingerprint mismatch"),
                "{}",
                render(&output)
            );
        }
        eprintln!("PASS {candidate}-dependent freshness: fresh native control, check/build reject changed default contract");
    }
}

fn copy_provider_bundle(source: &std::path::Path, destination: &std::path::Path) {
    // Replace the entire owned bundle to avoid stale sidecars selecting another verdict.
    for entry in fs::read_dir(destination).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file()
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("provider.")
        {
            fs::remove_file(path).unwrap();
        }
    }
    for entry in fs::read_dir(source).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file()
            && path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("provider.")
        {
            fs::copy(&path, destination.join(path.file_name().unwrap())).unwrap();
        }
    }
}

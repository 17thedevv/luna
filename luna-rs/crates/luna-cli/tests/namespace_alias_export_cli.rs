#[path = "support/stdlib.rs"]
mod support;
use std::{fs, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn namespace_alias_preserves_exported_owner_and_stays_local_in_both_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root()
        .parent()
        .unwrap()
        .join("tests/luna/language/alpha_modules");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        let project = root.join("project");
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::copy(fixtures.join("providers/alias_scope_owner.ln"), &provider).unwrap();
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
            let metadata = metadata.unwrap();
            let exports = &metadata.interface.exported_symbols;
            for name in ["first", "second", "facade"] {
                assert!(
                    exports.contains_key(name),
                    "namespace {name} disappeared from metadata"
                );
                assert_eq!(exports[name].symbol_id.symbol_path, name);
            }
            assert!(
                !exports.contains_key("selected"),
                "private namespace alias leaked into public metadata"
            );
            fs::remove_file(provider).unwrap();
        }
        let relocated = root.join("relocated");
        fs::rename(project, &relocated).unwrap();
        let source = relocated.join("main.ln");
        fs::copy(fixtures.join("using/alias_owner_consumer.ln"), &source).unwrap();
        let check = modes.check(root, &source);
        assert!(check.status.success(), "{}", render(&check));
        let (executable, build) = modes.build(root, &source, &format!("{mode}_alias_owner"));
        assert!(build.status.success(), "{}", render(&build));
        let run = Command::new(executable).output().unwrap();
        assert_eq!(run.status.code(), Some(0), "{}", render(&run));
        eprintln!("PASS {mode}/owner: original namespaces and private alias body; native exit0");
        fs::copy(fixtures.join("using/alias_owner_not_exported.ln"), &source).unwrap();
        let check = modes.check(root, &source);
        let (executable, build) = modes.build(root, &source, &format!("{mode}_alias_leak"));
        assert!(!executable.exists());
        for (command, output) in [("check", check), ("build", build)] {
            assert_eq!(output.status.code(), Some(1), "{}", render(&output));
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("error[E1001]"),
                "{}",
                render(&output)
            );
            eprintln!("PASS {mode}/{command}/local_alias: E1001");
        }
    }
}

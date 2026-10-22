#[path = "support/stdlib.rs"]
mod support;
use std::{fs, io::Cursor, path::Path, process::Command};
use support::{render, workspace_root, ProviderModes};

#[test]
fn target_sized_memory_and_collection_api_in_both_provider_modes() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/stdlib/size_index");
    for (mode, root) in [("source", &modes.source), ("artifact", &modes.artifact)] {
        for name in ["memory", "projected_temporary", "collections", "set_algebra", "bit_operations"] {
            let input = fixtures.join(format!("{name}.ln"));
            let check = modes.check(root, &input);
            assert!(check.status.success(), "{mode}/{name}: {}", render(&check));
            let (exe, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            assert!(build.status.success(), "{mode}/{name}: {}", render(&build));
            let run = Command::new(exe).output().unwrap();
            assert!(run.status.success(), "{mode}/{name}: {}", render(&run));
            eprintln!("PASS {mode}/{name}: check/build/native");
        }
        for (name, code) in [("reject_u64", "E2001"), ("reject_slice_u64", "E2001"), ("reject_length_u64", "E2001"), ("reject_set_clone", "E2021")] {
            let input = fixtures.join(format!("{name}.ln"));
            let (exe, build) = modes.build(root, &input, &format!("{name}_{mode}"));
            assert!(!exe.exists());
            for result in [modes.check(root, &input), build] {
                assert!(!result.status.success(), "accepted {mode}/{name}");
                assert!(String::from_utf8_lossy(&result.stderr).contains(&format!("error[{code}]")),
                    "{mode}/{name}: {}", render(&result));
            }
            eprintln!("PASS {mode}/{name}: {code} check/build");
        }
    }
    artifact_count_type_identity(&modes);
}

fn publish(root: &Path, dir: &Path, name: &str, source: &str) -> luna_llib::format::InterfaceFingerprint {
    let input = dir.join(format!("{name}.ln"));
    let artifact = input.with_extension("llib");
    fs::write(&input, source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build").arg(&input).args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(&artifact).arg("-I").arg(root).env("LUNA_SYSROOT", root).output().unwrap();
    assert!(result.status.success(), "{}", render(&result));
    fs::remove_file(input).unwrap();
    luna_llib::LlibReader::read_manifest(&mut Cursor::new(fs::read(artifact).unwrap()))
        .unwrap().provenance.interface_fingerprint
}

fn artifact_count_type_identity(modes: &ProviderModes) {
    let dir = modes.artifact.join("count_identity");
    fs::create_dir(&dir).unwrap();
    let first = publish(&modes.artifact, &dir, "a", "module a { export fn count() -> u64 { return 1 as u64; } }");
    publish(&modes.artifact, &dir, "b", "import \"a\"; module b { export fn count() -> i32 { return a::count() as i32; } }");
    let main = dir.join("main.ln");
    fs::write(&main, "import \"b\"; fn main() -> i32 { return b::count() - 2; }").unwrap();
    // Ordinary native calls relink after a body-only change, with unchanged ABI.
    let same = publish(&modes.artifact, &dir, "a", "module a { export fn count() -> u64 { return 2 as u64; } }");
    assert_eq!(first, same);
    let (exe, build) = modes.build(&modes.artifact, &main, "same_count_type");
    assert!(build.status.success(), "{}", render(&build));
    assert!(Command::new(exe).status().unwrap().success());
    let changed = publish(&modes.artifact, &dir, "a", "module a { export fn count() -> usize { return 2 as usize; } }");
    assert_ne!(first, changed, "same-width u64/usize lost interface identity");
    let (exe, build) = modes.build(&modes.artifact, &main, "stale_count_type");
    assert!(!exe.exists() && !build.status.success());
    assert!(String::from_utf8_lossy(&build.stderr).contains("dependency interface fingerprint mismatch for provider a"), "{}", render(&build));
    // Coherent rebuild accepts the changed public signature.
    publish(&modes.artifact, &dir, "b", "import \"a\"; module b { export fn count() -> i32 { return a::count() as i32; } }");
    let (exe, build) = modes.build(&modes.artifact, &main, "rebuilt_count_type");
    assert!(build.status.success(), "{}", render(&build));
    assert!(Command::new(exe).status().unwrap().success());
    let artifact = dir.join("a.llib");
    let bytes = fs::read(&artifact).unwrap();
    let mut header = luna_llib::LlibHeader::read_from(&mut Cursor::new(&bytes)).unwrap();
    header.compiler_version = 25;
    let mut old = Cursor::new(bytes);
    header.write_to(&mut old).unwrap();
    fs::write(artifact, old.into_inner()).unwrap();
    let (_, build) = modes.build(&modes.artifact, &main, "old_count_protocol");
    assert!(!build.status.success());
    let error = String::from_utf8_lossy(&build.stderr);
    assert!(error.contains("VersionMismatch") && error.contains("component: \"compiler\"") && error.contains("found: 25"), "{}", render(&build));
    eprintln!("PASS interface type identity, stale rejection, coherent rebuild and protocol-25 rejection");
}

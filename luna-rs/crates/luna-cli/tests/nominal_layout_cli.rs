#[path = "support/stdlib.rs"]
mod support;

use luna_llib::{metadata::{CanonicalNominalMembers, SemanticMetadata}, MlibReader};
use std::{fs, path::Path, process::Command};
use support::{render, workspace_root, ProviderModes};

fn publish(root: &Path, source: &Path) -> (luna_llib::format::Manifest, SemanticMetadata) {
    let output = Command::new(env!("CARGO_BIN_EXE_luna"))
        .arg("build").arg(source).args(["--lib", "--emit", "llib", "--quiet", "-o"])
        .arg(source.with_extension("llib")).env("LUNA_SYSROOT", root).output().unwrap();
    assert!(output.status.success(), "{}: {}", source.display(), render(&output));
    let bytes = fs::read(source.with_extension("llib")).unwrap();
    let (_, manifest, _, metadata) = MlibReader::read_module(&mut std::io::Cursor::new(bytes)).unwrap();
    fs::remove_file(source).unwrap();
    (manifest.unwrap(), metadata.unwrap())
}

fn copy_bundle(from: &Path, to: &Path) {
    for extension in ["llib", "obj"] {
        fs::copy(from.join(format!("provider.{extension}")), to.join(format!("provider.{extension}"))).unwrap();
    }
}

fn run(modes: &ProviderModes, root: &Path, main: &Path, label: &str) {
    let (executable, build) = modes.build(root, main, label);
    assert!(build.status.success(), "{label}: {}", render(&build));
    let result = Command::new(executable).current_dir(std::env::temp_dir()).output().unwrap();
    assert!(result.status.success(), "{label}: {}", render(&result));
    eprintln!("PASS native/{label}: exit0");
}

#[test]
fn ordered_nominal_abi_invalidates_stale_dependents_including_reachable_private_types() {
    let modes = ProviderModes::fresh();
    let fixtures = workspace_root().parent().unwrap().join("tests/luna/language/nominal_layout");
    for (case, before, after, expected) in [
        ("struct", "a: i32, b: u64", "b: u64, a: i32", 7),
        ("enum", "Red, Blue", "Blue, Red", 7),
        ("private", "u64", "u8", 16),
    ] {
        let provider_text = fs::read_to_string(fixtures.join(format!("{case}_provider.ln"))).unwrap();
        let wrapper_text = fs::read_to_string(fixtures.join(format!("{case}_wrapper.ln"))).unwrap();
        let source_project = modes.source.join(case);
        fs::create_dir(&source_project).unwrap();
        fs::write(source_project.join("provider.ln"), &provider_text).unwrap();
        fs::write(source_project.join("wrapper.ln"), &wrapper_text).unwrap();
        let main_text = format!("import \"wrapper\"; fn main() -> i32 {{ return client::read() - {expected}; }}");
        fs::write(source_project.join("main.ln"), &main_text).unwrap();
        run(&modes, &modes.source, &source_project.join("main.ln"), &format!("{case}_source"));

        let project = modes.artifact.join(case);
        fs::create_dir(&project).unwrap();
        let provider = project.join("provider.ln");
        fs::write(&provider, &provider_text).unwrap();
        let (baseline, metadata) = publish(&modes.artifact, &provider);
        let layout = metadata.interface.nominal_layouts.iter().find(|(id, _)|
            id.symbol_path.ends_with(if case == "enum" { "Color" } else { "Record" })).unwrap().1;
        match (&layout.members, case) {
            (CanonicalNominalMembers::Enum(variants), "enum") => assert_eq!(variants.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(), ["Red", "Blue"]),
            (CanonicalNominalMembers::Struct(fields), _) => assert_eq!(fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["a", "b"]),
            _ => panic!("wrong nominal kind"),
        }
        if case == "private" {
            assert!(!metadata.interface.exported_symbols.contains_key("Record"), "ABI reachability must not expose private names");
        }
        let decoded = luna_driver::metadata_decoder::InterfaceDecoder::new(
            luna_driver::registry::ModuleRegistry::default().allocate_id(), "provider".into(),
            metadata.clone(), Default::default(), baseline.provenance.interface_fingerprint,
        ).decode().unwrap();
        if case == "enum" {
            let variants = &decoded.exported_symbols["api"].children["Color"].children;
            assert_eq!(format!("{:?}", variants["Red"].sym.kind), "EnumVariant(0)");
            assert_eq!(format!("{:?}", variants["Blue"].sym.kind), "EnumVariant(1)");
        } else {
            assert!(decoded.symbol_struct_field_names.values().any(|names| names == &["a", "b"]));
        }
        let mut corrupt = metadata.interface.clone();
        let layout = corrupt.nominal_layouts.values_mut().next().unwrap();
        match &mut layout.members {
            CanonicalNominalMembers::Struct(fields) => fields[0].ty = u32::MAX,
            CanonicalNominalMembers::Enum(variants) => variants[0].payload = u32::MAX,
        }
        assert!(luna_llib::reader::validate_nominal_layouts(&corrupt, "provider").is_err());
        let mut corrupt = metadata.interface.clone();
        let layout = corrupt.nominal_layouts.values_mut().next().unwrap();
        match &mut layout.members {
            CanonicalNominalMembers::Struct(fields) => fields[1].name = fields[0].name.clone(),
            CanonicalNominalMembers::Enum(variants) => variants[1].name = variants[0].name.clone(),
        }
        assert!(luna_llib::reader::validate_nominal_layouts(&corrupt, "provider").is_err());
        let mut corrupt = metadata.interface.clone();
        corrupt.nominal_layouts.clear();
        assert!(luna_llib::reader::validate_nominal_layouts(&corrupt, "provider").is_err());

        fs::write(project.join("wrapper.ln"), &wrapper_text).unwrap();
        publish(&modes.artifact, &project.join("wrapper.ln"));
        fs::write(project.join("main.ln"), &main_text).unwrap();
        let relocated = modes.artifact.join(format!("relocated_{case}"));
        fs::rename(&project, &relocated).unwrap();
        assert!(!relocated.join("provider.ln").exists() && !relocated.join("wrapper.ln").exists());
        run(&modes, &modes.artifact, &relocated.join("main.ln"), &format!("{case}_artifact"));

        let changed = modes.artifact.join(format!("changed_{case}"));
        fs::create_dir(&changed).unwrap();
        let changed_text = provider_text.replace(before, after);
        assert_ne!(provider_text, changed_text);
        fs::write(changed.join("provider.ln"), changed_text).unwrap();
        let (current, _) = publish(&modes.artifact, &changed.join("provider.ln"));
        assert_ne!(baseline.provenance.interface_fingerprint, current.provenance.interface_fingerprint, "{case} ABI changed");
        copy_bundle(&changed, &relocated);
        let check = modes.check(&modes.artifact, &relocated.join("main.ln"));
        let (executable, build) = modes.build(&modes.artifact, &relocated.join("main.ln"), &format!("{case}_stale"));
        for output in [&check, &build] {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success() && stderr.contains("error[E6001]") && stderr.contains("interface"), "{case} stale: {}", render(output));
        }
        assert!(!executable.exists());
        eprintln!("PASS stale/{case}: typed check/build rejection, matching sidecar, no source fallback");

        fs::write(relocated.join("wrapper.ln"), &wrapper_text).unwrap();
        publish(&modes.artifact, &relocated.join("wrapper.ln"));
        if case == "private" {
            fs::write(relocated.join("main.ln"), "import \"wrapper\"; fn main() -> i32 { return client::read() - 8; }").unwrap();
        }
        run(&modes, &modes.artifact, &relocated.join("main.ln"), &format!("{case}_rebuilt"));
    }

    let recursive = fs::read_to_string(fixtures.join("recursive_provider.ln")).unwrap();
    let mut previous = None;
    for (label, text) in [
        ("baseline", recursive.clone()),
        ("binder_rename", recursive.replace("<T>", "<Element>").replace("value: T", "value: Element")),
        ("private_unreachable", recursive.replace("field: i32", "other: u64, extra: i32")),
        ("body_only", recursive.replace("return value.marker;", "return value.marker + 0u8;")),
    ] {
        let dir = modes.artifact.join(format!("recursive_{label}"));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("provider.ln"), text).unwrap();
        let (manifest, metadata) = publish(&modes.artifact, &dir.join("provider.ln"));
        assert_eq!(metadata.interface.nominal_layouts.len(), 2, "only Visible and reachable Hidden");
        if let Some(previous) = previous { assert_eq!(previous, manifest.provenance.interface_fingerprint, "{label} unchanged ABI"); }
        previous = Some(manifest.provenance.interface_fingerprint);
        eprintln!("PASS identity/{label}: recursive reachability and ABI controls");
    }
}

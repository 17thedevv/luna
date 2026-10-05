//! Isolated roots for existing driver parity regressions. New language/stdlib
//! acceptance should use CLI fixtures; this only repairs shared-state legacy tests.
use luna_driver::{compile, sysroot::Sysroot, CompilerOptions};
use std::{fs, path::Path};

pub struct ProviderPair {
    pub artifact: Sysroot,
    pub source: Sysroot,
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create isolated provider directory");
    for entry in fs::read_dir(from).expect("enumerate provider directory") {
        let entry = entry.expect("read provider entry");
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') { continue; }
        let source = entry.path();
        let destination = to.join(name);
        if source.is_dir() { copy_tree(&source, &destination); }
        else { fs::copy(source, destination).expect("copy provider file"); }
    }
}

impl ProviderPair {
    pub fn new(work: &Path, relative_source: &str) -> Self {
        let canonical = Sysroot::discover_for_test().expect("canonical test sysroot");
        let artifact_root = work.join("artifact_sysroot");
        copy_tree(canonical.external_dir(), &artifact_root.join("libs/external"));
        let artifact = Sysroot::from_root(artifact_root).expect("artifact test sysroot");
        let provider = artifact.external_dir().join(relative_source);
        let text = fs::read_to_string(&provider).expect("read isolated provider");
        let options = CompilerOptions {
            output_path: Some(provider.with_extension("llib").to_string_lossy().into_owned()),
            emit_llib: true, no_link: true, quiet: true, is_sysroot_build: true,
            search_paths: vec![artifact.root().to_string_lossy().into_owned()],
            ..Default::default()
        };
        let result = compile(provider.to_str().unwrap(), text, &options);
        assert!(result.is_ok(), "isolated provider publication: {result:?}");
        let source_root = work.join("source_sysroot");
        copy_tree(artifact.external_dir(), &source_root.join("libs/external"));
        let source = Sysroot::from_root(source_root).expect("source test sysroot");
        let source_provider = source.external_dir().join(relative_source);
        fs::remove_file(source_provider.with_extension("llib")).expect("force provider source route");
        fs::remove_file(source_provider.with_extension("obj")).expect("remove source sidecar");
        fs::remove_file(&provider).expect("force provider artifact route");
        assert!(source_provider.exists() && !source_provider.with_extension("llib").exists());
        assert!(!provider.exists() && provider.with_extension("llib").exists() && provider.with_extension("obj").exists());
        Self { artifact, source }
    }
}

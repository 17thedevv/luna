use luna_driver::sysroot_manifest::SysrootManifest;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Check provider paths, not just a historical count: an extra orphan must not
/// compensate for a missing canonical artifact.
pub fn assert_inventory(external: &Path, excluded: &[&str]) {
    let manifest = SysrootManifest::load_and_validate(&external.join("sysroot.toml"))
        .expect("valid copied canonical manifest");
    for extension in ["llib", "obj"] {
        let expected: BTreeSet<PathBuf> = manifest
            .providers()
            .filter(|entry| !excluded.contains(&entry.path.as_str()))
            .map(|entry| PathBuf::from(&entry.path).with_extension(extension))
            .collect();
        let mut actual = BTreeSet::new();
        collect_paths(external, external, extension, &mut actual);
        assert_eq!(actual, expected, "canonical {extension} inventory mismatch");
    }
}

fn collect_paths(root: &Path, dir: &Path, extension: &str, paths: &mut BTreeSet<PathBuf>) {
    for entry in fs::read_dir(dir).expect("enumerate isolated sysroot") {
        let path = entry.expect("read isolated sysroot entry").path();
        if path.is_dir() {
            collect_paths(root, &path, extension, paths);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            paths.insert(path.strip_prefix(root).expect("artifact inside sysroot").to_path_buf());
        }
    }
}

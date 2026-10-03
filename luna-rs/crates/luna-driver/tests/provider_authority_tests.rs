use luna_driver::discovery::{ComponentProvenance, ExternalComponentDiscovery};
use luna_driver::resolution_context::ProviderResolutionContext;
use luna_driver::sysroot::Sysroot;
use luna_driver::sysroot_manifest::ProviderCapability;
use std::fs;

#[test]
fn discovery_never_grants_authority_and_sysroot_authenticates_actual_files() {
    let work = std::env::temp_dir().join(format!(
        "luna_authority_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let canonical = work.join("sysroot/libs/external");
    let outside = work.join("outside");
    fs::create_dir_all(canonical.join("implementation")).unwrap();
    fs::create_dir_all(outside.join("implementation")).unwrap();
    fs::write(canonical.join("sysroot.toml"),
        "[[provider]]\nname = \"custom_name\"\npath = \"implementation/provider\"\nvisibility = \"public\"\ncapabilities = [\"slice_inherent_impl\"]\n").unwrap();
    let sysroot = Sysroot::from_root(work.join("sysroot")).unwrap();
    for extension in ["ln", "llib", "ms", "mlib"] {
        let trusted_file = canonical.join(format!("implementation/provider.{extension}"));
        let copied_file = outside.join(format!("implementation/provider.{extension}"));
        fs::write(&trusted_file, b"candidate").unwrap();
        fs::write(&copied_file, b"candidate").unwrap();
        let discovered = ExternalComponentDiscovery::discover(
            &canonical,
            "custom_name",
            sysroot.manifest(),
            ProviderResolutionContext::UserImport,
        )
        .unwrap();
        assert_eq!(discovered.provenance, ComponentProvenance::LocalProject);
        let trusted = sysroot
            .discover_provider(
                &canonical,
                "custom_name",
                ProviderResolutionContext::UserImport,
            )
            .unwrap();
        assert_eq!(
            trusted.provenance,
            ComponentProvenance::SysrootCanonical {
                capabilities: vec![ProviderCapability::SliceInherentImpl],
            }
        );
        let copied = sysroot
            .discover_provider(
                &outside,
                "custom_name",
                ProviderResolutionContext::UserImport,
            )
            .unwrap();
        assert_eq!(copied.provenance, ComponentProvenance::LocalProject);
        assert!(
            sysroot
                .get_canonical_provider_capabilities(&copied_file)
                .is_none()
        );
        fs::remove_file(trusted_file).unwrap();
        fs::remove_file(copied_file).unwrap();
    }
    #[cfg(unix)]
    {
        // A manifest entry resolving through a symlink out of the root is untrusted.
        let target = outside.join("symlink_target.ln");
        fs::write(&target, b"candidate").unwrap();
        std::os::unix::fs::symlink(&target, canonical.join("implementation/provider.ln")).unwrap();
        let descriptor = sysroot
            .discover_provider(
                &canonical,
                "custom_name",
                ProviderResolutionContext::UserImport,
            )
            .unwrap();
        assert_eq!(descriptor.provenance, ComponentProvenance::LocalProject);
    }
    // Delete only the unique test directory allocated above.
    assert_eq!(work.parent(), Some(std::env::temp_dir().as_path()));
    fs::remove_dir_all(work).unwrap();
}

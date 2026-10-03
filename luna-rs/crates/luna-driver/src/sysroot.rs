use std::path::{Path, PathBuf};
use crate::error::SysrootError;
use std::sync::Arc;
use crate::sysroot_manifest::SysrootManifest;

#[derive(Debug, Clone)]
pub struct Sysroot {
    root: PathBuf,
    external_dir: PathBuf,
    manifest: Arc<SysrootManifest>,
}

impl Sysroot {
    pub fn from_root(root: PathBuf) -> Result<Self, SysrootError> {
        let external_dir = if root.join("libs").join("external").exists() {
            root.join("libs").join("external")
        } else if root.ends_with("external") {
            root.clone()
        } else {
            root.join("libs").join("external")
        };
        
        let manifest_path = external_dir.join("sysroot.toml");
        let manifest = SysrootManifest::load_and_validate(&manifest_path).map_err(|e| {
            SysrootError::ManifestLoadFailed {
                path: manifest_path,
                error: e,
            }
        })?;

        Ok(Self {
            root,
            external_dir,
            manifest: Arc::new(manifest),
        })
    }

    pub fn new(root: PathBuf, external_dir: PathBuf, manifest: Arc<SysrootManifest>) -> Self {
        Self { root, external_dir, manifest }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &SysrootManifest {
        &self.manifest
    }

    pub fn external_dir(&self) -> &Path {
        &self.external_dir
    }

    /// Checks if a file corresponds to a canonical sysroot provider entry
    /// and returns its declared capabilities if so.
    pub fn get_canonical_provider_capabilities(
        &self,
        file_path: &Path,
    ) -> Option<Vec<crate::sysroot_manifest::ProviderCapability>> {
        let manifest = self.manifest();
        let canon_file = std::fs::canonicalize(file_path).ok()?;
        let canon_root = std::fs::canonicalize(self.external_dir()).ok()?;
        // A provider entry or directory symlink cannot confer authority on a
        // file outside the selected sysroot.
        if !canon_file.starts_with(&canon_root) {
            return None;
        }
        for entry in manifest.providers() {
            let stem = self.external_dir().join(&entry.path);
            for candidate in [
                self.external_dir().join(format!("{}.ln", entry.path)),
                self.external_dir().join(format!("{}.llib", entry.path)),
                self.external_dir().join(format!("{}.ms", entry.path)),
                self.external_dir().join(format!("{}.mlib", entry.path)),
                stem.join("package.ln"),
                stem.join("package.ms"),
            ] {
                if let Ok(canon_entry) = std::fs::canonicalize(candidate) {
                    if canon_file == canon_entry {
                        return Some(entry.capabilities.clone());
                    }
                }
            }
        }
        None
    }

    /// Discover a candidate and authenticate it against this selected sysroot.
    /// Search-path copies retain ordinary project provenance, even when they
    /// carry a sysroot provider name or a valid sysroot artifact.
    pub fn discover_provider(
        &self,
        search_dir: &Path,
        name: &str,
        context: crate::resolution_context::ProviderResolutionContext,
    ) -> Result<crate::discovery::ExternalComponentDescriptor, crate::error::ExternalComponentError> {
        let mut descriptor = crate::discovery::ExternalComponentDiscovery::discover(
            search_dir, name, self.manifest(), context,
        )?;
        if let Some(capabilities) = self.get_canonical_provider_capabilities(&descriptor.entry_file) {
            descriptor.provenance = crate::discovery::ComponentProvenance::SysrootCanonical {
                capabilities,
            };
        }
        Ok(descriptor)
    }

    pub fn validate(&self) -> Result<(), SysrootError> {
        if !self.external_dir.exists() {
            return Err(SysrootError::ExternalRootMissing(self.external_dir.clone()));
        }
        Ok(())
    }

    /// Production sysroot discovery:
    /// 1. Explicit argument (`--sysroot`)
    /// 2. `LUNA_SYSROOT` environment variable
    /// 3. Executable-relative path (`current_exe()` ancestor containing `libs/external`)
    pub fn discover(explicit: Option<&str>) -> Result<Self, SysrootError> {
        let mut searched_candidates = Vec::new();

        // 1. Explicit option
        if let Some(exp) = explicit {
            let root = PathBuf::from(exp);
            if !root.exists() {
                return Err(SysrootError::SpecifiedPathNotFound(root));
            }
            return Self::from_root(root);
        }

        // 2. Environment variable
        if let Ok(env_val) = std::env::var("LUNA_SYSROOT") {
            if !env_val.trim().is_empty() {
                let root = PathBuf::from(env_val.trim());
                if !root.exists() {
                    return Err(SysrootError::EnvVarPathNotFound(root));
                }
                return Self::from_root(root);
            }
        }

        // 3. Executable-relative discovery
        if let Ok(mut exe) = std::env::current_exe() {
            // <dir>/exe -> <dir>
            exe.pop();
            searched_candidates.push(exe.clone());
            if exe.join("libs").join("external").exists() {
                return Self::from_root(exe);
            }

            // e.g., <install_root>/bin -> <install_root>
            if exe.pop() {
                searched_candidates.push(exe.clone());
                if exe.join("libs").join("external").exists() {
                    return Self::from_root(exe);
                }

                // e.g., <repo_root>/target/debug -> <repo_root>
                if exe.pop() {
                    searched_candidates.push(exe.clone());
                    if exe.join("libs").join("external").exists() {
                        return Self::from_root(exe);
                    }
                }
            }
        }

        Err(SysrootError::DiscoveryFailed {
            searched: searched_candidates,
        })
    }

    /// Test/dev helper: tries production discovery first, then falls back to CWD ancestor walking
    pub fn discover_for_test() -> Result<Self, SysrootError> {
        if let Ok(sysroot) = Self::discover(None) {
            if sysroot.validate().is_ok() {
                return Ok(sysroot);
            }
        }

        if let Ok(mut cwd) = std::env::current_dir() {
            for _ in 0..6 {
                let ext = cwd.join("libs").join("external");
                if ext.exists() {
                    if let Ok(sysroot) = Self::from_root(cwd.clone()) {
                        return Ok(sysroot);
                    }
                }
                if !cwd.pop() {
                    break;
                }
            }
        }

        Self::discover(None)
    }
}

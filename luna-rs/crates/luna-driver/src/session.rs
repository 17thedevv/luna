use std::path::PathBuf;
use luna_ast::AstArena;
use luna_common::CompilerSession;
use luna_semantic::symbol::ProviderId;
use crate::sysroot::Sysroot;
use crate::registry::ModuleRegistry;
use crate::error::{BootstrapError, ExternalComponentError};
use crate::resolution_context::ProviderResolutionContext;
use crate::external::ExternalComponentLoader;

pub struct DriverSession<'a> {
    pub sysroot: Sysroot,
    pub registry: ModuleRegistry,
    pub compiler_session: &'a mut CompilerSession,
    pub search_paths: Vec<PathBuf>,
    pub collected_objects: Vec<PathBuf>,
    pub provider_bindings: crate::provider_binding::ProviderBindings,
    pub provider_paths: std::collections::HashMap<PathBuf, ProviderId>,
    pub loading_paths: std::collections::HashSet<PathBuf>,
}

impl<'a> DriverSession<'a> {
    pub fn new(
        sysroot: Sysroot,
        compiler_session: &'a mut CompilerSession,
        search_paths: &[String],
    ) -> Self {
        Self {
            sysroot,
            registry: ModuleRegistry::new(),
            compiler_session,
            search_paths: search_paths.iter().map(PathBuf::from).collect(),
            collected_objects: Vec::new(),
            provider_bindings: Default::default(),
            provider_paths: Default::default(),
            loading_paths: Default::default(),
        }
    }

    pub fn add_collected_object(&mut self, path: PathBuf) {
        if !self.collected_objects.contains(&path) {
            self.collected_objects.push(path);
        }
    }

    pub fn set_provider_bindings(&mut self, bindings: &crate::provider_binding::ProviderBindings) -> Result<(), Vec<luna_common::Diagnostic>> {
        for (name, binding) in bindings {
            let reason = if !crate::provider_binding::valid_provider_name(name) {
                Some(format!("Invalid configured provider name '{}'", name))
            } else if self.sysroot.manifest().find_provider(name).is_some() {
                Some(format!("Configured provider '{}' collides with a canonical sysroot name or alias", name))
            } else { None };
            if let Some(reason) = reason {
                let mut diagnostic = luna_common::Diagnostic::error(reason)
                    .with_code(luna_common::DiagnosticCode::ProviderConfigurationError);
                if let Some(origin) = &binding.origin {
                    let file = self.compiler_session.source_manager.add_file(origin.file.display().to_string(), origin.source.clone());
                    diagnostic = diagnostic.with_span(luna_common::Span::new(file, origin.key_range.start as u32, origin.key_range.end as u32));
                }
                return Err(vec![diagnostic]);
            }
        }
        self.provider_bindings = bindings.clone();
        Ok(())
    }

    /// Bootstraps required external language contracts.
    pub fn bootstrap_lang_contracts(
        &mut self,
        arena: &mut AstArena,
        
    ) -> Result<(), BootstrapError> {
        self.sysroot.validate().map_err(BootstrapError::Sysroot)?;

        let manifest = crate::lang_contracts::LangContractManifest::canonical();

        // Load all contracts in deterministic order.
        // If they depend on prerequisites (like core/panic), `load_package` will resolve them using standard machinery!
        for entry in manifest.contracts {
            let provider_id = match self.load_package(entry.provider_id, arena, ProviderResolutionContext::Bootstrap) {
                Ok(id) => id,
                Err(err) => {
                    for d in err.into_diagnostics() {
                        eprintln!("{:?}", d);
                    }
                    return Err(BootstrapError::MissingRequiredComponent(entry.provider_id.to_string()));
                }
            };
            
            // Note: controlled auto-visibility will be implemented in registry/importer side.
            // For now, we just ensure they are loaded and marked as external providers.
            self.registry.external_providers.insert(entry.provider_id.to_string());
        }

        Ok(())
    }

    /// Resolves an external package component (e.g. for `import <pkg>;`).
    /// Guarantees load-once semantics: returns existing ProviderId if already loaded.
    pub fn load_package(
        &mut self,
        name: &str,
        arena: &mut AstArena,
        context: ProviderResolutionContext,
    ) -> Result<ProviderId, ExternalComponentError> {
        if let Some(binding) = self.provider_bindings.get(name).cloned() {
            let result = crate::discovery::ExternalComponentDiscovery::discover_file(&binding.stem, name)
                .and_then(|descriptor| ExternalComponentLoader::load_component(&descriptor, arena, self));
            return result.map_err(|error| {
                let mut diagnostics = error.into_diagnostics();
                if let Some(origin) = &binding.origin {
                    let file = self.compiler_session.source_manager.add_file(origin.file.display().to_string(), origin.source.clone());
                    let span = luna_common::Span::new(file, origin.key_range.start as u32, origin.key_range.end as u32);
                    for diagnostic in &mut diagnostics {
                        diagnostic.related.push(luna_common::diagnostic::DiagnosticLabel {
                            span, message: format!("Provider '{}' is bound here to '{}'", name, binding.stem.display()),
                        });
                    }
                }
                ExternalComponentError::ImportFailed(diagnostics)
            });
        }
        if let Some(provider) = self.sysroot.manifest().find_provider(name) {
            if provider.visibility == crate::sysroot_manifest::ProviderVisibility::Internal
                && !context.can_access_internal()
            {
                return Err(ExternalComponentError::NotFound {
                    name: name.to_string(),
                    searched_dir: self.sysroot.external_dir().to_path_buf(),
                });
            }
        }

        if let Some(&id) = self.registry.providers.get(name) {
            return Ok(id);
        }

        let descriptor = {
            let sysroot_manifest = match self.sysroot.validate() {
                Ok(_) => Some(self.sysroot.manifest()),
                Err(_) => None,
            };

            let mut desc = None;
            for path in &self.search_paths {
                if sysroot_manifest.is_some() {
                    if let Ok(d) = self.sysroot.discover_provider(path, name, context) {
                        desc = Some(d);
                        break;
                    }
                }
            }
            if desc.is_none() && sysroot_manifest.is_some() {
                desc = self.sysroot.discover_provider(self.sysroot.external_dir(), name, context).ok();
            }
            desc
        };

        let descriptor = descriptor.ok_or_else(|| ExternalComponentError::NotFound {
            name: name.to_string(),
            searched_dir: self.sysroot.external_dir().to_path_buf(),
        })?;

        ExternalComponentLoader::load_component(&descriptor, arena, self)
    }
}

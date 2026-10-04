pub mod format;
pub mod ir;
pub mod metadata;
pub mod writer;
pub mod reader;

pub use format::*;
pub use ir::*;
pub use metadata::*;
pub use writer::*;
pub use reader::*;

#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub expected_compiler_version: String,
    pub expected_target: TargetContract,
    pub expected_source_fingerprint: Option<format::Fingerprint>,
    pub expected_dependencies: std::collections::HashMap<String, format::Fingerprint>,
    pub expected_execution_dependencies: std::collections::HashMap<String, format::Fingerprint>,
}

pub fn validate_artifact(manifest: &crate::format::Manifest, ctx: &ValidationContext) -> Result<(), String> {
    if manifest.provenance.compiler_version != ctx.expected_compiler_version {
        return Err(format!("compiler version mismatch: expected {}, got {}", ctx.expected_compiler_version, manifest.provenance.compiler_version));
    }
    if manifest.target.target_triple != ctx.expected_target.target_triple {
        return Err(format!("target triple mismatch: expected {}, got {}", ctx.expected_target.target_triple, manifest.target.target_triple));
    }
    for (field, actual, expected) in [
        ("CPU", &manifest.target.cpu, &ctx.expected_target.cpu),
        ("features", &manifest.target.features, &ctx.expected_target.features),
        ("object format", &manifest.target.object_format, &ctx.expected_target.object_format),
        ("ABI", &manifest.target.abi, &ctx.expected_target.abi),
        ("endianness", &manifest.target.endianness, &ctx.expected_target.endianness),
    ] {
        if actual != expected { return Err(format!("target {field} mismatch: expected {expected}, got {actual}")); }
    }
    if manifest.target.pointer_width != ctx.expected_target.pointer_width {
        return Err(format!("target pointer width mismatch: expected {}, got {}", ctx.expected_target.pointer_width, manifest.target.pointer_width));
    }
    if let Some(expected_source) = ctx.expected_source_fingerprint {
        if manifest.provenance.source_fingerprint != expected_source {
            return Err(format!("source fingerprint mismatch"));
        }
    }
    for dep in &manifest.dependencies.deps {
        if let Some(expected_dep_fingerprint) = ctx.expected_dependencies.get(&dep.provider_name) {
            if &dep.interface_fingerprint != expected_dep_fingerprint {
                return Err(format!("dependency interface fingerprint mismatch for provider {}", dep.provider_name));
            }
        }
        if let Some(expected_exec_fingerprint) = ctx.expected_execution_dependencies.get(&dep.provider_name) {
            if let Some(exec_fp) = &dep.execution_fingerprint {
                if exec_fp != expected_exec_fingerprint {
                    return Err(format!("dependency execution fingerprint mismatch for provider {}", dep.provider_name));
                }
            }
        }
    }
    Ok(())
}

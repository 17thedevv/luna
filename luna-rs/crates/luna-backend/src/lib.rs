pub mod llvm_codegen;
pub mod linker;
pub mod target_contract;
pub mod target_matrix;

pub use llvm_codegen::{LLVMBackend, TargetConfig, BackendError};
pub use linker::{compile_ll_to_exe, link_obj_to_exe, link_objs_to_exe, runtime_library};
pub use target_matrix::{canonical_supported_target, LunaTarget, SUPPORTED_TARGETS};

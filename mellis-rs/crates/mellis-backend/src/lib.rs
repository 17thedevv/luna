pub mod llvm_codegen;
pub mod linker;

pub use llvm_codegen::generate_llvm_ir;
pub use linker::compile_ll_to_exe;

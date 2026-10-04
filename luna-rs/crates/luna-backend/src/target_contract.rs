use crate::{BackendError, TargetConfig};
use inkwell::targets::{
    ByteOrdering, CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
    TargetTriple,
};
use inkwell::{context::Context, memory_buffer::MemoryBuffer};
use luna_llib::TargetContract;

impl TargetConfig {
    pub(crate) fn create_machine(&self) -> Result<TargetMachine, BackendError> {
        Target::initialize_all(&InitializationConfig::default());
        let triple = TargetTriple::create(&self.triple);
        Target::from_triple(&triple)
            .map_err(|error| BackendError::TargetInitFailed(error.to_string()))?
            .create_target_machine(
                &triple,
                &self.cpu,
                &self.features,
                self.optimization,
                if self.triple.contains("windows") {
                    RelocMode::Default
                } else {
                    RelocMode::PIC
                },
                CodeModel::Default,
            )
            .ok_or_else(|| BackendError::TargetInitFailed("target machine unavailable".into()))
    }

    /// Describe the same LLVM target machine used to emit the provider object.
    /// This does not certify Luna's lowering/layout for an arbitrary target.
    pub fn target_contract(&self) -> Result<TargetContract, BackendError> {
        self.describe_target().map(|(contract, _)| contract)
    }

    fn describe_target(&self) -> Result<(TargetContract, ObjectIdentity), BackendError> {
        let machine = self.create_machine()?;
        let data = machine.get_target_data();
        let context = Context::create();
        let probe = context.create_module("target_contract");
        probe.set_triple(&machine.get_triple());
        probe.set_data_layout(&data.get_data_layout());
        let object = machine
            .write_to_memory_buffer(&probe, FileType::Object)
            .map_err(|error| BackendError::ObjectEmissionFailed(error.to_string()))?;
        let identity = object_identity(object.as_slice())?;
        Ok((
            TargetContract {
                target_triple: self.triple.clone(),
                cpu: self.cpu.clone(),
                features: self.features.clone(),
                object_format: identity.format.into(),
                // The triple identifies the platform ABI/environment; this opaque
                // identity additionally binds LLVM's default calling convention and
                // physical data layout. No host word-size assumption is involved.
                abi: format!(
                    "llvm-default;layout={}",
                    data.get_data_layout().as_str().to_string_lossy()
                ),
                pointer_width: (data.get_pointer_byte_size(None) * 8)
                    .try_into()
                    .map_err(|_| {
                        BackendError::TargetInitFailed("pointer width cannot be represented".into())
                    })?,
                endianness: match data.get_byte_ordering() {
                    ByteOrdering::LittleEndian => "little",
                    ByteOrdering::BigEndian => "big",
                }
                .into(),
            },
            identity,
        ))
    }
}

fn default_description() -> Result<(TargetContract, ObjectIdentity), BackendError> {
    static CONTRACT: std::sync::OnceLock<Result<(TargetContract, ObjectIdentity), String>> =
        std::sync::OnceLock::new();
    CONTRACT
        .get_or_init(|| {
            TargetConfig::default()
                .describe_target()
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(BackendError::TargetInitFailed)
}

pub fn default_target_contract() -> Result<TargetContract, BackendError> {
    default_description().map(|(contract, _)| contract)
}

pub fn default_object_identity() -> Result<ObjectIdentity, BackendError> {
    default_description().map(|(_, identity)| identity)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectIdentity {
    pub format: &'static str,
    pub machine: u32,
    pub pointer_width: Option<u8>,
    pub endianness: &'static str,
}

pub fn object_identity(bytes: &[u8]) -> Result<ObjectIdentity, BackendError> {
    let format = object_format(bytes)?;
    let invalid = || BackendError::ObjectEmissionFailed("invalid relocatable object header".into());
    let (machine, pointer_width, endianness) = match format {
        "COFF" => (
            u16::from_le_bytes(bytes.get(..2).ok_or_else(invalid)?.try_into().unwrap()) as u32,
            None,
            "little",
        ),
        "ELF" => {
            let width = match bytes.get(4) {
                Some(1) => 32,
                Some(2) => 64,
                _ => return Err(invalid()),
            };
            let little = match bytes.get(5) {
                Some(1) => true,
                Some(2) => false,
                _ => return Err(invalid()),
            };
            let kind: [u8; 2] = bytes.get(16..18).ok_or_else(invalid)?.try_into().unwrap();
            if (if little {
                u16::from_le_bytes(kind)
            } else {
                u16::from_be_bytes(kind)
            }) != 1
            {
                return Err(invalid());
            }
            let machine: [u8; 2] = bytes.get(18..20).ok_or_else(invalid)?.try_into().unwrap();
            (
                (if little {
                    u16::from_le_bytes(machine)
                } else {
                    u16::from_be_bytes(machine)
                }) as u32,
                Some(width),
                if little { "little" } else { "big" },
            )
        }
        "MachO" => {
            let magic = bytes.get(..4).ok_or_else(invalid)?;
            let (width, little) = match magic {
                [0xce, 0xfa, 0xed, 0xfe] => (32, true),
                [0xcf, 0xfa, 0xed, 0xfe] => (64, true),
                [0xfe, 0xed, 0xfa, 0xce] => (32, false),
                [0xfe, 0xed, 0xfa, 0xcf] => (64, false),
                _ => return Err(invalid()),
            };
            let kind: [u8; 4] = bytes.get(12..16).ok_or_else(invalid)?.try_into().unwrap();
            if (if little {
                u32::from_le_bytes(kind)
            } else {
                u32::from_be_bytes(kind)
            }) != 1
            {
                return Err(invalid());
            }
            let machine: [u8; 4] = bytes.get(4..8).ok_or_else(invalid)?.try_into().unwrap();
            (
                if little {
                    u32::from_le_bytes(machine)
                } else {
                    u32::from_be_bytes(machine)
                },
                Some(width),
                if little { "little" } else { "big" },
            )
        }
        _ => {
            return Err(BackendError::ObjectEmissionFailed(format!(
                "unsupported relocatable object format: {format}"
            )))
        }
    };
    Ok(ObjectIdentity {
        format,
        machine,
        pointer_width,
        endianness,
    })
}

/// LLVM validates the binary instead of guessing its format from a filename.
pub fn object_format(bytes: &[u8]) -> Result<&'static str, BackendError> {
    use llvm_sys::object::*;
    let buffer = MemoryBuffer::create_from_memory_range_copy(bytes, "object-format");
    let mut error = std::ptr::null_mut();
    // SAFETY: buffer remains live until after the binary is disposed. Object
    // parsing does not require an LLVM IR context. LLVM owns the error message.
    unsafe {
        let binary = LLVMCreateBinary(buffer.as_mut_ptr(), std::ptr::null_mut(), &mut error);
        if binary.is_null() {
            let message = if error.is_null() {
                "invalid object binary".into()
            } else {
                std::ffi::CStr::from_ptr(error)
                    .to_string_lossy()
                    .into_owned()
            };
            if !error.is_null() {
                llvm_sys::core::LLVMDisposeMessage(error);
            }
            return Err(BackendError::ObjectEmissionFailed(message));
        }
        let kind = LLVMBinaryGetType(binary);
        LLVMDisposeBinary(binary);
        match kind {
            LLVMBinaryType::LLVMBinaryTypeCOFF => Ok("COFF"),
            LLVMBinaryType::LLVMBinaryTypeELF32L
            | LLVMBinaryType::LLVMBinaryTypeELF32B
            | LLVMBinaryType::LLVMBinaryTypeELF64L
            | LLVMBinaryType::LLVMBinaryTypeELF64B => Ok("ELF"),
            LLVMBinaryType::LLVMBinaryTypeMachO32L
            | LLVMBinaryType::LLVMBinaryTypeMachO32B
            | LLVMBinaryType::LLVMBinaryTypeMachO64L
            | LLVMBinaryType::LLVMBinaryTypeMachO64B => Ok("MachO"),
            LLVMBinaryType::LLVMBinaryTypeWasm => Ok("Wasm"),
            other => Err(BackendError::ObjectEmissionFailed(format!(
                "unsupported object binary: {other:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contracts_follow_llvm_platform_and_pointer_layout() {
        for (triple, format, width) in [
            ("x86_64-pc-windows-gnu", "COFF", 64),
            ("x86_64-unknown-linux-gnu", "ELF", 64),
            ("x86_64-apple-darwin", "MachO", 64),
            ("i686-unknown-linux-gnu", "ELF", 32),
        ] {
            let config = TargetConfig {
                triple: triple.into(),
                cpu: "generic".into(),
                features: String::new(),
                optimization: inkwell::OptimizationLevel::None,
            };
            let contract = config.target_contract().unwrap();
            assert_eq!(contract.target_triple, triple);
            assert_eq!(contract.object_format, format);
            assert_eq!(contract.pointer_width, width);
            assert_eq!(contract.endianness, "little");
            assert!(contract.abi.starts_with("llvm-default;layout=e-"));
        }
        // These are target-descriptor probes, not native-language conformance
        // or a claim that Luna's 32-bit lowering is supported.
    }

    #[test]
    fn invalid_target_and_nonobject_bytes_fail_closed() {
        let mut config = TargetConfig::default();
        config.triple = "invalid-target".into();
        assert!(config.target_contract().is_err());
        assert!(object_format(b"not an object file").is_err());
    }

    #[test]
    fn incomplete_32_bit_lowering_is_rejected_despite_valid_descriptor() {
        let config = TargetConfig {
            triple: "i686-unknown-linux-gnu".into(),
            cpu: "generic".into(),
            features: String::new(),
            optimization: inkwell::OptimizationLevel::None,
        };
        assert_eq!(config.target_contract().unwrap().pointer_width, 32);
        let context = Context::create();
        let module = luna_mvir::Module::new();
        let semantic = luna_semantic::SemanticContext::new();
        let mut backend = crate::LLVMBackend::new(&context, &module, &semantic, "layout", &config);
        assert!(
            matches!(backend.compile(), Err(BackendError::TargetInitFailed(message))
            if message.contains("refusing incomplete target layout"))
        );
    }
}

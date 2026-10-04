use super::{BackendError, LLVMBackend};
use inkwell::targets::{ByteOrdering, CodeModel, RelocMode, Target, TargetData, TargetTriple};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, PointerValue};
use luna_mvir::static_data::{StaticData, StaticType, StaticValue};
use luna_semantic::BuiltinType;

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::InvariantViolation(message.into())
}

impl<'a, 'ctx> LLVMBackend<'a, 'ctx> {
    fn static_layout(&self) -> Result<TargetData, BackendError> {
        let config = &self.target_config;
        let triple = TargetTriple::create(&config.triple);
        let target = Target::from_triple(&triple)
            .map_err(|e| BackendError::TargetInitFailed(e.to_string()))?;
        let machine = target
            .create_target_machine(
                &triple,
                &config.cpu,
                &config.features,
                config.optimization,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| {
                BackendError::TargetInitFailed("static storage target layout unavailable".into())
            })?;
        Ok(machine.get_target_data())
    }

    fn static_type(
        &self,
        ty: &StaticType,
        layout: &TargetData,
    ) -> Result<BasicTypeEnum<'ctx>, BackendError> {
        Ok(match ty {
            StaticType::Unit => self.context.struct_type(&[], false).into(),
            StaticType::Primitive(b) => match b {
                BuiltinType::Bool => self.context.bool_type().into(),
                BuiltinType::I8 | BuiltinType::U8 => self.context.i8_type().into(),
                BuiltinType::I16 | BuiltinType::U16 => self.context.i16_type().into(),
                BuiltinType::I32 | BuiltinType::U32 | BuiltinType::Char => {
                    self.context.i32_type().into()
                }
                BuiltinType::I64 | BuiltinType::U64 | BuiltinType::Isize | BuiltinType::Usize => {
                    self.context.i64_type().into()
                }
                BuiltinType::I128 | BuiltinType::U128 => self.context.i128_type().into(),
                BuiltinType::F32 => self.context.f32_type().into(),
                BuiltinType::F64 => self.context.f64_type().into(),
                BuiltinType::String => self.context.ptr_type(Default::default()).into(),
            },
            StaticType::Aggregate(fields) => self
                .context
                .struct_type(
                    &fields
                        .iter()
                        .map(|f| self.static_type(f, layout))
                        .collect::<Result<Vec<_>, _>>()?,
                    false,
                )
                .into(),
            StaticType::Array { element, len } => self
                .static_type(element, layout)?
                .array_type(
                    (*len)
                        .try_into()
                        .map_err(|_| invalid("constant array exceeds LLVM array length"))?,
                )
                .into(),
            StaticType::Enum(variants) => {
                let max = variants
                    .iter()
                    .map(|v| self.static_type(v, layout).map(|t| layout.get_abi_size(&t)))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .max()
                    .unwrap_or(0);
                let words = max.div_ceil(8).max(2);
                self.context
                    .struct_type(
                        &[
                            self.context.i32_type().into(),
                            self.context
                                .i64_type()
                                .array_type(
                                    words
                                        .try_into()
                                        .map_err(|_| invalid("constant enum payload too large"))?,
                                )
                                .into(),
                        ],
                        false,
                    )
                    .into()
            }
        })
    }

    fn static_string(&self, value: &str, name: &str) -> Result<PointerValue<'ctx>, BackendError> {
        let init = self.context.const_string(value.as_bytes(), true);
        if let Some(global) = self.llvm_module.get_global(name) {
            if global.get_initializer() != Some(init.into()) {
                return Err(invalid("inconsistent constant string identity"));
            }
            return Ok(global.as_pointer_value());
        }
        let global = self.llvm_module.add_global(init.get_type(), None, name);
        global.set_initializer(&init);
        global.set_constant(true);
        // Coalesce strings together with their owning constant when a portable
        // generic body and a native provider both instantiate the same data.
        self.static_linkage(global, name);
        Ok(global.as_pointer_value())
    }

    fn static_linkage(&self, global: inkwell::values::GlobalValue<'ctx>, name: &str) {
        global.set_linkage(inkwell::module::Linkage::LinkOnceODR);
        if self.supports_comdat {
            let comdat = self.llvm_module.get_or_insert_comdat(name);
            comdat.set_selection_kind(inkwell::comdat::ComdatSelectionKind::Any);
            global.set_comdat(comdat);
        }
    }

    fn static_initializer(
        &self,
        ty: &StaticType,
        value: &StaticValue,
        name: &str,
        layout: &TargetData,
    ) -> Result<BasicValueEnum<'ctx>, BackendError> {
        let llvm_ty = self.static_type(ty, layout)?;
        Ok(match (ty, value) {
            (StaticType::Unit, StaticValue::Unit) => llvm_ty.const_zero(),
            (StaticType::Primitive(BuiltinType::Bool), StaticValue::Bool(v)) => {
                self.context.bool_type().const_int(*v as u64, false).into()
            }
            (StaticType::Primitive(b), StaticValue::Int(v)) if b.is_integer() => {
                let bits = *v as u128;
                llvm_ty
                    .into_int_type()
                    .const_int_arbitrary_precision(&[bits as u64, (bits >> 64) as u64])
                    .into()
            }
            (StaticType::Primitive(BuiltinType::Char), StaticValue::Char(v))
                if char::from_u32(*v).is_some() =>
            {
                self.context.i32_type().const_int(*v as u64, false).into()
            }
            (StaticType::Primitive(BuiltinType::F32 | BuiltinType::F64), StaticValue::Float(v)) => {
                llvm_ty
                    .into_float_type()
                    .const_float(f64::from_bits(*v))
                    .into()
            }
            (StaticType::Primitive(BuiltinType::String), StaticValue::Str(v)) => {
                self.static_string(v, &format!("{name}.str"))?.into()
            }
            (StaticType::Aggregate(types), StaticValue::Aggregate(values))
                if types.len() == values.len() =>
            {
                let fields = types
                    .iter()
                    .zip(values)
                    .enumerate()
                    .map(|(i, (t, v))| {
                        self.static_initializer(t, v, &format!("{name}.{i}"), layout)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                llvm_ty
                    .into_struct_type()
                    .const_named_struct(&fields)
                    .into()
            }
            (StaticType::Array { element, len }, StaticValue::Aggregate(values))
                if *len == values.len() as u64 =>
            {
                let elems = values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        self.static_initializer(element, v, &format!("{name}.{i}"), layout)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                match self.static_type(element, layout)? {
                    BasicTypeEnum::IntType(t) => t
                        .const_array(&elems.iter().map(|v| v.into_int_value()).collect::<Vec<_>>())
                        .into(),
                    BasicTypeEnum::FloatType(t) => t
                        .const_array(
                            &elems
                                .iter()
                                .map(|v| v.into_float_value())
                                .collect::<Vec<_>>(),
                        )
                        .into(),
                    BasicTypeEnum::PointerType(t) => t
                        .const_array(
                            &elems
                                .iter()
                                .map(|v| v.into_pointer_value())
                                .collect::<Vec<_>>(),
                        )
                        .into(),
                    BasicTypeEnum::StructType(t) => t
                        .const_array(
                            &elems
                                .iter()
                                .map(|v| v.into_struct_value())
                                .collect::<Vec<_>>(),
                        )
                        .into(),
                    BasicTypeEnum::ArrayType(t) => t
                        .const_array(
                            &elems
                                .iter()
                                .map(|v| v.into_array_value())
                                .collect::<Vec<_>>(),
                        )
                        .into(),
                    _ => return Err(invalid("unsupported constant array element")),
                }
            }
            (StaticType::Enum(variants), StaticValue::Enum { tag, payload, .. }) => {
                let payload_ty = variants
                    .get(*tag as usize)
                    .ok_or_else(|| invalid("constant enum tag out of range"))?;
                let payload_array = llvm_ty
                    .into_struct_type()
                    .get_field_type_at_index(1)
                    .unwrap()
                    .into_array_type();
                let mut bytes = vec![0u8; payload_array.len() as usize * 8];
                let mut pointers = Vec::new();
                self.pack_static(
                    payload_ty,
                    payload,
                    name,
                    layout,
                    &mut bytes,
                    &mut pointers,
                    0,
                )?;
                let little = layout.get_byte_ordering() == ByteOrdering::LittleEndian;
                let mut words = bytes
                    .chunks_exact(8)
                    .map(|chunk| {
                        let array: [u8; 8] = chunk.try_into().unwrap();
                        self.context.i64_type().const_int(
                            if little {
                                u64::from_le_bytes(array)
                            } else {
                                u64::from_be_bytes(array)
                            },
                            false,
                        )
                    })
                    .collect::<Vec<_>>();
                for (offset, pointer) in pointers {
                    if offset % 8 != 0 || offset + 8 > bytes.len() {
                        return Err(invalid("constant pointer relocation is not word aligned"));
                    }
                    words[offset / 8] = pointer.const_to_int(self.context.i64_type());
                }
                llvm_ty
                    .into_struct_type()
                    .const_named_struct(&[
                        self.context.i32_type().const_int(*tag as u64, false).into(),
                        self.context.i64_type().const_array(&words).into(),
                    ])
                    .into()
            }
            _ => return Err(invalid("constant storage initializer/type mismatch")),
        })
    }

    /// Pack an enum's payload according to the target layout. Pointer fields
    /// remain LLVM relocations, never host addresses or host-endian byte casts.
    fn pack_static(
        &self,
        ty: &StaticType,
        value: &StaticValue,
        name: &str,
        layout: &TargetData,
        bytes: &mut [u8],
        pointers: &mut Vec<(usize, PointerValue<'ctx>)>,
        offset: usize,
    ) -> Result<(), BackendError> {
        let llvm_ty = self.static_type(ty, layout)?;
        let size = layout.get_abi_size(&llvm_ty) as usize;
        if offset.checked_add(size).is_none_or(|end| end > bytes.len()) {
            return Err(invalid("constant payload exceeds enum storage"));
        }
        let little = layout.get_byte_ordering() == ByteOrdering::LittleEndian;
        match (ty, value) {
            (StaticType::Unit, StaticValue::Unit) => {}
            (StaticType::Primitive(BuiltinType::String), StaticValue::Str(v)) => {
                if size != 8 {
                    return Err(invalid(
                        "enum pointer storage requires the current 64-bit ABI",
                    ));
                }
                pointers.push((offset, self.static_string(v, &format!("{name}.str"))?));
            }
            (StaticType::Primitive(b), v) => {
                let bits = match (b, v) {
                    (b, StaticValue::Int(v)) if b.is_integer() => *v as u128,
                    (BuiltinType::Bool, StaticValue::Bool(v)) => *v as u128,
                    (BuiltinType::Char, StaticValue::Char(v)) if char::from_u32(*v).is_some() => {
                        *v as u128
                    }
                    (BuiltinType::F32, StaticValue::Float(v)) => {
                        (f64::from_bits(*v) as f32).to_bits() as u128
                    }
                    (BuiltinType::F64, StaticValue::Float(v)) => *v as u128,
                    _ => return Err(invalid("invalid constant scalar payload")),
                };
                let repr = if little {
                    bits.to_le_bytes()
                } else {
                    bits.to_be_bytes()
                };
                let repr = if little {
                    &repr[..size]
                } else {
                    &repr[16 - size..]
                };
                bytes[offset..offset + size].copy_from_slice(repr);
            }
            (StaticType::Aggregate(types), StaticValue::Aggregate(values))
                if types.len() == values.len() =>
            {
                let struct_ty = llvm_ty.into_struct_type();
                for (i, (t, v)) in types.iter().zip(values).enumerate() {
                    let field_offset = layout
                        .offset_of_element(&struct_ty, i as u32)
                        .ok_or_else(|| invalid("missing static field layout"))?
                        as usize;
                    self.pack_static(
                        t,
                        v,
                        &format!("{name}.{i}"),
                        layout,
                        bytes,
                        pointers,
                        offset + field_offset,
                    )?;
                }
            }
            (StaticType::Array { element, len }, StaticValue::Aggregate(values))
                if *len == values.len() as u64 =>
            {
                let stride = layout.get_abi_size(&self.static_type(element, layout)?) as usize;
                for (i, v) in values.iter().enumerate() {
                    self.pack_static(
                        element,
                        v,
                        &format!("{name}.{i}"),
                        layout,
                        bytes,
                        pointers,
                        offset + i * stride,
                    )?;
                }
            }
            (StaticType::Enum(variants), StaticValue::Enum { tag, payload, .. }) => {
                let repr = if little {
                    tag.to_le_bytes()
                } else {
                    tag.to_be_bytes()
                };
                bytes[offset..offset + 4].copy_from_slice(&repr);
                let t = variants
                    .get(*tag as usize)
                    .ok_or_else(|| invalid("invalid nested static enum tag"))?;
                let payload_offset = layout
                    .offset_of_element(&llvm_ty.into_struct_type(), 1)
                    .unwrap() as usize;
                self.pack_static(
                    t,
                    payload,
                    name,
                    layout,
                    bytes,
                    pointers,
                    offset + payload_offset,
                )?;
            }
            _ => return Err(invalid("invalid aggregate constant payload")),
        }
        Ok(())
    }

    pub(super) fn static_address(
        &self,
        data: &StaticData,
        ty: luna_semantic::SemanticTypeId,
    ) -> Result<BasicValueEnum<'ctx>, BackendError> {
        let layout = self.static_layout()?;
        if self.static_type(&data.ty, &layout)? != self.map_type(ty)? {
            return Err(invalid(
                "constant storage layout differs from its semantic ABI",
            ));
        }
        let init = self.static_initializer(&data.ty, &data.value, &data.name, &layout)?;
        if self.llvm_module.get_function(&data.name).is_some() {
            return Err(invalid("constant identity collides with a function"));
        }
        if let Some(global) = self.llvm_module.get_global(&data.name) {
            if global.get_initializer() != Some(init) {
                return Err(invalid(
                    "conflicting initializers for the same module constant",
                ));
            }
            return Ok(global.as_pointer_value().into());
        }
        let global = self
            .llvm_module
            .add_global(init.get_type(), None, &data.name);
        global.set_initializer(&init);
        global.set_constant(true);
        global.set_alignment(layout.get_abi_alignment(&init.get_type()));
        self.static_linkage(global, &data.name);
        Ok(global.as_pointer_value().into())
    }
}

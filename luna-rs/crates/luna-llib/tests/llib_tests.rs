use std::io::Cursor;
use luna_llib::{MlibHeader, MlibReader, MlibWriter, MLIB_MAGIC, MLIB_FORMAT_VERSION};
use luna_mvir::{ValueOrigin, Module, Function, BasicBlock, ValueData, Instruction, Terminator, Operand, ValueId, GlobalId, LabelId};
use luna_semantic::ty::SemanticTypeId;
use luna_common::SymbolId;

fn dummy_module() -> Module {
    let mut func = Function {
        name: GlobalId { name: "test_func".to_string(), symbol_id: Some(SymbolId(0)) },
        arg_count: 0,
        link_name: None,
        param_types: vec![],
        is_extern: false,
        is_async: false,
        ret_ty: SemanticTypeId(0),
        lifetime_info: Default::default(),
        values: Vec::new(),
        blocks: Vec::new(),
    };
    
    // Add some dummy instructions
    func.values.push(ValueData { inst: Instruction::Alloca, ty: SemanticTypeId(0), span: None, origin: ValueOrigin::Temporary });
    func.values.push(ValueData { inst: Instruction::FieldPtr { base: Operand::Value(ValueId(0)), field_idx: 0, field_name: Some("payload".into()) }, ty: SemanticTypeId(0), span: None, origin: ValueOrigin::Temporary });
    func.values.push(ValueData::new(Instruction::StaticAddress(luna_mvir::static_data::StaticData {
        name: "__luna_const_provider_namespace_declaration".into(),
        ty: luna_mvir::static_data::StaticType::Aggregate(vec![
            luna_mvir::static_data::StaticType::Primitive(luna_semantic::BuiltinType::I128),
            luna_mvir::static_data::StaticType::Primitive(luna_semantic::BuiltinType::String),
        ]),
        value: luna_mvir::static_data::StaticValue::Aggregate(vec![
            luna_mvir::static_data::StaticValue::Int(i128::MIN),
            luna_mvir::static_data::StaticValue::Str("λ\0data".into()),
        ]),
    }), SemanticTypeId(0), None));
    
    // Add block
    func.blocks.push(BasicBlock {
        label: LabelId { name: "entry0".to_string() },
        insts: vec![ValueId(0), ValueId(1), ValueId(2)],
        terminator: Some(Terminator::Ret { value: Some(Operand::Value(ValueId(1))) }),
    });

    let mut module = Module::new();
    module.functions.push(func);
    module
}

#[test]
fn test_golden_roundtrip() {
    let module = dummy_module();
    let mut buffer = Vec::new();
    let arena = luna_ast::AstArena::new();
    let source = "";
    
    let manifest = dummy_manifest();
    MlibWriter::write_module(&module, &arena, &[], source, manifest, None, None, &mut buffer).expect("write failed");
    
    let mut cursor = Cursor::new(buffer);
    let (mlib_module, _, _, _) = MlibReader::read_module(&mut cursor).expect("read failed");
    
    assert_eq!(mlib_module.functions.len(), 1);
    assert_eq!(mlib_module.functions[0].name, "test_func");
    assert_eq!(mlib_module.functions[0].values.len(), 3);
    let Instruction::StaticAddress(expected) = &module.functions[0].values[2].inst else { unreachable!() };
    assert!(matches!(&mlib_module.functions[0].values[2].inst, luna_llib::MlibInstruction::StaticAddress(actual) if actual == expected));
    assert!(matches!(
        &mlib_module.functions[0].values[1].inst,
        luna_llib::MlibInstruction::FieldPtr { field_idx: 0, field_name: Some(name), .. } if name == "payload"
    ));
}

#[test]
fn test_version_mismatch() {
    let mut buffer = Vec::new();
    let mut header = MlibHeader::new();
    header.format_version = 999; // wrong version
    header.write_to(&mut buffer).unwrap();
    
    let mut cursor = Cursor::new(buffer);
    let result = MlibReader::read_module(&mut cursor).map(|(m, _, _, _)| m);
    assert!(matches!(result, Err(luna_llib::MlibError::VersionMismatch(999))));
}

#[test]
fn test_mvir_version_mismatch() {
    let mut buffer = Vec::new();
    let mut header = MlibHeader::new();
    header.mvir_version = 999;
    header.write_to(&mut buffer).unwrap();

    let mut cursor = Cursor::new(buffer);
    let result = MlibReader::read_module(&mut cursor);
    assert!(matches!(result, Err(luna_llib::MlibError::VersionMismatch(999))));
}

#[test]
fn test_invalid_magic() {
    let mut buffer = Vec::new();
    let mut header = MlibHeader::new();
    header.magic = *b"XXXX"; // wrong magic
    header.write_to(&mut buffer).unwrap();
    
    let mut cursor = Cursor::new(buffer);
    let result = MlibReader::read_module(&mut cursor).map(|(m, _, _, _)| m);
    assert!(matches!(result, Err(luna_llib::MlibError::InvalidMagic)));
}

#[test]
fn test_corrupted_data() {
    let module = dummy_module();
    let arena = luna_ast::AstArena::new();
    let source = "";
    let mut buffer = Vec::new();
    let manifest = luna_llib::Manifest {
        identity: luna_llib::ArtifactIdentity { package_id: "".into(), version: "".into(), module_id: "".into(), artifact_id: "".into() },
        target: luna_llib::TargetContract { target_triple: "".into(), cpu: "".into(), features: "".into(), object_format: "".into(), abi: "".into(), pointer_width: 64, endianness: "".into() },
        dependencies: luna_llib::DependencyTable::default(),
        object_metadata: None,
        provenance: luna_llib::Provenance {
            source_fingerprint: luna_llib::Fingerprint([0; 32]),
            compiler_version: "".into(),
            codegen_options: "".into(),
            interface_fingerprint: luna_llib::Fingerprint([0; 32]),
            execution_fingerprint: None,
        },
        export_table: Default::default(),
    };
    MlibWriter::write_module(&module, &arena, &[], source, manifest, None, None, &mut buffer).unwrap();
    
    // truncate buffer
    buffer.truncate(buffer.len() / 2);
    
    let mut cursor = Cursor::new(buffer);
    let result = MlibReader::read_module(&mut cursor).map(|(m, _, _, _)| m);
    assert!(result.is_err()); // Either Io or BincodeError
}

#[test]
fn test_unknown_section() {
    let mut buffer = Vec::new();
    let mut header = MlibHeader::new();
    header.section_count = 1;
    header.write_to(&mut buffer).unwrap();
    
    // Write an unknown section ID 0xFFFFFFFF
    buffer.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // section_type
    buffer.extend_from_slice(&0u64.to_le_bytes()); // offset
    buffer.extend_from_slice(&0u64.to_le_bytes()); // length
    
    let mut cursor = Cursor::new(buffer);
    let result = MlibReader::read_module(&mut cursor).map(|(m, _, _, _)| m);
    // The reader errors because of Unknown section during reading the entry
    assert!(result.is_err());
}

fn dummy_manifest() -> luna_llib::Manifest {
    luna_llib::Manifest {
        identity: luna_llib::ArtifactIdentity { package_id: "".into(), version: "".into(), module_id: "".into(), artifact_id: "".into() },
        target: luna_llib::TargetContract { target_triple: "".into(), cpu: "".into(), features: "".into(), object_format: "".into(), abi: "".into(), pointer_width: 64, endianness: "".into() },
        dependencies: luna_llib::DependencyTable::default(),
        object_metadata: None,
        provenance: luna_llib::Provenance {
            source_fingerprint: luna_llib::Fingerprint([0; 32]),
            compiler_version: "".into(),
            codegen_options: "".into(),
            interface_fingerprint: luna_llib::Fingerprint([0; 32]),
            execution_fingerprint: None,
        },
        export_table: Default::default(),
    }
}

#[test]
fn lifetime_annotations_roundtrip_and_reject_nonexistent_abi_or_value_slots() {
    let mut module = dummy_module();
    let function = &mut module.functions[0];
    function.arg_count = 2;
    function.lifetime_info.infer_input_requirements = true;
    function.lifetime_info.input_assumptions = vec![(0,1)];
    function.lifetime_info.checks.insert(ValueId(1), vec![luna_mvir::OutlivesCheck {
        longer_subject: luna_semantic::CanonicalContractSubject::Param(0),
        shorter_subject: luna_semantic::CanonicalContractSubject::SelfVal,
        longer: Operand::Value(ValueId(0)), shorter: Operand::Value(ValueId(1)),
    }]);
    let encode = |module: &Module| {
        let mut buffer = Vec::new();
        MlibWriter::write_module(module, &luna_ast::AstArena::new(), &[], "", dummy_manifest(), None, None, &mut buffer).unwrap();
        MlibReader::read_module(&mut Cursor::new(buffer))
    };
    let (decoded, ..) = encode(&module).unwrap();
    let info = &decoded.functions[0].lifetime_info;
    assert!(info.infer_input_requirements);
    assert_eq!(info.input_assumptions, vec![(0,1)]);
    let check = &info.checks[&1][0];
    assert_eq!(check.longer_subject, luna_semantic::CanonicalContractSubject::Param(0));
    assert_eq!(check.shorter_subject, luna_semantic::CanonicalContractSubject::SelfVal);
    assert_eq!(check.longer, luna_llib::MlibOperand::Value(0));
    for invalid_kind in 0..3 {
        let mut invalid = module.clone();
        let info = &mut invalid.functions[0].lifetime_info;
        match invalid_kind {
            0 => info.input_assumptions = vec![(0,2)],
            1 => { let checks = info.checks.remove(&ValueId(1)).unwrap(); info.checks.insert(ValueId(99), checks); }
            _ => info.checks.get_mut(&ValueId(1)).unwrap()[0].shorter = Operand::Value(ValueId(99)),
        }
        assert!(encode(&invalid).is_err(), "invalid lifetime annotation accepted: {invalid_kind}");
    }
}

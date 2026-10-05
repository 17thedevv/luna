use std::io::{Read, Seek, SeekFrom};
use crate::format::{LlibHeader, MlibHeader, SectionEntry, SectionType, LLIB_MAGIC, MLIB_MAGIC, LLIB_FORMAT_VERSION, MLIB_FORMAT_VERSION};
use crate::ir::{MlibModule, MlibFunction, MlibValue, MlibBlock, MlibInstruction, MlibTerminator, MlibOperand, MlibTypeEntry, MlibCaptureInfo};

#[derive(Debug)]
pub enum MlibError {
    Io(std::io::Error),
    InvalidMagic,
    VersionMismatch(u16),
    UnsupportedContractVersion(u32),
    TargetMismatch,
    ObjectIntegrityMismatch(&'static str),
    SectionChecksumMismatch(u32),
    UnknownSection(u32),
    CorruptedData,
}

pub type LlibError = MlibError;

impl From<std::io::Error> for MlibError {
    fn from(err: std::io::Error) -> Self {
        MlibError::Io(err)
    }
}

pub struct LlibReader;
pub type MlibReader = LlibReader;

#[cfg(test)]
mod raw_anchor_validation_tests {
    use super::*;
    use crate::metadata::{CanonicalInterface, CanonicalType, ExportedSymbol, StableSymbolId};
    use luna_semantic::ty::{BuiltinType, Mutability};
    use std::collections::BTreeMap;

    fn symbol(kind: &str, path: &str, ty_index: Option<u32>) -> ExportedSymbol {
        ExportedSymbol {
            kind: kind.into(),
            ty_index,
            visibility: 1,
            generic_params: Vec::new(),
            constraints: crate::metadata::GenericConstraints::default(),
            symbol_id: StableSymbolId { provider_name: "test".into(), symbol_path: path.into() },
            children: BTreeMap::new(),
            lifetime_contract: None,
            type_lifetime_contract: None,
            raw_storage_anchor_contract: None,
            raw_pointer_effects: None,
            is_unsafe: false,
        }
    }

    fn interface(contract: luna_semantic::CanonicalRawStorageAnchorContract) -> CanonicalInterface {
        let mut field = symbol("Variable", "Owner.ptr", Some(0));
        field.symbol_id = StableSymbolId { provider_name: "test".into(), symbol_path: "Owner.ptr".into() };
        let mut owner = symbol("Struct", "Owner", None);
        owner.children.insert("ptr".into(), field);
        owner.raw_storage_anchor_contract = Some(contract);
        CanonicalInterface {
            exported_symbols: BTreeMap::from([("Owner".into(), owner)]),
            types: vec![CanonicalType::Pointer(Mutability::Mutable, 1), CanonicalType::Primitive(BuiltinType::U8)],
            traits: BTreeMap::new(),
            impl_headers: Vec::new(),
            nominal_layouts: BTreeMap::new(),
        }
    }

    #[test]
    fn rejects_unknown_anchor_contract_version() {
        let iface = interface(luna_semantic::CanonicalRawStorageAnchorContract {
            version: 999,
            field_names: vec!["ptr".into()],
        });
        assert!(matches!(validate_raw_storage_anchor_contracts(&iface), Err(MlibError::UnsupportedContractVersion(999))));
    }

    #[test]
    fn rejects_contract_on_non_struct_or_non_pointer_field() {
        let mut iface = interface(luna_semantic::CanonicalRawStorageAnchorContract::new(vec!["ptr".into()]));
        iface.exported_symbols.get_mut("Owner").unwrap().kind = "Enum".into();
        assert!(matches!(validate_raw_storage_anchor_contracts(&iface), Err(MlibError::CorruptedData)));

        let mut iface = interface(luna_semantic::CanonicalRawStorageAnchorContract::new(vec!["ptr".into()]));
        iface.types[0] = CanonicalType::Primitive(BuiltinType::U8);
        assert!(matches!(validate_raw_storage_anchor_contracts(&iface), Err(MlibError::CorruptedData)));
    }

    #[test]
    fn rejects_raw_pointer_effect_parameter_outside_function_signature() {
        use crate::metadata::{
            CanonicalRawPointerAnchor, CanonicalRawPointerEffect,
            CanonicalRawPointerEffects, CanonicalRawPointerOrigin,
        };
        let mut function = symbol("Function", "relay", Some(0));
        function.raw_pointer_effects = Some(CanonicalRawPointerEffects {
            returned: CanonicalRawPointerEffect {
                origin: CanonicalRawPointerOrigin::FromParameters(vec![1]),
                anchor: CanonicalRawPointerAnchor::Unknown,
            },
            direct_fields: BTreeMap::new(),
        });
        let interface = CanonicalInterface {
            exported_symbols: BTreeMap::from([("relay".into(), function)]),
            types: vec![CanonicalType::Function { params: vec![1], return_type: 2, is_unsafe: false }],
            traits: BTreeMap::new(),
            impl_headers: Vec::new(),
            nominal_layouts: BTreeMap::new(),
        };
        assert!(matches!(validate_raw_pointer_effects(&interface), Err(MlibError::CorruptedData)));
    }
}

/// Validate declaration ownership and index references before trusting generic
/// contracts. Bounds must constrain a binder declared by that owner.
pub fn validate_generic_contracts(interface: &crate::metadata::CanonicalInterface) -> Result<(), MlibError> {
    use crate::metadata::{ExportedSymbol, GenericConstraints, StableSymbolId};
    fn constraints(value: &GenericConstraints, parameters: &[StableSymbolId], len: usize) -> Result<(), MlibError> {
        if value.traits.iter().any(|bound| !parameters.contains(&bound.param)
            || bound.trait_args.iter().any(|&ty| ty as usize >= len))
            || value.associated_equalities.iter().any(|bound| !parameters.contains(&bound.param)
                || bound.target_type as usize >= len) {
            return Err(MlibError::CorruptedData);
        }
        Ok(())
    }
    fn symbol(value: &ExportedSymbol, len: usize) -> Result<(), MlibError> {
        if value.ty_index.is_some_and(|ty| ty as usize >= len) { return Err(MlibError::CorruptedData); }
        constraints(&value.constraints, &value.generic_params, len)?;
        for child in value.children.values() { symbol(child, len)?; }
        Ok(())
    }
    let len = interface.types.len();
    for value in interface.exported_symbols.values() { symbol(value, len)?; }
    let mut owners = std::collections::HashSet::new();
    for header in &interface.impl_headers {
        if !owners.insert(&header.identity) || header.self_type as usize >= len
            || header.trait_args.iter().any(|&ty| ty as usize >= len)
            || header.associated_types.values().any(|&ty| ty as usize >= len)
            || header.methods.len() != header.method_contracts.len() {
            return Err(MlibError::CorruptedData);
        }
        constraints(&header.constraints, &header.generic_params, len)?;
        for (name, method) in &header.method_contracts {
            symbol(method, len)?;
            if method.ty_index != header.methods.get(name).copied() { return Err(MlibError::CorruptedData); }
        }
    }
    for definition in interface.traits.values() {
        constraints(&definition.constraints, &definition.generic_params, len)?;
        if definition.methods.values().any(|&ty| ty as usize >= len) { return Err(MlibError::CorruptedData); }
    }
    for layout in interface.nominal_layouts.values() {
        constraints(&layout.constraints, &layout.generic_params, len)?;
    }
    Ok(())
}

/// A nominal definition carries an ordered ABI contract independently of name
/// lookup. Foreign definitions are validated in their owning dependencies.
pub fn validate_nominal_layouts(interface: &crate::metadata::CanonicalInterface, provider: &str) -> Result<(), MlibError> {
    use crate::metadata::{CanonicalType, CanonicalNominalMembers};
    let len = interface.types.len();
    fn visit(value: &crate::metadata::ExportedSymbol, interface: &crate::metadata::CanonicalInterface) -> Result<(), MlibError> {
        match value.kind.as_str() {
            "Struct" | "Enum" => {
                let layout = interface.nominal_layouts.get(&value.symbol_id).ok_or(MlibError::CorruptedData)?;
                match (&layout.members, value.kind.as_str()) {
                    (CanonicalNominalMembers::Struct(fields), "Struct") => {
                        for field in fields {
                            let child = value.children.get(&field.name).ok_or(MlibError::CorruptedData)?;
                            if child.ty_index != Some(field.ty) || child.visibility != field.visibility { return Err(MlibError::CorruptedData); }
                        }
                    }
                    (CanonicalNominalMembers::Enum(variants), "Enum") => {
                        for variant in variants {
                            if !value.children.get(&variant.name).is_some_and(|child| child.kind == "EnumVariant") { return Err(MlibError::CorruptedData); }
                        }
                    }
                    _ => return Err(MlibError::CorruptedData),
                }
            }
            "EnumVariant" => {
                let (owner, name) = value.symbol_id.symbol_path.rsplit_once("::").ok_or(MlibError::CorruptedData)?;
                let identity = crate::metadata::StableSymbolId { provider_name: value.symbol_id.provider_name.clone(), symbol_path: owner.into() };
                let layout = interface.nominal_layouts.get(&identity).ok_or(MlibError::CorruptedData)?;
                if !matches!(&layout.members, CanonicalNominalMembers::Enum(variants) if variants.iter().any(|v| v.name == name)) {
                    return Err(MlibError::CorruptedData);
                }
            }
            _ => {}
        }
        for child in value.children.values() { visit(child, interface)?; }
        Ok(())
    }
    for (identity, layout) in &interface.nominal_layouts {
        if identity.provider_name != provider { return Err(MlibError::CorruptedData); }
        let mut names = std::collections::HashSet::new();
        match &layout.members {
            CanonicalNominalMembers::Struct(fields) => {
                for field in fields {
                    if field.ty as usize >= len || field.visibility > 2 || !names.insert(&field.name) {
                        return Err(MlibError::CorruptedData);
                    }
                }
            }
            CanonicalNominalMembers::Enum(variants) => {
                if layout.raw_storage_anchor_contract.is_some() { return Err(MlibError::CorruptedData); }
                for variant in variants {
                    if variant.payload as usize >= len || !names.insert(&variant.name) {
                        return Err(MlibError::CorruptedData);
                    }
                }
            }
        }
        if let Some(contract) = &layout.raw_storage_anchor_contract {
            if contract.version != luna_semantic::CanonicalRawStorageAnchorContract::CURRENT_VERSION {
                return Err(MlibError::UnsupportedContractVersion(contract.version));
            }
            let CanonicalNominalMembers::Struct(fields) = &layout.members else { unreachable!() };
            if contract.field_names.is_empty() || contract.field_names.windows(2).any(|pair| pair[0] >= pair[1])
                || contract.field_names.iter().any(|name| !fields.iter().any(|field|
                    &field.name == name && matches!(interface.types[field.ty as usize], CanonicalType::Pointer(..)))) {
                return Err(MlibError::CorruptedData);
            }
        }
    }
    for ty in &interface.types {
        match ty {
            CanonicalType::Struct(identity, _, inline) | CanonicalType::Enum(identity, _, inline) => {
                if !inline.is_empty() { return Err(MlibError::CorruptedData); }
                if identity.provider_name == provider {
                    let layout = interface.nominal_layouts.get(identity).ok_or(MlibError::CorruptedData)?;
                    if matches!(ty, CanonicalType::Struct(..)) != matches!(layout.members, CanonicalNominalMembers::Struct(_)) {
                        return Err(MlibError::CorruptedData);
                    }
                }
            }
            _ => {}
        }
    }
    for value in interface.exported_symbols.values() { visit(value, interface)?; }
    for header in &interface.impl_headers {
        for method in header.method_contracts.values() { visit(method, interface)?; }
    }
    Ok(())
}

fn check_semantic_metadata_version(data: &[u8]) -> Result<(), MlibError> {
    if data.len() < std::mem::size_of::<u16>() {
        return Err(MlibError::CorruptedData);
    }
    let version = u16::from_le_bytes([data[0], data[1]]);
    if version == crate::format::SEMANTIC_METADATA_VERSION {
        Ok(())
    } else {
        Err(MlibError::VersionMismatch(version))
    }
}

/// Validate stable raw-storage contracts before exposing artifact metadata to
/// semantic reconstruction. A malformed contract must never be silently
/// dropped by a downstream decoder.
pub fn validate_raw_storage_anchor_contracts(
    interface: &crate::metadata::CanonicalInterface,
) -> Result<(), MlibError> {
    fn visit(
        symbol: &crate::metadata::ExportedSymbol,
        interface: &crate::metadata::CanonicalInterface,
    ) -> Result<(), MlibError> {
        if let Some(contract) = &symbol.raw_storage_anchor_contract {
            if contract.version != luna_semantic::CanonicalRawStorageAnchorContract::CURRENT_VERSION {
                return Err(MlibError::UnsupportedContractVersion(contract.version));
            }
            if symbol.kind != "Struct" || contract.field_names.is_empty() {
                return Err(MlibError::CorruptedData);
            }
            let mut previous: Option<&str> = None;
            for field_name in &contract.field_names {
                if previous.is_some_and(|p| p >= field_name.as_str()) {
                    return Err(MlibError::CorruptedData);
                }
                previous = Some(field_name);
                let field = symbol.children.get(field_name).ok_or(MlibError::CorruptedData)?;
                let ty_index = field.ty_index.ok_or(MlibError::CorruptedData)? as usize;
                if !matches!(interface.types.get(ty_index), Some(crate::metadata::CanonicalType::Pointer(_, _))) {
                    return Err(MlibError::CorruptedData);
                }
            }
        }
        for child in symbol.children.values() {
            visit(child, interface)?;
        }
        Ok(())
    }

    for symbol in interface.exported_symbols.values() {
        visit(symbol, interface)?;
    }
    for header in &interface.impl_headers {
        for symbol in header.method_contracts.values() { visit(symbol, interface)?; }
    }
    Ok(())
}

/// Validate stable raw-pointer return effects before exposing them to callers.
/// Parameter indices are interpreted against the canonical function type;
/// no borrowck state or session-local identity is read from the artifact.
pub fn validate_raw_pointer_effects(
    interface: &crate::metadata::CanonicalInterface,
) -> Result<(), MlibError> {
    use crate::metadata::{CanonicalRawPointerAnchor, CanonicalRawPointerAnchorSource, CanonicalRawPointerOrigin};

    fn validate_effect(
        effect: &crate::metadata::CanonicalRawPointerEffect,
        parameter_count: usize,
    ) -> Result<(), MlibError> {
        if let CanonicalRawPointerOrigin::FromParameters(parameters) = &effect.origin {
            if parameters.iter().any(|parameter| *parameter as usize >= parameter_count) {
                return Err(MlibError::CorruptedData);
            }
        }
        if let CanonicalRawPointerAnchor::From(sources) = &effect.anchor {
            for source in sources {
                match source {
                    CanonicalRawPointerAnchorSource::RawParameter(parameter) => {
                        if *parameter as usize >= parameter_count { return Err(MlibError::CorruptedData); }
                    }
                    CanonicalRawPointerAnchorSource::OwnerField { parameter, field_name } => {
                        if *parameter as usize >= parameter_count || field_name.is_empty() {
                            return Err(MlibError::CorruptedData);
                        }
                    }
                    CanonicalRawPointerAnchorSource::Unknown => {}
                }
            }
        }
        Ok(())
    }

    fn visit(
        symbol: &crate::metadata::ExportedSymbol,
        interface: &crate::metadata::CanonicalInterface,
    ) -> Result<(), MlibError> {
        if let Some(effects) = &symbol.raw_pointer_effects {
            if !matches!(symbol.kind.as_str(), "Function" | "ExternFunction") {
                return Err(MlibError::CorruptedData);
            }
            let ty_index = symbol.ty_index.ok_or(MlibError::CorruptedData)? as usize;
            let Some(crate::metadata::CanonicalType::Function { params, .. }) = interface.types.get(ty_index) else {
                return Err(MlibError::CorruptedData);
            };
            validate_effect(&effects.returned, params.len())?;
            for (field_name, effect) in &effects.direct_fields {
                if field_name.is_empty() { return Err(MlibError::CorruptedData); }
                validate_effect(effect, params.len())?;
            }
        }
        for child in symbol.children.values() { visit(child, interface)?; }
        Ok(())
    }

    for symbol in interface.exported_symbols.values() { visit(symbol, interface)?; }
    for header in &interface.impl_headers {
        for symbol in header.method_contracts.values() { visit(symbol, interface)?; }
    }
    Ok(())
}

fn validate_target_header(header: &LlibHeader, manifest: &crate::format::Manifest) -> Result<(), MlibError> {
    let end = header.target_triple.iter().position(|byte| *byte == 0).unwrap_or(64);
    if header.target_triple[end..].iter().any(|byte| *byte != 0)
        || header.target_triple[..end] != *manifest.target.target_triple.as_bytes() {
        return Err(MlibError::TargetMismatch);
    }
    Ok(())
}

fn validate_header_versions(header: &LlibHeader) -> Result<(), MlibError> {
    if header.compiler_version != crate::format::LLIB_COMPILER_VERSION {
        return Err(MlibError::VersionMismatch(header.compiler_version));
    }
    if header.mvir_version != crate::format::LLIB_MVIR_VERSION {
        return Err(MlibError::VersionMismatch(header.mvir_version));
    }
    Ok(())
}
// The native envelope is checked before any payload is exposed. This is a
// consistency check, not authentication of the artifact publisher.
fn read_native_envelope<R: Read + Seek>(
    reader: &mut R,
    header: &LlibHeader,
    sections: &[SectionEntry],
) -> Result<(Option<crate::format::Manifest>, Option<Vec<u8>>), MlibError> {
    let mut manifest: Option<crate::format::Manifest> = None;
    let mut object = None;
    for section in sections {
        if !matches!(section.section_type, SectionType::Manifest | SectionType::ObjectCode) { continue; }
        reader.seek(SeekFrom::Start(section.offset))?;
        let size = usize::try_from(section.size).map_err(|_| MlibError::CorruptedData)?;
        let mut bytes = vec![0u8; size];
        reader.read_exact(&mut bytes)?;
        match section.section_type {
            SectionType::Manifest => {
                if manifest.is_some() { return Err(MlibError::CorruptedData); }
                let value = bincode::deserialize(&bytes).map_err(|_| MlibError::CorruptedData)?;
                validate_target_header(header, &value)?;
                manifest = Some(value);
            }
            SectionType::ObjectCode => {
                if object.is_some() { return Err(MlibError::ObjectIntegrityMismatch("duplicate object sections")); }
                object = Some(bytes);
            }
            _ => unreachable!(),
        }
    }
    match (&manifest, object.as_deref()) {
        (Some(manifest), Some(bytes)) => {
            let metadata = manifest.object_metadata.as_ref()
                .ok_or(MlibError::ObjectIntegrityMismatch("missing object metadata"))?;
            if bytes.is_empty() { return Err(MlibError::ObjectIntegrityMismatch("empty object payload")); }
            if metadata.size != bytes.len() as u64 { return Err(MlibError::ObjectIntegrityMismatch("object size mismatch")); }
            if metadata.hash != crate::format::Fingerprint::from_slice(bytes).0 {
                return Err(MlibError::ObjectIntegrityMismatch("object hash mismatch"));
            }
            if metadata.format != manifest.target.object_format {
                return Err(MlibError::ObjectIntegrityMismatch("object metadata format mismatch"));
            }
        }
        (Some(manifest), None) if manifest.object_metadata.is_some() => {
            return Err(MlibError::ObjectIntegrityMismatch("object metadata without payload"));
        }
        (None, Some(_)) => return Err(MlibError::ObjectIntegrityMismatch("object payload without manifest")),
        _ => {},
    }
    Ok((manifest, object))
}

fn read_header_and_sections<R: Read + Seek>(reader: &mut R) -> Result<(LlibHeader, Vec<SectionEntry>), MlibError> {
    let header = LlibHeader::read_from(reader)?;
    if header.magic != LLIB_MAGIC && header.magic != MLIB_MAGIC { return Err(MlibError::InvalidMagic); }
    if header.format_version != LLIB_FORMAT_VERSION && header.format_version != MLIB_FORMAT_VERSION {
        return Err(MlibError::VersionMismatch(header.format_version));
    }
    validate_header_versions(&header)?;
    let header_end = reader.stream_position()?;
    let file_end = reader.seek(SeekFrom::End(0))?;
    if header.section_table_offset < header_end || header.section_table_offset > file_end {
        return Err(MlibError::CorruptedData);
    }
    reader.seek(SeekFrom::Start(header.section_table_offset))?;
    let mut sections = Vec::new();
    for _ in 0..header.section_count { sections.push(SectionEntry::read_from(reader)?); }
    let table_end = reader.stream_position()?;
    let mut ranges = Vec::new();
    for section in &sections {
        let end = section.offset.checked_add(section.size).ok_or(MlibError::CorruptedData)?;
        if section.offset < table_end || end > file_end { return Err(MlibError::CorruptedData); }
        if section.size != 0 { ranges.push((section.offset, end)); }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) { return Err(MlibError::CorruptedData); }
    // Stream checksums before deserializing portable AST, metadata or MVIR.
    // The table's existing u64 field carries the first 64 bits of SHA-256.
    // This detects incoherent/corrupt payloads; it is not publisher authentication.
    use sha2::{Digest, Sha256};
    for section in &sections {
        reader.seek(SeekFrom::Start(section.offset))?;
        let mut remaining = section.size;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 8192];
        while remaining != 0 {
            let size = remaining.min(buffer.len() as u64) as usize;
            reader.read_exact(&mut buffer[..size])?;
            digest.update(&buffer[..size]);
            remaining -= size as u64;
        }
        let hash = digest.finalize();
        if section.hash != u64::from_le_bytes(hash[..8].try_into().unwrap()) {
            return Err(MlibError::SectionChecksumMismatch(section.section_type as u32));
        }
    }
    Ok((header, sections))
}

impl LlibReader {
    pub fn read_manifest<R: Read + Seek>(reader: &mut R) -> Result<crate::format::Manifest, MlibError> {
        let (header, sections) = read_header_and_sections(reader)?;
        let (manifest, _) = read_native_envelope(reader, &header, &sections)?;
        manifest.ok_or(MlibError::CorruptedData)
    }

    pub fn read_object_code<R: Read + Seek>(reader: &mut R) -> Result<Option<Vec<u8>>, MlibError> {
        let (header, sections) = read_header_and_sections(reader)?;
        let (_, object) = read_native_envelope(reader, &header, &sections)?;
        Ok(object)
    }

    pub fn read_module<R: Read + Seek>(reader: &mut R) -> Result<(MlibModule, Option<crate::format::Manifest>, Option<Vec<u8>>, Option<crate::metadata::SemanticMetadata>), MlibError> {
        let (header, sections) = read_header_and_sections(reader)?;
        let (manifest_opt, obj_bytes_opt) = read_native_envelope(reader, &header, &sections)?;

        let mut raw_string_table = Vec::new();

        // Pass 1: String Table
        for section in &sections {
            if section.section_type == SectionType::StringTable {
                reader.seek(SeekFrom::Start(section.offset))?;
                let mut data = vec![0u8; section.size as usize];
                reader.read_exact(&mut data)?;
                raw_string_table = data;
            }
        }
        
        let mut mlib_module = MlibModule::default();
        let mut semantic_opt = None;
        
        // Helper to extract string by offset
        let get_string = |offset: u32| -> String {
            if offset as usize >= raw_string_table.len() {
                return String::new();
            }
            let slice = &raw_string_table[offset as usize..];
            let end = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
            String::from_utf8_lossy(&slice[..end]).into_owned()
        };

        // Pass 2: Metadata and MVIR
        for section in &sections {
            match section.section_type {
                SectionType::Manifest | SectionType::ObjectCode => {}, // Checked above.
                SectionType::SemanticMetadata => {
                    reader.seek(SeekFrom::Start(section.offset))?;
                    let mut data = vec![0u8; section.size as usize];
                    reader.read_exact(&mut data)?;
                    // `metadata_version` is the first field in the canonical
                    // bincode envelope. Check it before decoding the versioned
                    // payload so an older schema (which lacks required fields)
                    // is rejected explicitly rather than reported as generic
                    // corruption or silently defaulted.
                    check_semantic_metadata_version(&data)?;
                    let metadata: crate::metadata::SemanticMetadata =
                        bincode::deserialize(&data).map_err(|_| MlibError::CorruptedData)?;
                    validate_raw_storage_anchor_contracts(&metadata.interface)?;
                    validate_raw_pointer_effects(&metadata.interface)?;
                    validate_generic_contracts(&metadata.interface)?;
                    semantic_opt = Some(metadata);
                }
                SectionType::TypeMetadata => {
                    reader.seek(SeekFrom::Start(section.offset))?;
                    // Read TypeMetadata header/entries
                    // Wait, TypeMetadata section layout:
                    // uint32_t count;
                    // count * TypeEntry
                    
                    let mut count_buf = [0u8; 4];
                    reader.read_exact(&mut count_buf)?;
                    let count = u32::from_le_bytes(count_buf);
                    
                    for _ in 0..count {
                        let mut entry_buf = [0u8; 32];
                        reader.read_exact(&mut entry_buf)?;
                        
                        // TypeEntry layout:
                        // uint32_t nameStringID; 0-3
                        // uint32_t namespaceID; 4-7
                        // uint64_t size; 8-15
                        // uint64_t alignment; 16-23
                        // uint8_t  visibility; 24
                        // uint8_t pad[3]? wait, moduleID is uint32_t.
                        // Wait, C++ struct layout:
                        // uint32_t nameStringID (4)
                        // uint32_t namespaceID (4)
                        // uint64_t size (8)
                        // uint64_t alignment (8)
                        // uint8_t visibility (1)
                        // padding (3 bytes for alignment to 4) -> wait, moduleID is uint32_t.
                        // 1 + 3 padding + 4 = 8. Total = 4 + 4 + 8 + 8 + 8 = 32 bytes!
                        
                        let name_id = u32::from_le_bytes(entry_buf[0..4].try_into().unwrap());
                        let namespace_id = u32::from_le_bytes(entry_buf[4..8].try_into().unwrap());
                        let size = u64::from_le_bytes(entry_buf[8..16].try_into().unwrap());
                        let alignment = u64::from_le_bytes(entry_buf[16..24].try_into().unwrap());
                        let visibility = entry_buf[24];
                        let module_id = u32::from_le_bytes(entry_buf[28..32].try_into().unwrap());
                        
                        mlib_module.types.push(crate::ir::MlibTypeEntry {
                            name: get_string(name_id),
                            namespace_id,
                            size,
                            alignment,
                            visibility,
                            module_id,
                        });
                    }
                }
                SectionType::GenericMVIR => {
                    reader.seek(SeekFrom::Start(section.offset))?;
                    let mut data = vec![0u8; section.size as usize];
                    reader.read_exact(&mut data)?;
                    let mut cursor = std::io::Cursor::new(data);
                    let m = match Self::deserialize_module_internal(&mut cursor) {
                        Ok(m) => m,
                        Err(_) => return Err(MlibError::CorruptedData),
                    };
                    mlib_module.functions = m.functions;
                    if !m.strings.is_empty() {
                        mlib_module.strings = m.strings;
                    }
                    if mlib_module.types.is_empty() && !m.types.is_empty() {
                        mlib_module.types = m.types;
                    }
                }
                SectionType::StringTable => {} // Already parsed
                _ => {
                    // Unknown section, skip it
                }
            }
        }
        
        if let Some(metadata) = &semantic_opt {
            let manifest = manifest_opt.as_ref().ok_or(MlibError::CorruptedData)?;
            validate_nominal_layouts(&metadata.interface, &manifest.identity.module_id)?;
        }
        Ok((mlib_module, manifest_opt, obj_bytes_opt, semantic_opt))
    }

    fn read_string<R: Read>(r: &mut R) -> std::io::Result<String> {
        let mut len_buf = [0u8; 4];
        r.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut bytes = vec![0u8; len];
        r.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn deserialize_operand<R: Read>(r: &mut R) -> std::io::Result<MlibOperand> {
        let mut tag_buf = [0u8; 1];
        r.read_exact(&mut tag_buf)?;
        match tag_buf[0] {
            0 => {
                let mut v_buf = [0u8; 4];
                r.read_exact(&mut v_buf)?;
                Ok(MlibOperand::Value(u32::from_le_bytes(v_buf)))
            }
            1 => {
                let s = Self::read_string(r)?;
                Ok(MlibOperand::Number(s))
            }
            2 => {
                let mut b_buf = [0u8; 1];
                r.read_exact(&mut b_buf)?;
                Ok(MlibOperand::Boolean(b_buf[0] != 0))
            }
            3 => {
                let mut id_buf = [0u8; 4];
                r.read_exact(&mut id_buf)?;
                Ok(MlibOperand::Block(u32::from_le_bytes(id_buf)))
            }
            4 => {
                let s = Self::read_string(r)?;
                Ok(MlibOperand::Global(s))
            }
            5 => {
                let s = Self::read_string(r)?;
                Ok(MlibOperand::StringRef(s))
            }
            6 => {
                let s = Self::read_string(r)?;
                Ok(MlibOperand::Char(s))
            }
            7 => {
                let mut ty_buf = [0u8; 1];
                r.read_exact(&mut ty_buf)?;
                let ty = if ty_buf[0] == 0 {
                    crate::ir::MlibFloatType::F32
                } else {
                    crate::ir::MlibFloatType::F64
                };
                let text = Self::read_string(r)?;
                Ok(MlibOperand::Float { text, ty })
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid operand tag")),
        }
    }

    fn deserialize_instruction<R: Read>(r: &mut R) -> std::io::Result<MlibInstruction> {
        let mut tag_buf = [0u8; 1];
        r.read_exact(&mut tag_buf)?;
        match tag_buf[0] {
            0 => Ok(MlibInstruction::Alloca),
            0x20 => Ok(MlibInstruction::HeapAlloc),
            1 => {
                let op = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Assign(op))
            }
            2 => {
                let mut ptr_buf = [0u8; 4];
                r.read_exact(&mut ptr_buf)?;
                let ptr = u32::from_le_bytes(ptr_buf);
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Store { ptr, value })
            }
            0x28 => {
                let mut ptr_buf = [0u8; 4];
                r.read_exact(&mut ptr_buf)?;
                let ptr = u32::from_le_bytes(ptr_buf);
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::StoreAnchored { ptr, value })
            }
            3 => {
                let ptr = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Load { ptr })
            }
            0x71 => {
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Neg { value })
            }
            4 => {
                let callee = Self::read_string(r)?;
                let mut count_buf = [0u8; 4];
                r.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf) as usize;
                let mut args = Vec::with_capacity(count);
                for _ in 0..count {
                    args.push(Self::deserialize_operand(r)?);
                }
                Ok(MlibInstruction::CallDirect { callee, args })
            }
            28 => {
                let callee = Self::deserialize_operand(r)?;
                let mut count_buf = [0u8; 4];
                r.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf) as usize;
                let mut args = Vec::with_capacity(count);
                for _ in 0..count {
                    args.push(Self::deserialize_operand(r)?);
                }
                Ok(MlibInstruction::CallIndirect { callee, args })
            }
            29 => {
                let closure = Self::deserialize_operand(r)?;
                let mut count_buf = [0u8; 4];
                r.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf) as usize;
                let mut args = Vec::with_capacity(count);
                for _ in 0..count {
                    args.push(Self::deserialize_operand(r)?);
                }
                Ok(MlibInstruction::CallClosure { closure, args })
            }
            0x23 => {
                let mut len_buf = [0u8; 4];
                r.read_exact(&mut len_buf)?;
                let len = u32::from_le_bytes(len_buf) as usize;
                let mut name_buf = vec![0u8; len];
                r.read_exact(&mut name_buf)?;
                let func = String::from_utf8(name_buf).unwrap();
                let env_ptr = Self::deserialize_operand(r)?;
                let mut count_buf = [0u8; 4];
                r.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf) as usize;
                let mut captures = Vec::with_capacity(count);
                for _ in 0..count {
                    let mut symbol_buf = [0u8; 4];
                    let mut source_buf = [0u8; 4];
                    let mut field_buf = [0u8; 4];
                    let mut mode_buf = [0u8; 1];
                    let mut ty_buf = [0u8; 4];
                    let mut env_ty_buf = [0u8; 4];
                    r.read_exact(&mut symbol_buf)?;
                    r.read_exact(&mut source_buf)?;
                    r.read_exact(&mut field_buf)?;
                    r.read_exact(&mut mode_buf)?;
                    r.read_exact(&mut ty_buf)?;
                    r.read_exact(&mut env_ty_buf)?;
                    if mode_buf[0] > 2 {
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid closure capture mode"));
                    }
                    captures.push(MlibCaptureInfo {
                        symbol: u32::from_le_bytes(symbol_buf),
                        source: u32::from_le_bytes(source_buf),
                        env_field: u32::from_le_bytes(field_buf),
                        mode: mode_buf[0],
                        ty: u32::from_le_bytes(ty_buf),
                        env_ty: u32::from_le_bytes(env_ty_buf),
                    });
                }
                Ok(MlibInstruction::MakeClosure { func, env_ptr, captures })
            }
            0x70 => {
                let obj = Self::deserialize_operand(r)?;
                let mut idx_buf = [0u8; 4];
                r.read_exact(&mut idx_buf)?;
                let method_idx = u32::from_le_bytes(idx_buf);
                let mut count_buf = [0u8; 4];
                r.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf) as usize;
                let mut args = Vec::with_capacity(count);
                for _ in 0..count {
                    args.push(Self::deserialize_operand(r)?);
                }
                Ok(MlibInstruction::CallVirt { obj, method_idx, args })
            }
            0x25 => {
                let data_ptr = Self::deserialize_operand(r)?;
                let vtable = Self::read_string(r)?;
                let mut trait_buf = [0u8; 4];
                r.read_exact(&mut trait_buf)?;
                let trait_sym = u32::from_le_bytes(trait_buf);
                let mut concrete_buf = [0u8; 4];
                r.read_exact(&mut concrete_buf)?;
                let concrete_sym = u32::from_le_bytes(concrete_buf);
                Ok(MlibInstruction::MakeTraitObject { data_ptr, vtable, trait_sym, concrete_sym })
            }
            0x6E => {
                let data_ptr = Self::deserialize_operand(r)?;
                let len = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::MakeSlice { data_ptr, len })
            }
            0x6F => {
                let obj = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::DropVirt { obj })
            }
            5 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Add { left, right })
            }
            6 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Sub { left, right })
            }
            7 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Mul { left, right })
            }
            63 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Div { left, right })
            }
            64 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Rem { left, right })
            }
            8 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Eq { left, right })
            }
            42 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::NotEq { left, right })
            }
            17 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Eq { left, right })
            }
            22 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::LessThan { left, right })
            }
            65 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::LessOrEq { left, right })
            }
            66 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::GreaterThan { left, right })
            }
            67 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::GreaterOrEq { left, right })
            }
            68 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::BitAnd { left, right })
            }
            69 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::BitOr { left, right })
            }
            70 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::BitXor { left, right })
            }
            71 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Shl { left, right })
            }
            72 => {
                let left = Self::deserialize_operand(r)?;
                let right = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Shr { left, right })
            }
            9 => {
                let mut rw_buf = [0u8; 1];
                r.read_exact(&mut rw_buf)?;
                let base = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Borrow { is_rw: rw_buf[0] != 0, base })
            }
            10 => {
                let mut ty_buf = [0u8; 4];
                r.read_exact(&mut ty_buf)?;
                let enum_ty = u32::from_le_bytes(ty_buf);
                let mut idx_buf = [0u8; 4];
                r.read_exact(&mut idx_buf)?;
                let variant_idx = u32::from_le_bytes(idx_buf);
                let mut count_buf = [0u8; 4];
                r.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf) as usize;
                let mut args = Vec::with_capacity(count);
                for _ in 0..count {
                    args.push(Self::deserialize_operand(r)?);
                }
                Ok(MlibInstruction::Variant { enum_ty, variant_idx, args })
            }
            11 => {
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Tag { value })
            }
            12 => {
                let value = Self::deserialize_operand(r)?;
                let mut var_buf = [0u8; 4];
                r.read_exact(&mut var_buf)?;
                let mut field_buf = [0u8; 4];
                r.read_exact(&mut field_buf)?;
                Ok(MlibInstruction::Extract {
                    value,
                    variant_idx: u32::from_le_bytes(var_buf),
                    field_idx: u32::from_le_bytes(field_buf),
                })
            }
            27 => {
                let base = Self::deserialize_operand(r)?;
                let mut field_buf = [0u8; 4];
                r.read_exact(&mut field_buf)?;
                let mut name_tag = [0u8; 1];
                r.read_exact(&mut name_tag)?;
                let field_name = match name_tag[0] {
                    0 => None,
                    1 => Some(Self::read_string(r)?),
                    _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid field-name tag")),
                };
                Ok(MlibInstruction::FieldPtr {
                    base,
                    field_idx: u32::from_le_bytes(field_buf),
                    field_name,
                })
            }
            13 => {
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Drop { value })
            }
            15 => {
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::HeapFree { value })
            }
            18 => Ok(MlibInstruction::ListNew),
            0x21 => {
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::MarkInit { value })
            }
            23 => {
                let mut ty_buf = [0u8; 4];
                r.read_exact(&mut ty_buf)?;
                Ok(MlibInstruction::SizeOf { ty: u32::from_le_bytes(ty_buf) })
            }
            24 => {
                let mut ty_buf = [0u8; 4];
                r.read_exact(&mut ty_buf)?;
                Ok(MlibInstruction::AlignOf { ty: u32::from_le_bytes(ty_buf) })
            }
            0x22 => {
                let mut ty_buf = [0u8; 4];
                r.read_exact(&mut ty_buf)?;
                Ok(MlibInstruction::Null { ty: u32::from_le_bytes(ty_buf) })
            }
            0x24 => Ok(MlibInstruction::Nop),
            25 => {
                let value = Self::deserialize_operand(r)?;
                let mut ty_buf = [0u8; 4];
                r.read_exact(&mut ty_buf)?;
                Ok(MlibInstruction::Cast { value, ty: u32::from_le_bytes(ty_buf) })
            }
            26 => {
                let base = Self::deserialize_operand(r)?;
                let offset = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::PtrOffset { base, offset })
            }
            19 => {
                let list = Self::deserialize_operand(r)?;
                let value = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::ListPush { list, value })
            }
            20 => {
                let list = Self::deserialize_operand(r)?;
                let index = Self::deserialize_operand(r)?;
                let mut mut_buf = [0u8; 1];
                r.read_exact(&mut mut_buf)?;
                Ok(MlibInstruction::ListGet { list, index, is_mut: mut_buf[0] != 0 })
            }
            0x26 => {
                let future = Self::deserialize_operand(r)?;
                Ok(MlibInstruction::Await { future })
            }
            0x29 => {
                use bincode::Options;
                let mut size = [0; 4];
                r.read_exact(&mut size)?;
                let size = u32::from_le_bytes(size) as u64;
                let mut payload = Vec::new();
                let mut limited = r.take(size);
                limited.read_to_end(&mut payload)?;
                if payload.len() as u64 != size {
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "truncated static initializer"));
                }
                let data = bincode::DefaultOptions::new().with_fixint_encoding().with_limit(size)
                    .reject_trailing_bytes().deserialize(&payload)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                Ok(MlibInstruction::StaticAddress(data))
            }
            _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Unknown instruction opcode {}", tag_buf[0]))),
        }
    }

    fn deserialize_terminator<R: Read>(r: &mut R) -> std::io::Result<MlibTerminator> {
        let mut tag_buf = [0u8; 1];
        r.read_exact(&mut tag_buf)?;
        match tag_buf[0] {
            0 => {
                let mut target_buf = [0u8; 4];
                r.read_exact(&mut target_buf)?;
                Ok(MlibTerminator::Br { target: u32::from_le_bytes(target_buf) })
            }
            1 => {
                let condition = Self::deserialize_operand(r)?;
                let mut t_buf = [0u8; 4];
                r.read_exact(&mut t_buf)?;
                let true_target = u32::from_le_bytes(t_buf);
                let mut f_buf = [0u8; 4];
                r.read_exact(&mut f_buf)?;
                let false_target = u32::from_le_bytes(f_buf);
                Ok(MlibTerminator::CondBr { condition, true_target, false_target })
            }
            2 => {
                let mut has_val_buf = [0u8; 1];
                r.read_exact(&mut has_val_buf)?;
                let value = if has_val_buf[0] != 0 {
                    Some(Self::deserialize_operand(r)?)
                } else {
                    None
                };
                Ok(MlibTerminator::Ret { value })
            }
            4 => Ok(MlibTerminator::Unreachable),
            5 => Ok(MlibTerminator::MissingReturn),
            _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid terminator tag")),
        }
    }

    fn deserialize_block<R: Read>(r: &mut R) -> std::io::Result<MlibBlock> {
        let mut id_buf = [0u8; 4];
        r.read_exact(&mut id_buf)?;
        let id = u32::from_le_bytes(id_buf);
        let label = Self::read_string(r)?;
        let mut insts_count_buf = [0u8; 4];
        r.read_exact(&mut insts_count_buf)?;
        let insts_count = u32::from_le_bytes(insts_count_buf) as usize;
        let mut insts = Vec::with_capacity(insts_count);
        for _ in 0..insts_count {
            let mut inst_id_buf = [0u8; 4];
            r.read_exact(&mut inst_id_buf)?;
            insts.push(u32::from_le_bytes(inst_id_buf));
        }
        let mut has_term_buf = [0u8; 1];
        r.read_exact(&mut has_term_buf)?;
        let terminator = if has_term_buf[0] != 0 {
            Some(Self::deserialize_terminator(r)?)
        } else {
            None
        };
        Ok(MlibBlock { id, label, insts, terminator })
    }

    fn deserialize_value<R: Read>(r: &mut R) -> std::io::Result<MlibValue> {
        let mut id_buf = [0u8; 4];
        r.read_exact(&mut id_buf)?;
        let id = u32::from_le_bytes(id_buf);
        let inst = Self::deserialize_instruction(r)?;
        Ok(MlibValue { id, inst })
    }

    fn deserialize_function<R: Read>(r: &mut R) -> std::io::Result<MlibFunction> {
        let name = Self::read_string(r)?;
        let mut arg_count_buf = [0u8; 4];
        r.read_exact(&mut arg_count_buf)?;
        let arg_count = u32::from_le_bytes(arg_count_buf);
        let mut is_async_buf = [0u8; 1];
        r.read_exact(&mut is_async_buf)?;
        let is_async = is_async_buf[0] != 0;
        let mut val_count_buf = [0u8; 4];
        r.read_exact(&mut val_count_buf)?;
        let val_count = u32::from_le_bytes(val_count_buf) as usize;
        let mut values = Vec::with_capacity(val_count);
        for _ in 0..val_count {
            values.push(Self::deserialize_value(r)?);
        }
        let mut block_count_buf = [0u8; 4];
        r.read_exact(&mut block_count_buf)?;
        let block_count = u32::from_le_bytes(block_count_buf) as usize;
        let mut blocks = Vec::with_capacity(block_count);
        for _ in 0..block_count {
            blocks.push(Self::deserialize_block(r)?);
        }
        Ok(MlibFunction { name, arg_count, is_async, values, blocks })
    }

    fn deserialize_type_entry<R: Read>(r: &mut R) -> std::io::Result<MlibTypeEntry> {
        let name = Self::read_string(r)?;
        let mut ns_buf = [0u8; 4];
        r.read_exact(&mut ns_buf)?;
        let namespace_id = u32::from_le_bytes(ns_buf);
        let mut sz_buf = [0u8; 8];
        r.read_exact(&mut sz_buf)?;
        let size = u64::from_le_bytes(sz_buf);
        let mut align_buf = [0u8; 8];
        r.read_exact(&mut align_buf)?;
        let alignment = u64::from_le_bytes(align_buf);
        let mut vis_buf = [0u8; 1];
        r.read_exact(&mut vis_buf)?;
        let visibility = vis_buf[0];
        let mut mod_buf = [0u8; 4];
        r.read_exact(&mut mod_buf)?;
        let module_id = u32::from_le_bytes(mod_buf);
        Ok(MlibTypeEntry { name, namespace_id, size, alignment, visibility, module_id })
    }

    fn deserialize_module_internal<R: Read>(r: &mut R) -> std::io::Result<MlibModule> {
        let mut func_count_buf = [0u8; 4];
        r.read_exact(&mut func_count_buf)?;
        let func_count = u32::from_le_bytes(func_count_buf) as usize;
        let mut functions = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            functions.push(Self::deserialize_function(r)?);
        }
        let mut str_count_buf = [0u8; 4];
        r.read_exact(&mut str_count_buf)?;
        let str_count = u32::from_le_bytes(str_count_buf) as usize;
        let mut strings = Vec::with_capacity(str_count);
        for _ in 0..str_count {
            strings.push(Self::read_string(r)?);
        }
        let mut ty_count_buf = [0u8; 4];
        r.read_exact(&mut ty_count_buf)?;
        let ty_count = u32::from_le_bytes(ty_count_buf) as usize;
        let mut types = Vec::with_capacity(ty_count);
        for _ in 0..ty_count {
            types.push(Self::deserialize_type_entry(r)?);
        }
        Ok(MlibModule { functions, strings, types })
    }

    pub fn read_ast_interface<R: Read + Seek>(reader: &mut R) -> Result<Option<(luna_ast::AstArena, Vec<luna_ast::Item>, String)>, MlibError> {
        let (header, sections) = read_header_and_sections(reader)?;
        read_native_envelope(reader, &header, &sections)?;

        for section in &sections {
            if section.section_type == SectionType::AstInterface {
                reader.seek(SeekFrom::Start(section.offset))?;
                let mut data = vec![0u8; section.size as usize];
                reader.read_exact(&mut data)?;
                
                let result = bincode::deserialize::<(luna_ast::AstArena, Vec<luna_ast::Item>, String)>(&data)
                    .map_err(|e| MlibError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))?;
                return Ok(Some(result));
            }
        }
        
        Ok(None)
    }
}

#[cfg(test)]
mod semantic_metadata_version_tests {
    use super::check_semantic_metadata_version;
    use crate::format::SEMANTIC_METADATA_VERSION;
    use crate::MlibError;

    #[test]
    fn static_initializer_rejects_truncation_and_trailing_bytes() {
        let data = luna_mvir::static_data::StaticData {
            name: "constant".into(), ty: luna_mvir::static_data::StaticType::Primitive(luna_semantic::BuiltinType::I32),
            value: luna_mvir::static_data::StaticValue::Int(20),
        };
        let valid = bincode::serialize(&data).unwrap();
        for payload in [&valid[..valid.len() - 1], &[valid.as_slice(), &[0u8]].concat()[..]] {
            let mut bytes = vec![0x29];
            bytes.extend((payload.len() as u32).to_le_bytes());
            bytes.extend(payload);
            assert!(super::LlibReader::deserialize_instruction(&mut std::io::Cursor::new(bytes)).is_err());
        }
    }

    #[test]
    fn older_mvir_is_rejected_before_static_data_decode() {
        for version in 1..crate::format::LLIB_MVIR_VERSION {
            let mut header = crate::format::LlibHeader::new();
            header.mvir_version = version;
            assert!(matches!(super::validate_header_versions(&header), Err(MlibError::VersionMismatch(v)) if v == version));
        }
    }

    #[test]
    fn older_compiler_artifacts_are_rejected_before_payload_decode() {
        for old_version in 1..crate::format::LLIB_COMPILER_VERSION {
            let mut header = crate::format::LlibHeader::new();
            header.compiler_version = old_version;
            let mut bytes = Vec::new();
            header.write_to(&mut bytes).unwrap();
            let mut reader = std::io::Cursor::new(bytes);
            assert!(matches!(super::LlibReader::read_manifest(&mut reader),
                Err(MlibError::VersionMismatch(version)) if version == header.compiler_version));
        }
    }

    #[test]
    fn old_semantic_metadata_is_rejected_before_payload_decode() {
        let old_version = SEMANTIC_METADATA_VERSION - 1;
        let bytes = old_version.to_le_bytes();
        assert!(matches!(
            check_semantic_metadata_version(&bytes),
            Err(MlibError::VersionMismatch(version)) if version == old_version
        ));
    }

    #[test]
    fn truncated_semantic_metadata_version_is_corrupt() {
        assert!(matches!(
            check_semantic_metadata_version(&[0]),
            Err(MlibError::CorruptedData)
        ));
    }
}

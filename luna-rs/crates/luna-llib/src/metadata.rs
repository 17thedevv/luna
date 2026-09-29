use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

use luna_semantic::ty::{BuiltinType, Mutability};
use crate::format::{InterfaceFingerprint, Fingerprint};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMetadata {
    pub metadata_version: u16,        // Must match SEMANTIC_METADATA_VERSION
    pub language_version: u16,        // Core language version compatibility
    pub target_triple: String,        // Target architecture
    pub interface_fingerprint: InterfaceFingerprint,
    pub interface: CanonicalInterface,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct StableSymbolId {
    pub provider_name: String,
    pub symbol_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CanonicalLifetime {
    Static,
    Named(String),
    Anonymous,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CanonicalType {
    Primitive(BuiltinType),
    Struct(StableSymbolId, Vec<u32>, Vec<u32>), // trait bounds and generic arguments as type indices
    Enum(StableSymbolId, Vec<u32>, Vec<u32>),
    Tuple(Vec<u32>),
    Array(u32, u64),
    Slice(u32),
    Function { params: Vec<u32>, return_type: u32 },
    Pointer(Mutability, u32),
    Reference(CanonicalLifetime, Mutability, u32),
    Void,
    Never,
    Error,
    GenericParam(StableSymbolId),
    Closure(u64, Vec<u32>, u32), // unique id, captures, return type
    DynTrait(StableSymbolId),
    Future(u32),
    Range(u32),
    Projection { self_type: u32, trait_id: StableSymbolId, assoc_type: StableSymbolId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalInterface {
    /// Exported symbols - using BTreeMap for deterministic iteration order.
    pub exported_symbols: BTreeMap<String, ExportedSymbol>,
    pub types: Vec<CanonicalType>, // Deduplicated type registry for this ABI
    /// Trait definitions - using BTreeMap for deterministic iteration order.
    pub traits: BTreeMap<StableSymbolId, TraitDefinition>,
    pub impl_headers: Vec<ImplHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSymbol {
    pub kind: String, // "Function", "Struct", etc.
    pub ty_index: Option<u32>, // Index into CanonicalInterface::types, None for modules
    pub visibility: u8,
    pub generic_params: Vec<StableSymbolId>,
    pub symbol_id: StableSymbolId,
    #[serde(default)]
    pub children: BTreeMap<String, ExportedSymbol>, // BTreeMap for deterministic ordering
    #[serde(default)]
    pub lifetime_contract: Option<luna_semantic::CanonicalLifetimeContract>,
    #[serde(default)]
    pub type_lifetime_contract: Option<luna_semantic::CanonicalTypeLifetimeContract>,
    pub raw_storage_anchor_contract: Option<luna_semantic::CanonicalRawStorageAnchorContract>,
    /// Stable, parameter-relative raw-pointer effects inferred from a
    /// non-generic function body. These are separate from safe lifetime
    /// contracts and contain no compiler-session identities.
    pub raw_pointer_effects: Option<CanonicalRawPointerEffects>,
    #[serde(default)]
    pub is_unsafe: bool,
}

/// Canonical raw-pointer effect summary exported across an artifact boundary.
/// Parameter indices and canonical field names are stable semantic identities;
/// ValueId/PlaceId/DeclId and borrowck implementation state are never stored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalRawPointerEffects {
    pub returned: CanonicalRawPointerEffect,
    pub direct_fields: BTreeMap<String, CanonicalRawPointerEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalRawPointerEffect {
    pub origin: CanonicalRawPointerOrigin,
    pub anchor: CanonicalRawPointerAnchor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalRawPointerOrigin {
    Independent,
    FromParameters(Vec<u32>),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CanonicalRawPointerAnchor {
    Independent,
    From(Vec<CanonicalRawPointerAnchorSource>),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalRawPointerAnchorSource {
    RawParameter(u32),
    OwnerField { parameter: u32, field_name: String },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDefinition {
    pub name: String,
    pub methods: BTreeMap<String, u32>, // BTreeMap for deterministic ordering
    pub associated_types: Vec<StableSymbolId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplHeader {
    pub trait_id: Option<StableSymbolId>, // None for inherent impls
    pub self_type: u32,
    #[serde(default)]
    pub generic_params: Vec<StableSymbolId>,
    #[serde(default)]
    pub trait_args: Vec<u32>,
    pub methods: BTreeMap<String, u32>, // Method name -> Type index
}


#[cfg(test)]
mod raw_storage_anchor_interface_tests {
    use super::*;

    fn interface(anchor: Option<luna_semantic::CanonicalRawStorageAnchorContract>) -> CanonicalInterface {
        let owner_id = StableSymbolId {
            provider_name: "test_provider".into(),
            symbol_path: "RawOwner".into(),
        };
        let owner = ExportedSymbol {
            kind: "Struct".into(),
            ty_index: None,
            visibility: 1,
            generic_params: Vec::new(),
            symbol_id: owner_id,
            children: BTreeMap::new(),
            lifetime_contract: None,
            type_lifetime_contract: None,
            raw_storage_anchor_contract: anchor,
            raw_pointer_effects: None,
            is_unsafe: false,
        };
        CanonicalInterface {
            exported_symbols: BTreeMap::from([("RawOwner".into(), owner)]),
            types: Vec::new(),
            traits: BTreeMap::new(),
            impl_headers: Vec::new(),
        }
    }

    #[test]
    fn anchor_contract_is_owned_by_type_identity_and_changes_interface_fingerprint() {
        use sha2::{Digest, Sha256};

        let without_anchor = interface(None);
        let with_anchor = interface(Some(
            luna_semantic::CanonicalRawStorageAnchorContract::new(vec!["data".into()]),
        ));
        let owner = with_anchor.exported_symbols.get("RawOwner").unwrap();
        assert_eq!(owner.kind, "Struct");
        assert_eq!(owner.symbol_id.symbol_path, "RawOwner");
        assert_eq!(owner.raw_storage_anchor_contract.as_ref().unwrap().field_names, ["data"]);

        let fingerprint = |iface: &CanonicalInterface| {
            let bytes = bincode::serialize(iface).unwrap();
            Sha256::digest(bytes)
        };
        assert_ne!(fingerprint(&without_anchor), fingerprint(&with_anchor));
    }
}

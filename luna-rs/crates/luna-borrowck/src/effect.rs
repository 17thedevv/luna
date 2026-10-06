use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessKind {
    None,
    Read,
    Write,
    ReadWrite,
    Unknown,
}

impl PartialOrd for AccessKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            return Some(std::cmp::Ordering::Equal);
        }
        match (self, other) {
            (AccessKind::None, _) => Some(std::cmp::Ordering::Less),
            (_, AccessKind::Unknown) => Some(std::cmp::Ordering::Less),
            (AccessKind::Unknown, _) => Some(std::cmp::Ordering::Greater),
            (_, AccessKind::None) => Some(std::cmp::Ordering::Greater),
            (AccessKind::Read, AccessKind::ReadWrite) => Some(std::cmp::Ordering::Less),
            (AccessKind::Write, AccessKind::ReadWrite) => Some(std::cmp::Ordering::Less),
            (AccessKind::ReadWrite, AccessKind::Read) => Some(std::cmp::Ordering::Greater),
            (AccessKind::ReadWrite, AccessKind::Write) => Some(std::cmp::Ordering::Greater),
            _ => None,
        }
    }
}

impl AccessKind {
    pub fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (AccessKind::Unknown, _) | (_, AccessKind::Unknown) => AccessKind::Unknown,
            (AccessKind::ReadWrite, _) | (_, AccessKind::ReadWrite) => AccessKind::ReadWrite,
            (AccessKind::Read, AccessKind::Write) | (AccessKind::Write, AccessKind::Read) => {
                AccessKind::ReadWrite
            }
            (AccessKind::Read, _) | (_, AccessKind::Read) => AccessKind::Read,
            (AccessKind::Write, _) | (_, AccessKind::Write) => AccessKind::Write,
            (AccessKind::None, AccessKind::None) => AccessKind::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum OwnershipKind {
    Copy,
    BorrowShared,
    BorrowMut,
    Consume,
    Unknown,
}

impl OwnershipKind {
    pub fn merge(&self, other: &Self) -> Self {
        std::cmp::max(self, other).clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EscapeKind {
    NoEscape,
    CallOnly,
    MayEscape,
    Unknown,
}

impl EscapeKind {
    pub fn merge(&self, other: &Self) -> Self {
        std::cmp::max(self, other).clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnEffect {
    Independent,
    BorrowsFrom(Vec<usize>), // Arg indices
    BorrowsCarried(Vec<usize>),
    /// Preserve formal-storage and carried-loan sources from all return paths.
    BorrowsBoth { direct: Vec<usize>, carried: Vec<usize> },
    Unknown,
}

/// Origin relationship for a returned raw pointer. This is intentionally
/// separate from `ReturnEffect`: raw pointer origin is not a live safe loan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawPointerReturnEffect {
    Independent,
    From(Vec<usize>),
    Unknown,
}

impl RawPointerReturnEffect {
    pub fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Independent, Self::Independent) => Self::Independent,
            (Self::From(a), Self::From(b)) => {
                let mut indices = a.clone();
                for index in b {
                    if !indices.contains(index) {
                        indices.push(*index);
                    }
                }
                indices.sort_unstable();
                Self::From(indices)
            }
            // A returned pointer that is independent on one path and derives
            // from an argument on another has no single parameter-relative
            // origin that callers may rely on.
            (Self::Independent, Self::From(_)) | (Self::From(_), Self::Independent) => Self::Unknown,
        }
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        if self == other {
            return Some(Ordering::Equal);
        }
        match (self, other) {
            (Self::Independent, _) => Some(Ordering::Less),
            (_, Self::Independent) => Some(Ordering::Greater),
            (Self::Unknown, _) => Some(Ordering::Greater),
            (_, Self::Unknown) => Some(Ordering::Less),
            (Self::From(a), Self::From(b)) => {
                let a_subset = a.iter().all(|index| b.contains(index));
                let b_subset = b.iter().all(|index| a.contains(index));
                match (a_subset, b_subset) {
                    (true, true) => Some(Ordering::Equal),
                    (true, false) => Some(Ordering::Less),
                    (false, true) => Some(Ordering::Greater),
                    (false, false) => None,
                }
            }
        }
    }
}

/// Provenance relation for the logical owner anchor of a returned raw pointer.
/// This is intentionally separate from `RawPointerReturnEffect`: the raw
/// address may be unknown while a declared owner/field invariant establishes
/// that its validity is governed by a particular owner.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RawPointerAnchorSource {
    /// The result preserves the anchor attached to a raw-pointer argument.
    RawParam(usize),
    /// The result was loaded from this contracted field of an owner argument.
    OwnerField { param: usize, field: String },
    /// A valid local anchor exists but cannot be expressed relative to a
    /// function parameter (or the analysis cannot prove one).
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawPointerAnchorReturnEffect {
    /// The result has no owner anchor (e.g. an independent raw allocation).
    Independent,
    /// The result's anchor is derived from one or more formal parameters.
    From(Vec<RawPointerAnchorSource>),
    /// Analysis found an untracked, mixed, or otherwise non-portable anchor.
    Unknown,
}

impl RawPointerAnchorReturnEffect {
    pub fn merge(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Independent, Self::Independent) => Self::Independent,
            (Self::From(a), Self::From(b)) => {
                let mut sources = a.clone();
                sources.extend(b.iter().cloned());
                sources.sort_unstable();
                sources.dedup();
                Self::From(sources)
            }
            // A path with no owner anchor invalidates a must-anchor summary.
            (Self::Independent, Self::From(_)) | (Self::From(_), Self::Independent) => Self::Unknown,
        }
    }

    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        if self == other {
            return Some(Ordering::Equal);
        }
        match (self, other) {
            (Self::Independent, _) => Some(Ordering::Less),
            (_, Self::Independent) => Some(Ordering::Greater),
            (Self::Unknown, _) => Some(Ordering::Greater),
            (_, Self::Unknown) => Some(Ordering::Less),
            (Self::From(a), Self::From(b)) => {
                let a_subset = a.iter().all(|source| b.contains(source));
                let b_subset = b.iter().all(|source| a.contains(source));
                match (a_subset, b_subset) {
                    (true, true) => Some(Ordering::Equal),
                    (true, false) => Some(Ordering::Less),
                    (false, true) => Some(Ordering::Greater),
                    (false, false) => None,
                }
            }
        }
    }
}

impl PartialOrd for ReturnEffect {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering;
        if self == other { return Some(Ordering::Equal); }
        match (self, other) {
            (Self::Independent, _) | (_, Self::Unknown) => Some(Ordering::Less),
            (_, Self::Independent) | (Self::Unknown, _) => Some(Ordering::Greater),
            _ => {
                let (ad, ac) = self.sources();
                let (bd, bc) = other.sources();
                let a_subset = ad.iter().all(|i| bd.contains(i)) && ac.iter().all(|i| bc.contains(i));
                let b_subset = bd.iter().all(|i| ad.contains(i)) && bc.iter().all(|i| ac.contains(i));
                match (a_subset, b_subset) {
                    (true, true) => Some(Ordering::Equal),
                    (true, false) => Some(Ordering::Less),
                    (false, true) => Some(Ordering::Greater),
                    (false, false) => None,
                }
            }
        }
    }
}

impl ReturnEffect {
    pub fn sources(&self) -> (&[usize], &[usize]) {
        match self {
            Self::BorrowsFrom(indices) => (indices, &[]),
            Self::BorrowsCarried(indices) => (&[], indices),
            Self::BorrowsBoth { direct, carried } => (direct, carried),
            Self::Independent | Self::Unknown => (&[], &[]),
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        if matches!(self, Self::Unknown) || matches!(other, Self::Unknown) { return Self::Unknown; }
        let (ad, ac) = self.sources();
        let (bd, bc) = other.sources();
        let mut direct = ad.iter().chain(bd).copied().collect::<Vec<_>>();
        let mut carried = ac.iter().chain(bc).copied().collect::<Vec<_>>();
        direct.sort_unstable(); direct.dedup();
        carried.sort_unstable(); carried.dedup();
        match (direct.is_empty(), carried.is_empty()) {
            (true, true) => Self::Independent,
            (false, true) => Self::BorrowsFrom(direct),
            (true, false) => Self::BorrowsCarried(carried),
            (false, false) => Self::BorrowsBoth { direct, carried },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgEffect {
    pub access: AccessKind,
    pub ownership: OwnershipKind,
    pub escape: EscapeKind,
}

impl PartialOrd for ArgEffect {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let access_cmp = self.access.partial_cmp(&other.access);
        let ownership_cmp = self.ownership.partial_cmp(&other.ownership);
        let escape_cmp = self.escape.partial_cmp(&other.escape);

        let cmps = [access_cmp, ownership_cmp, escape_cmp];

        let mut has_less = false;
        let mut has_greater = false;

        for cmp in cmps {
            match cmp {
                Some(std::cmp::Ordering::Less) => has_less = true,
                Some(std::cmp::Ordering::Greater) => has_greater = true,
                None => return None,
                _ => {}
            }
        }

        if has_less && has_greater {
            None
        } else if has_less {
            Some(std::cmp::Ordering::Less)
        } else if has_greater {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl ArgEffect {
    pub fn default() -> Self {
        Self {
            access: AccessKind::None,
            ownership: OwnershipKind::Copy,
            escape: EscapeKind::CallOnly,
        }
    }

    pub fn worst_case() -> Self {
        Self {
            access: AccessKind::Unknown,
            ownership: OwnershipKind::Unknown,
            escape: EscapeKind::Unknown,
        }
    }

    pub fn merge(&self, other: &Self) -> Self {
        Self {
            access: self.access.merge(&other.access),
            ownership: self.ownership.merge(&other.ownership),
            escape: self.escape.merge(&other.escape),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallEffectSummary {
    /// ABI-relative obligations inferred for generated default entry bodies.
    pub input_outlives: std::collections::BTreeSet<(u16, u16)>,
    pub args: Vec<ArgEffect>,
    pub ret: ReturnEffect,
    pub raw_pointer_ret: RawPointerReturnEffect,
    pub raw_pointer_anchor_ret: RawPointerAnchorReturnEffect,
    /// Direct raw-pointer fields of an aggregate result. Keys are canonical
    /// source field names, never function-local place/value identities.
    pub raw_pointer_field_ret: std::collections::BTreeMap<String, RawPointerFieldReturnEffect>,
    pub is_opaque: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPointerFieldReturnEffect {
    pub origin: RawPointerReturnEffect,
    pub anchor: RawPointerAnchorReturnEffect,
}

impl RawPointerFieldReturnEffect {
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            origin: self.origin.merge(&other.origin),
            anchor: self.anchor.merge(&other.anchor),
        }
    }
}

/// Only direct fields are part of RAW-STORAGE-ANCHOR-v1. Resolve field names
/// through semantic symbols so summaries remain independent of field ordinals.
pub(crate) fn direct_raw_pointer_fields(
    ctx: &luna_semantic::SemanticContext,
    ty: luna_semantic::SemanticTypeId,
) -> Vec<(u32, String)> {
    let luna_semantic::SemanticType::Struct(symbol, _, field_types) =
        ctx.types.get(ctx.types.resolve(ty)) else { return Vec::new(); };
    let Some(field_names) = ctx.tables.struct_field_names.get(symbol) else { return Vec::new(); };
    field_types.iter().zip(field_names).enumerate().filter_map(|(index, (field_ty, field_name))| {
        if matches!(ctx.types.get(ctx.types.resolve(*field_ty)), luna_semantic::SemanticType::Pointer(..)) {
            Some((index as u32, field_name.clone()))
        } else {
            None
        }
    }).collect()
}

impl PartialOrd for CallEffectSummary {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.args.len() != other.args.len() {
            return None;
        }

        let mut has_less = false;
        let mut has_greater = false;

        for (a, b) in self.args.iter().zip(other.args.iter()) {
            match a.partial_cmp(b) {
                Some(std::cmp::Ordering::Less) => has_less = true,
                Some(std::cmp::Ordering::Greater) => has_greater = true,
                None => return None,
                _ => {}
            }
        }

        match self.ret.partial_cmp(&other.ret) {
            Some(std::cmp::Ordering::Less) => has_less = true,
            Some(std::cmp::Ordering::Greater) => has_greater = true,
            None => return None,
            _ => {}
        }

        match self.raw_pointer_ret.partial_cmp(&other.raw_pointer_ret) {
            Some(std::cmp::Ordering::Less) => has_less = true,
            Some(std::cmp::Ordering::Greater) => has_greater = true,
            None => return None,
            _ => {}
        }

        match self.raw_pointer_anchor_ret.partial_cmp(&other.raw_pointer_anchor_ret) {
            Some(std::cmp::Ordering::Less) => has_less = true,
            Some(std::cmp::Ordering::Greater) => has_greater = true,
            None => return None,
            _ => {}
        }

        if self.input_outlives != other.input_outlives { return None; }
        if self.raw_pointer_field_ret != other.raw_pointer_field_ret {
            // Field-wise effects are a product lattice. The fixed-point engine
            // uses equality; do not invent an ordering across unrelated fields.
            return None;
        }

        if has_less && has_greater {
            None
        } else if has_less {
            Some(std::cmp::Ordering::Less)
        } else if has_greater {
            Some(std::cmp::Ordering::Greater)
        } else {
            Some(std::cmp::Ordering::Equal)
        }
    }
}

impl Default for CallEffectSummary {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            ret: ReturnEffect::Independent,
            raw_pointer_ret: RawPointerReturnEffect::Independent,
            raw_pointer_anchor_ret: RawPointerAnchorReturnEffect::Independent,
            raw_pointer_field_ret: std::collections::BTreeMap::new(),
            input_outlives: Default::default(),
            is_opaque: false,
        }
    }
}

impl CallEffectSummary {
    pub fn default_for_args(num_args: usize) -> Self {
        Self {
            args: vec![ArgEffect::default(); num_args],
            ret: ReturnEffect::Independent,
            raw_pointer_ret: RawPointerReturnEffect::Independent,
            raw_pointer_anchor_ret: RawPointerAnchorReturnEffect::Independent,
            raw_pointer_field_ret: std::collections::BTreeMap::new(),
            input_outlives: Default::default(),
            is_opaque: false,
        }
    }

    pub fn worst_case(num_args: usize) -> Self {
        Self {
            args: vec![ArgEffect::worst_case(); num_args],
            ret: ReturnEffect::Unknown,
            raw_pointer_ret: RawPointerReturnEffect::Unknown,
            raw_pointer_anchor_ret: RawPointerAnchorReturnEffect::Unknown,
            raw_pointer_field_ret: std::collections::BTreeMap::new(),
            input_outlives: Default::default(),
            is_opaque: true,
        }
    }
}

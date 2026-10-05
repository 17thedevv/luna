use crate::dataflow::{DataflowAnalysis, DataflowEngine};
use crate::effect::{
    direct_raw_pointer_fields, AccessKind, CallEffectSummary, EscapeKind, OwnershipKind,
    RawPointerAnchorReturnEffect, RawPointerAnchorSource, RawPointerFieldReturnEffect,
    RawPointerReturnEffect, ReturnEffect,
};
use crate::borrow_analysis::{compute_place_desc, PlaceDesc, Projection};
use luna_mvir::{ValueOrigin, Function, GlobalId, Instruction, Operand, Terminator, ValueId};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum TaintSource {
    Direct(usize),
    Carried(usize),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum RawPointerSource {
    SafeDirect(usize),
    SafeCarried(usize),
    RawDirect(usize),
    RawCarried(usize),
    Unknown,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct TaintState {
    /// DataflowEngine initializes non-entry blocks to `Default`; distinguish
    /// that bottom state from a real incoming state before applying must-fact
    /// joins for anchor provenance.
    initialized: bool,
    pub direct: HashMap<ValueId, HashSet<TaintSource>>,
    pub carried: HashMap<ValueId, HashSet<TaintSource>>,
    /// Provenance that represents a safe loan carried by a value or place.
    /// These sets are intentionally separate from generic data/raw-pointer
    /// taints above: safe references and validity-contracted borrowed headers
    /// may transport them; raw pointer fields do not originate safe loans.
    pub safe_direct: HashMap<ValueId, HashSet<TaintSource>>,
    pub safe_carried: HashMap<ValueId, HashSet<TaintSource>>,
    /// Safe-loan provenance attached to address computations. These tags are
    /// only consumed by a later typed Load or Borrow; the raw address value is
    /// not itself a safe-loan carrier and must not export a return effect.
    pub safe_place_direct: HashMap<ValueId, HashSet<TaintSource>>,
    pub safe_place_carried: HashMap<ValueId, HashSet<TaintSource>>,
    /// Raw pointer origin is tracked independently from safe-loan liveness.
    pub raw_direct: HashMap<ValueId, HashSet<RawPointerSource>>,
    pub raw_carried: HashMap<ValueId, HashSet<RawPointerSource>>,
    pub raw_place_direct: HashMap<ValueId, HashSet<RawPointerSource>>,
    pub raw_place_carried: HashMap<ValueId, HashSet<RawPointerSource>>,
    /// Raw-pointer VALUE origins stored in memory places. Kept separate from
    /// `raw_place_*`, which describe the origin of an address used to access
    /// a place and cannot establish the origin of the pointer value in it.
    pub raw_pointer_storage: HashMap<PlaceDesc, HashSet<RawPointerSource>>,
    /// Logical owner-anchor facts stored with raw pointer values. These never
    /// enter the safe-loan channel and are summarized relative to parameters.
    pub raw_pointer_anchor_direct: HashMap<ValueId, HashSet<RawPointerAnchorSource>>,
    pub raw_pointer_anchor_carried: HashMap<ValueId, HashSet<RawPointerAnchorSource>>,
    pub raw_pointer_anchor_storage: HashMap<PlaceDesc, HashSet<RawPointerAnchorSource>>,
    pub aliases: HashMap<ValueId, Operand>,
}

pub struct EffectInference<'a> {
    arg_count: usize,
    arg_values: Vec<ValueId>, // The ValueIds corresponding to the function arguments
    pub summary: CallEffectSummary,
    callee_summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
    func: &'a Function,
    ctx: Option<&'a luna_semantic::SemanticContext>,
    saw_raw_pointer_return: bool,
    saw_raw_pointer_anchor_return: bool,
}

impl<'a> EffectInference<'a> {
    pub fn new(
        func: &'a Function,
        arg_count: usize,
        arg_values: Vec<ValueId>,
        callee_summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
        ctx: Option<&'a luna_semantic::SemanticContext>,
    ) -> Self {
        Self {
            arg_count,
            arg_values,
            summary: CallEffectSummary::default_for_args(arg_count),
            callee_summaries,
            func,
            ctx,
            saw_raw_pointer_return: false,
            saw_raw_pointer_anchor_return: false,
        }
    }

    pub fn infer(
        func: &'a Function,
        arg_values: Vec<ValueId>,
        callee_summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
    ) -> CallEffectSummary {
        Self::infer_with_context(func, arg_values, callee_summaries, None)
    }

    pub fn infer_with_context(
        func: &'a Function,
        arg_values: Vec<ValueId>,
        callee_summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
        ctx: Option<&'a luna_semantic::SemanticContext>,
    ) -> CallEffectSummary {
        let mut analyzer = Self::new(func, arg_values.len(), arg_values, callee_summaries, ctx);
        let _ = DataflowEngine::run_forward(func, &mut analyzer);
        analyzer.summary
    }

    fn add_direct_taint(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if let Operand::Value(src_val) = src {
            if let Some(taints) = state.direct.get(src_val).cloned() {
                state.direct.entry(dest).or_default().extend(taints);
            }
        }
    }

    fn add_carried_taint(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if let Operand::Value(src_val) = src {
            if let Some(taints) = state.carried.get(src_val).cloned() {
                state.carried.entry(dest).or_default().extend(taints);
            }
        }
    }

    fn get_direct_taints(&self, state: &TaintState, op: &Operand) -> HashSet<TaintSource> {
        let mut taints = HashSet::new();
        if let Operand::Value(val) = op {
            if let Some(t) = state.direct.get(val) {
                taints.extend(t.iter().copied());
            }
        }
        taints
    }

    fn get_carried_taints(&self, state: &TaintState, op: &Operand) -> HashSet<TaintSource> {
        let mut taints = HashSet::new();
        if let Operand::Value(val) = op {
            if let Some(t) = state.carried.get(val) {
                taints.extend(t.iter().copied());
            }
        }
        taints
    }

    fn add_safe_direct_taint(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if let Operand::Value(src_val) = src {
            if let Some(taints) = state.safe_direct.get(src_val).cloned() {
                state.safe_direct.entry(dest).or_default().extend(taints);
            }
        }
    }

    fn add_safe_carried_taint(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if let Operand::Value(src_val) = src {
            if let Some(taints) = state.safe_carried.get(src_val).cloned() {
                state.safe_carried.entry(dest).or_default().extend(taints);
            }
        }
    }

    fn get_safe_direct_taints(&self, state: &TaintState, op: &Operand) -> HashSet<TaintSource> {
        let mut taints = HashSet::new();
        if let Operand::Value(val) = op {
            if let Some(found) = state.safe_direct.get(val) {
                taints.extend(found.iter().copied());
            }
        }
        taints
    }

    fn get_safe_carried_taints(&self, state: &TaintState, op: &Operand) -> HashSet<TaintSource> {
        let mut taints = HashSet::new();
        if let Operand::Value(val) = op {
            if let Some(found) = state.safe_carried.get(val) {
                taints.extend(found.iter().copied());
            }
        }
        taints
    }

    fn can_carry_safe_loan(&self, ty: luna_semantic::SemanticTypeId) -> bool {
        self.ctx
            .map(|ctx| ctx.may_carry_validity_borrow(ty))
            // Context-free tests retain their conservative pre-contract model.
            .unwrap_or(true)
    }

    fn is_raw_pointer(&self, ty: luna_semantic::SemanticTypeId) -> bool {
        self.ctx
            .map(|ctx| matches!(ctx.types.get(ctx.types.resolve(ty)), luna_semantic::SemanticType::Pointer(..)))
            .unwrap_or(false)
    }

    fn record_raw_pointee_access(&mut self, pointer: &Operand, access: &AccessKind, state: &TaintState) {
        let Operand::Value(value) = pointer else { return; };
        if !self.is_raw_pointer(self.func.value(*value).ty)
            || matches!(self.func.value(*value).inst, Instruction::Alloca | Instruction::FieldPtr { .. })
        { return; }
        // Raw VALUE provenance identifies the parameter whose pointee is
        // accessed. Generic data taint from loading the parameter slot has a
        // different meaning and must not erase a subsequent pointee write.
        for source in self.get_raw_direct_taints(state, pointer).into_iter().chain(self.get_raw_carried_taints(state, pointer)) {
            let index = match source {
                RawPointerSource::RawDirect(index) | RawPointerSource::RawCarried(index)
                    | RawPointerSource::SafeDirect(index) | RawPointerSource::SafeCarried(index) => index,
                RawPointerSource::Unknown => continue,
            };
            if let Some(parameter) = self.summary.args.get_mut(index) {
                parameter.access = parameter.access.merge(access);
            }
        }
    }

    fn add_safe_value_provenance(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if !self.can_carry_safe_loan(self.func.value(dest).ty) {
            return;
        }
        self.add_safe_direct_taint(state, dest, src);
        self.add_safe_carried_taint(state, dest, src);
    }

    fn add_safe_place_provenance(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        // Address computations are not safe-loan carriers themselves. These
        // tags only describe which safe-reference provenance a later Borrow or
        // typed Load may recover from the addressed place.
        if let Operand::Value(src_val) = src {
            if let Some(taints) = state.safe_direct.get(src_val).cloned() {
                state.safe_place_direct.entry(dest).or_default().extend(taints);
            }
            if let Some(taints) = state.safe_carried.get(src_val).cloned() {
                state.safe_place_carried.entry(dest).or_default().extend(taints);
            }
            if let Some(taints) = state.safe_place_direct.get(src_val).cloned() {
                state.safe_place_direct.entry(dest).or_default().extend(taints);
            }
            if let Some(taints) = state.safe_place_carried.get(src_val).cloned() {
                state.safe_place_carried.entry(dest).or_default().extend(taints);
            }
        }
    }

    fn add_raw_place_provenance(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if let Operand::Value(src_val) = src {
            let mut direct = HashSet::new();
            for taint in state.safe_direct.get(src_val).into_iter().flatten() {
                direct.insert(match taint {
                    TaintSource::Direct(index) => RawPointerSource::SafeDirect(*index),
                    TaintSource::Carried(index) => RawPointerSource::SafeCarried(*index),
                });
            }
            direct.extend(state.raw_direct.get(src_val).cloned().unwrap_or_default());
            direct.extend(state.raw_place_direct.get(src_val).cloned().unwrap_or_default());
            let mut carried = HashSet::new();
            for taint in state.safe_carried.get(src_val).into_iter().flatten() {
                carried.insert(match taint {
                    TaintSource::Direct(index) => RawPointerSource::SafeDirect(*index),
                    TaintSource::Carried(index) => RawPointerSource::SafeCarried(*index),
                });
            }
            carried.extend(state.raw_carried.get(src_val).cloned().unwrap_or_default());
            carried.extend(state.raw_place_carried.get(src_val).cloned().unwrap_or_default());
            state.raw_place_direct.entry(dest).or_default().extend(direct);
            state.raw_place_carried.entry(dest).or_default().extend(carried);
        }
    }

    fn get_raw_direct_taints(&self, state: &TaintState, op: &Operand) -> HashSet<RawPointerSource> {
        if let Operand::Value(val) = op {
            state.raw_direct.get(val).cloned().unwrap_or_default()
        } else {
            HashSet::new()
        }
    }

    fn get_raw_carried_taints(&self, state: &TaintState, op: &Operand) -> HashSet<RawPointerSource> {
        if let Operand::Value(val) = op {
            state.raw_carried.get(val).cloned().unwrap_or_default()
        } else {
            HashSet::new()
        }
    }

    fn add_safe_borrow_provenance(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if !self.can_carry_safe_loan(self.func.value(dest).ty) {
            return;
        }
        self.add_safe_direct_taint(state, dest, src);
        self.add_safe_carried_taint(state, dest, src);
        if let Operand::Value(src_val) = src {
            if !self.is_raw_pointer(self.func.value(*src_val).ty) {
                if let Some(taints) = state.safe_place_direct.get(src_val).cloned() {
                    state.safe_direct.entry(dest).or_default().extend(taints);
                }
                if let Some(taints) = state.safe_place_carried.get(src_val).cloned() {
                    state.safe_carried.entry(dest).or_default().extend(taints);
                }
            }
        }
    }

    fn resolve_alias<'b>(&self, op: &'b Operand, state: &'b TaintState) -> &'b Operand {
        let mut current = op;
        while let Operand::Value(v) = current {
            if let Some(alias) = state.aliases.get(v) {
                current = alias;
            } else {
                break;
            }
        }
        current
    }

    fn place_desc(&self, op: &Operand, state: &TaintState) -> PlaceDesc {
        compute_place_desc(op, &self.func.values, Some(&state.aliases), None)
    }

    fn direct_call_is_extern(&self, callee: &GlobalId) -> bool {
        let Some(ctx) = self.ctx else { return false; };
        let symbol_id = callee.symbol_id.or_else(|| {
            ctx.symbol_table.lookup(&callee.name, luna_semantic::symbol::ScopeId(0))
        });
        symbol_id.is_some_and(|symbol_id| {
            ctx.tables.extern_functions.contains(&symbol_id)
                || ctx.symbol_table.get_symbol(symbol_id).kind == luna_semantic::symbol::SymbolKind::ExternFunction
        })
    }

    fn raw_pointer_ffi_effect(&self, arg: &Operand) -> Option<(AccessKind, OwnershipKind)> {
        let ctx = self.ctx?;
        let Operand::Value(value) = arg else { return None; };
        match ctx.types.get(self.func.value(*value).ty) {
            luna_semantic::SemanticType::Pointer(luna_semantic::ty::Mutability::Mutable, _) => {
                Some((AccessKind::ReadWrite, OwnershipKind::BorrowMut))
            }
            luna_semantic::SemanticType::Pointer(_, _) => {
                Some((AccessKind::Read, OwnershipKind::BorrowShared))
            }
            _ => None,
        }
    }

    fn raw_value_origins(&self, state: &TaintState, value: &Operand) -> HashSet<RawPointerSource> {
        let mut origins = self.get_raw_direct_taints(state, value);
        origins.extend(self.get_raw_carried_taints(state, value));
        if origins.is_empty() {
            origins.insert(RawPointerSource::Unknown);
        }
        origins
    }

    fn raw_anchor_sources(&self, state: &TaintState, value: &Operand) -> HashSet<RawPointerAnchorSource> {
        let Operand::Value(value_id) = value else {
            return HashSet::from([RawPointerAnchorSource::Unknown]);
        };
        let mut sources = state.raw_pointer_anchor_direct.get(value_id).cloned().unwrap_or_default();
        sources.extend(state.raw_pointer_anchor_carried.get(value_id).into_iter().flatten().cloned());
        if sources.is_empty() {
            sources.insert(RawPointerAnchorSource::Unknown);
        }
        sources
    }

    fn raw_return_effect(origins: &HashSet<RawPointerSource>) -> RawPointerReturnEffect {
        let mut indices = Vec::new();
        for source in origins {
            match source {
                RawPointerSource::SafeDirect(index) | RawPointerSource::SafeCarried(index)
                | RawPointerSource::RawDirect(index) | RawPointerSource::RawCarried(index) => indices.push(*index),
                RawPointerSource::Unknown => return RawPointerReturnEffect::Unknown,
            }
        }
        indices.sort_unstable();
        indices.dedup();
        if indices.is_empty() { RawPointerReturnEffect::Independent } else { RawPointerReturnEffect::From(indices) }
    }

    fn raw_anchor_return_effect(anchors: &HashSet<RawPointerAnchorSource>) -> RawPointerAnchorReturnEffect {
        if anchors.contains(&RawPointerAnchorSource::Unknown) {
            return RawPointerAnchorReturnEffect::Unknown;
        }
        if anchors.is_empty() {
            return RawPointerAnchorReturnEffect::Independent;
        }
        let mut sources: Vec<_> = anchors.iter().cloned().collect();
        sources.sort_unstable();
        RawPointerAnchorReturnEffect::From(sources)
    }

    fn owner_field_anchor_source(
        &self,
        field_ptr: ValueId,
        state: &TaintState,
    ) -> Option<RawPointerAnchorSource> {
        let Instruction::FieldPtr { base, field_name: Some(field_name), .. } =
            &self.func.values.get(field_ptr.0 as usize)?.inst else { return None; };
        let ctx = self.ctx?;
        let Operand::Value(base_value) = self.resolve_alias(base, state) else { return None; };
        let mut owner_ty = ctx.types.resolve(self.func.value(*base_value).ty);
        loop {
            match ctx.types.get(owner_ty) {
                luna_semantic::SemanticType::Reference(_, _, inner)
                | luna_semantic::SemanticType::Pointer(_, inner) => owner_ty = ctx.types.resolve(*inner),
                luna_semantic::SemanticType::Struct(owner_sym, ..) => {
                    let contract = ctx.tables.raw_storage_anchor_contracts.get(owner_sym)?;
                    if !contract.field_names.iter().any(|name| name == field_name) {
                        return None;
                    }
                    break;
                }
                _ => return None,
            }
        }
        let owner_params: HashSet<_> = self.get_direct_taints(state, base).into_iter()
            .chain(self.get_carried_taints(state, base))
            .map(|taint| match taint { TaintSource::Direct(param) | TaintSource::Carried(param) => param })
            .collect();
        if owner_params.len() == 1 {
            owner_params.into_iter().next().map(|param| RawPointerAnchorSource::OwnerField {
                param,
                field: field_name.clone(),
            })
        } else {
            None
        }
    }

    fn propagate_raw_anchor(&self, state: &mut TaintState, dest: ValueId, src: &Operand) {
        if let Operand::Value(src_id) = src {
            if let Some(sources) = state.raw_pointer_anchor_direct.get(src_id).cloned() {
                state.raw_pointer_anchor_direct.entry(dest).or_default().extend(sources);
            }
            if let Some(sources) = state.raw_pointer_anchor_carried.get(src_id).cloned() {
                state.raw_pointer_anchor_carried.entry(dest).or_default().extend(sources);
            }
        }
    }

    fn invalidate_raw_anchor_values_for_field(
        &self,
        state: &mut TaintState,
        field_source: &RawPointerAnchorSource,
        except_value: Option<ValueId>,
    ) {
        let RawPointerAnchorSource::OwnerField { param, field } = field_source else { return; };
        for map in [&mut state.raw_pointer_anchor_direct, &mut state.raw_pointer_anchor_carried] {
            for (value, anchors) in map.iter_mut() {
                if Some(*value) == except_value { continue; }
                if anchors.contains(&RawPointerAnchorSource::OwnerField { param: *param, field: field.clone() }) {
                    anchors.clear();
                    anchors.insert(RawPointerAnchorSource::Unknown);
                }
            }
        }
        for anchors in state.raw_pointer_anchor_storage.values_mut() {
            if anchors.contains(&RawPointerAnchorSource::OwnerField { param: *param, field: field.clone() }) {
                anchors.clear();
                anchors.insert(RawPointerAnchorSource::Unknown);
            }
        }
    }
}

impl<'a> DataflowAnalysis<TaintState> for EffectInference<'a> {
    fn init_entry_state(&mut self, _func: &Function, state: &mut TaintState) {
        state.initialized = true;
        for (i, &val) in self.arg_values.iter().enumerate() {
            let mut s_dir = HashSet::new();
            s_dir.insert(TaintSource::Direct(i));
            state.direct.insert(val, s_dir);
            
            let mut s_car = HashSet::new();
            s_car.insert(TaintSource::Carried(i));
            state.carried.insert(val, s_car);

            // Direct tags identify the parameter storage a Borrow may refer
            // to. Carried tags represent a safe loan only when the parameter's
            // semantic value can actually contain one.
            state.safe_direct.entry(val).or_default().insert(TaintSource::Direct(i));
            if self.can_carry_safe_loan(self.func.param_types.get(i).copied().unwrap_or(self.func.value(val).ty)) {
                state.safe_carried.entry(val).or_default().insert(TaintSource::Carried(i));
            }
            if self.is_raw_pointer(self.func.param_types.get(i).copied().unwrap_or(self.func.value(val).ty)) {
                state.raw_direct.entry(val).or_default().insert(RawPointerSource::RawDirect(i));
                state.raw_carried.entry(val).or_default().insert(RawPointerSource::RawCarried(i));
                state.raw_pointer_anchor_direct.entry(val).or_default().insert(RawPointerAnchorSource::RawParam(i));
                state.raw_pointer_anchor_carried.entry(val).or_default().insert(RawPointerAnchorSource::RawParam(i));
                // MVIR represents parameter values through local storage in
                // some lowering paths. Seed that slot with the parameter's
                // raw value origin so a parameter Load is not confused with
                // an uninitialized/unknown raw-pointer field load.
                let place = self.place_desc(&Operand::Value(val), state);
                state.raw_pointer_storage.entry(place).or_default().extend([
                    RawPointerSource::RawDirect(i),
                    RawPointerSource::RawCarried(i),
                ]);
                state.raw_pointer_anchor_storage.entry(self.place_desc(&Operand::Value(val), state))
                    .or_default().insert(RawPointerAnchorSource::RawParam(i));
            }
        }
    }

    fn transfer_instruction(&mut self, val_id: ValueId, inst: &Instruction, state: &mut TaintState) {
        match inst {
            Instruction::Assign(source) => {
                self.add_direct_taint(state, val_id, source);
                self.add_carried_taint(state, val_id, source);
                self.add_safe_value_provenance(state, val_id, source);
                if self.is_raw_pointer(self.func.value(val_id).ty) {
                    let direct = self.get_raw_direct_taints(state, source);
                    let carried = self.get_raw_carried_taints(state, source);
                    state.raw_direct.entry(val_id).or_default().extend(direct);
                    state.raw_carried.entry(val_id).or_default().extend(carried);
                    self.propagate_raw_anchor(state, val_id, source);
                    if let Operand::Value(source_id) = source {
                        state.aliases.insert(val_id, Operand::Value(*source_id));
                    }
                }
                if let (Some(ctx), Operand::Value(_source_id)) = (self.ctx, source) {
                    if ctx.types.contains_pointer_or_reference(self.func.value(val_id).ty) {
                        let source_place = self.place_desc(source, state);
                        let destination_place = self.place_desc(&Operand::Value(val_id), state);
                        if source_place != destination_place {
                            let raw_origins: Vec<_> = state.raw_pointer_storage.iter()
                                .filter(|(place, _)| place.root == source_place.root && place.projections.starts_with(&source_place.projections))
                                .map(|(place, facts)| {
                                    let mut target = destination_place.clone();
                                    target.projections.extend_from_slice(&place.projections[source_place.projections.len()..]);
                                    (target, facts.clone())
                                }).collect();
                            for (place, facts) in raw_origins { state.raw_pointer_storage.insert(place, facts); }
                            let anchors: Vec<_> = state.raw_pointer_anchor_storage.iter()
                                .filter(|(place, _)| place.root == source_place.root && place.projections.starts_with(&source_place.projections))
                                .map(|(place, facts)| {
                                    let mut target = destination_place.clone();
                                    target.projections.extend_from_slice(&place.projections[source_place.projections.len()..]);
                                    (target, facts.clone())
                                }).collect();
                            for (place, facts) in anchors { state.raw_pointer_anchor_storage.insert(place, facts); }
                        }
                    }
                }
            }
            Instruction::Load { ptr } => {
                self.record_raw_pointee_access(ptr, &AccessKind::Read, state);
                for taint in self.get_direct_taints(state, ptr) {
                    if let TaintSource::Direct(arg_idx) = taint {
                        self.summary.args[arg_idx].access =
                            self.summary.args[arg_idx].access.merge(&AccessKind::Read);
                    }
                }
                // Loaded values conservatively inherit the pointer's carried
                // provenance. Even with type context, aggregate and
                // projection layouts can carry loans that are not visible from
                // the immediate MVIR value type. Return boundaries below use the
                // semantic return type to decide whether that provenance can escape.
                if let Operand::Value(ptr_val) = ptr {
                    if let Some(taints) = state.carried.get(ptr_val).cloned() {
                        state.direct.entry(val_id).or_default().extend(taints.clone());
                        state.carried.entry(val_id).or_default().extend(taints);
                    }
                }
                // A scalar loaded through a borrowed/raw address may retain
                // generic data provenance, but it cannot keep the address's
                // safe loan alive. Only a value whose semantic shape can carry
                // safe references inherits SafeLoanSet provenance.
                if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                    let carried = self.get_safe_carried_taints(state, ptr);
                    let Operand::Value(ptr_val) = ptr else { return; };
                    let place_direct = state.safe_place_direct.get(ptr_val).cloned().unwrap_or_default();
                    let place_carried = state.safe_place_carried.get(ptr_val).cloned().unwrap_or_default();
                    let recovered: HashSet<_> = carried
                        .into_iter()
                        .chain(place_direct)
                        .chain(place_carried)
                        .collect();
                    state.safe_direct.entry(val_id).or_default().extend(recovered.iter().copied());
                    state.safe_carried.entry(val_id).or_default().extend(recovered);
                } else if self.is_raw_pointer(self.func.value(val_id).ty) {
                    // The field/address provenance describes where the load
                    // occurs, not the raw pointer VALUE stored there. Recover
                    // only an explicit stored-value fact; otherwise retain an
                    // unknown alternative so a helper summary cannot turn the
                    // owner's field address into pointee origin.
                    if let Operand::Value(ptr_val) = ptr {
                        let place = self.place_desc(ptr, state);
                        let origins = state
                            .raw_pointer_storage
                            .get(&place)
                            .cloned()
                            .unwrap_or_else(|| HashSet::from([RawPointerSource::Unknown]));
                        state.raw_direct.entry(val_id).or_default().extend(origins.iter().copied());
                        state.raw_carried.entry(val_id).or_default().extend(origins);

                        let anchor_sources = state.raw_pointer_anchor_storage.get(&place).cloned()
                            .or_else(|| self.owner_field_anchor_source(*ptr_val, state).map(|source| HashSet::from([source])))
                            .unwrap_or_else(|| HashSet::from([RawPointerAnchorSource::Unknown]));
                        state.raw_pointer_anchor_direct.entry(val_id).or_default().extend(anchor_sources.iter().cloned());
                        state.raw_pointer_anchor_carried.entry(val_id).or_default().extend(anchor_sources);
                    } else {
                        state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                        state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                        state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                        state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    }
                }
            }
            Instruction::Store { ptr, value } | Instruction::StoreAnchored { ptr, value } => {
                self.record_raw_pointee_access(ptr, &AccessKind::Write, state);
                for taint in self.get_direct_taints(state, ptr) {
                    if let TaintSource::Direct(arg_idx) = taint {
                        self.summary.args[arg_idx].access =
                            self.summary.args[arg_idx].access.merge(&AccessKind::Write);
                    }
                }

                if let Operand::Global(_) = ptr {
                    for taint in self.get_direct_taints(state, value) {
                        if let TaintSource::Direct(arg_idx) = taint {
                            self.summary.args[arg_idx].escape = self.summary.args[arg_idx]
                                .escape
                                .merge(&EscapeKind::MayEscape);
                        }
                    }
                }
                for taint in self.get_direct_taints(state, value) {
                    if let TaintSource::Direct(arg_idx) = taint {
                        self.summary.args[arg_idx].ownership = self.summary.args[arg_idx]
                            .ownership
                            .merge(&OwnershipKind::Consume);
                    }
                }
                let ptr_val_opt = {
                    let resolved_ptr = self.resolve_alias(ptr, state);
                    if let Operand::Value(ptr_val) = resolved_ptr {
                        Some(*ptr_val)
                    } else {
                        None
                    }
                };
                if let Some(ptr_val) = ptr_val_opt {
                    // Pointer's carried provenance absorbs the value's direct and carried provenance
                    let val_taints = self.get_direct_taints(state, value);
                    if !val_taints.is_empty() {
                        state.carried.entry(ptr_val).or_default().extend(val_taints);
                    }
                    let val_carried = self.get_carried_taints(state, value);
                    if !val_carried.is_empty() {
                        state.carried.entry(ptr_val).or_default().extend(val_carried);
                    }

                    let value_carries_safe_loan = match value {
                        Operand::Value(value_id) => self.can_carry_safe_loan(self.func.value(*value_id).ty),
                        _ => false,
                    };
                    if value_carries_safe_loan {
                        let safe_direct = self.get_safe_direct_taints(state, value);
                        let safe_carried = self.get_safe_carried_taints(state, value);
                        state.safe_carried.entry(ptr_val).or_default().extend(safe_direct);
                        state.safe_carried.entry(ptr_val).or_default().extend(safe_carried);
                    } else if let Operand::Value(value_id) = value {
                        if self.is_raw_pointer(self.func.value(*value_id).ty) {
                            let place = self.place_desc(ptr, state);
                            let anchor_field = self.owner_field_anchor_source(ptr_val, state);
                            if let Some(anchor_field) = &anchor_field {
                                self.invalidate_raw_anchor_values_for_field(state, anchor_field, Some(*value_id));
                            }
                            let origins = self.raw_value_origins(state, value);
                            let is_direct_alloca = matches!(ptr, Operand::Value(pointer_value)
                                if *pointer_value == ptr_val
                                    && matches!(self.func.value(*pointer_value).inst, Instruction::Alloca));
                            if is_direct_alloca || anchor_field.is_some() {
                                state.raw_pointer_storage.insert(place.clone(), origins);
                            } else {
                                state.raw_pointer_storage.entry(place.clone()).or_default().extend(origins);
                            }

                            let anchors = if matches!(inst, Instruction::StoreAnchored { .. }) {
                                anchor_field.clone()
                                    .map(|source| HashSet::from([source]))
                                    .unwrap_or_else(|| HashSet::from([RawPointerAnchorSource::Unknown]))
                            } else {
                                self.raw_anchor_sources(state, value)
                            };
                            if is_direct_alloca || anchor_field.is_some() {
                                state.raw_pointer_anchor_storage.insert(place, anchors);
                            } else {
                                state.raw_pointer_anchor_storage.entry(place).or_default().extend(anchors);
                            }
                        }
                    }
                }
            }
            Instruction::Borrow { is_rw, base } => {
                let is_raw_pointer_to_safe_borrow = self.ctx.is_some_and(|ctx| matches!(base, Operand::Value(base_value)
                    if matches!(ctx.types.get(self.func.value(*base_value).ty), luna_semantic::SemanticType::Pointer(..))));
                if !is_raw_pointer_to_safe_borrow {
                    for taint in self.get_direct_taints(state, base) {
                        if let TaintSource::Direct(arg_idx) = taint {
                            let ownership = if *is_rw {
                                OwnershipKind::BorrowMut
                            } else {
                                OwnershipKind::BorrowShared
                            };
                            self.summary.args[arg_idx].ownership =
                                self.summary.args[arg_idx].ownership.merge(&ownership);
                        }
                    }
                }
                if !is_raw_pointer_to_safe_borrow {
                    self.add_direct_taint(state, val_id, base);
                    self.add_carried_taint(state, val_id, base);
                    self.add_safe_borrow_provenance(state, val_id, base);
                }
                if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                    if let Operand::Value(base_val) = base {
                        // A validated raw-to-safe promotion from a contracted
                        // field starts a safe loan here. Preserve that owner
                        // relation for callers, without treating the raw
                        // pointer as having carried a safe loan beforehand.
                        let anchor_sources = state.raw_pointer_anchor_direct.get(base_val).cloned().unwrap_or_default();
                        if anchor_sources.len() == 1 {
                            if let Some(RawPointerAnchorSource::OwnerField { param, .. }) = anchor_sources.iter().next() {
                                state.safe_direct.entry(val_id).or_default().insert(TaintSource::Direct(*param));
                                if *param < self.summary.args.len() {
                                    let ownership = if *is_rw { OwnershipKind::BorrowMut } else { OwnershipKind::BorrowShared };
                                    self.summary.args[*param].ownership = self.summary.args[*param].ownership.merge(&ownership);
                                }
                            }
                        }
                        for origin in state.raw_direct.get(base_val).into_iter().flatten()
                            .chain(state.raw_carried.get(base_val).into_iter().flatten())
                        {
                            match origin {
                                RawPointerSource::SafeDirect(index) => {
                                    state.safe_direct.entry(val_id).or_default().insert(TaintSource::Direct(*index));
                                }
                                RawPointerSource::SafeCarried(index) => {
                                    state.safe_carried.entry(val_id).or_default().insert(TaintSource::Carried(*index));
                                }
                                RawPointerSource::RawDirect(_) | RawPointerSource::RawCarried(_) | RawPointerSource::Unknown => {}
                            }
                        }
                    }
                }
                if let Operand::Value(b) = base {
                    state.aliases.insert(val_id, Operand::Value(*b));
                }
            }
            Instruction::CallDirect { args, callee, .. } => {
                let mut applied_summary = None;
                let is_extern_call = self.direct_call_is_extern(callee);

                if let Some(map) = self.callee_summaries {
                    if let Some(sum) = map.get(callee) {
                        applied_summary = Some(sum.clone());
                    }
                }

                if let Some(sum) = applied_summary {
                    // Apply Formal to Actual Mapping
                    for (i, actual_arg) in args.iter().enumerate() {
                        if i < sum.args.len() {
                            let ffi_effect = if is_extern_call {
                                self.raw_pointer_ffi_effect(actual_arg)
                                    .map(|(access, ownership)| (access, ownership, EscapeKind::CallOnly))
                            } else {
                                None
                            };
                            let (access, ownership, escape) = ffi_effect.unwrap_or_else(|| {
                                let formal_effect = &sum.args[i];
                                (formal_effect.access.clone(), formal_effect.ownership.clone(), formal_effect.escape.clone())
                            });
                            self.record_raw_pointee_access(actual_arg, &access, state);
                            for taint in self.get_direct_taints(state, actual_arg) {
                                if let TaintSource::Direct(arg_idx) = taint {
                                    self.summary.args[arg_idx].access = self.summary.args[arg_idx]
                                        .access
                                        .merge(&access);
                                    self.summary.args[arg_idx].escape = self.summary.args[arg_idx]
                                        .escape
                                        .merge(&escape);
                                    self.summary.args[arg_idx].ownership = self.summary.args[arg_idx]
                                        .ownership
                                        .merge(&ownership);
                                }
                            }
                        }
                    }

                    // Safe validity loans are orthogonal to raw pointer origin/anchor facts.
                    if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                        let (direct_sources, carried_sources) = sum.ret.sources();
                        for (indices, carried) in [(direct_sources, false), (carried_sources, true)] {
                            let mut taints = HashSet::new();
                            for &formal_idx in indices {
                                if let Some(actual) = args.get(formal_idx) {
                                    if !carried { taints.extend(self.get_safe_direct_taints(state, actual)); }
                                    taints.extend(self.get_safe_carried_taints(state, actual));
                                }
                            }
                            if !taints.is_empty() {
                                if carried { state.safe_carried.entry(val_id).or_default().extend(taints); }
                                else { state.safe_direct.entry(val_id).or_default().extend(taints); }
                            }
                        }
                    }
                    if self.is_raw_pointer(self.func.value(val_id).ty) {
                        match &sum.raw_pointer_ret {
                            RawPointerReturnEffect::From(indices) => {
                                for &idx in indices {
                                    if idx < args.len() {
                                        let direct = self.get_raw_direct_taints(state, &args[idx]);
                                        let carried = self.get_raw_carried_taints(state, &args[idx]);
                                        state.raw_direct.entry(val_id).or_default().extend(direct);
                                        state.raw_carried.entry(val_id).or_default().extend(carried);
                                    }
                                }
                            }
                            RawPointerReturnEffect::Unknown => {
                                state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                                state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                            }
                            RawPointerReturnEffect::Independent => {}
                        }

                        match &sum.raw_pointer_anchor_ret {
                            RawPointerAnchorReturnEffect::From(sources) => {
                                for source in sources {
                                    match source {
                                        RawPointerAnchorSource::RawParam(idx) if *idx < args.len() => {
                                            let anchors = self.raw_anchor_sources(state, &args[*idx]);
                                            state.raw_pointer_anchor_direct.entry(val_id).or_default().extend(anchors.iter().cloned());
                                            state.raw_pointer_anchor_carried.entry(val_id).or_default().extend(anchors);
                                        }
                                        RawPointerAnchorSource::OwnerField { param, field } if *param < args.len() => {
                                            let owner_params: HashSet<_> = self.get_direct_taints(state, &args[*param]).into_iter()
                                                .chain(self.get_carried_taints(state, &args[*param]))
                                                .map(|taint| match taint { TaintSource::Direct(index) | TaintSource::Carried(index) => index })
                                                .collect();
                                            let propagated = if owner_params.len() == 1 {
                                                RawPointerAnchorSource::OwnerField {
                                                    param: *owner_params.iter().next().unwrap(),
                                                    field: field.clone(),
                                                }
                                            } else {
                                                RawPointerAnchorSource::Unknown
                                            };
                                            state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(propagated.clone());
                                            state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(propagated);
                                        }
                                        RawPointerAnchorSource::RawParam(_) | RawPointerAnchorSource::OwnerField { .. } => {
                                            state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                                            state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                                        }
                                        RawPointerAnchorSource::Unknown => {
                                            state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                                            state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                                        }
                                    }
                                }
                            }
                            RawPointerAnchorReturnEffect::Unknown => {
                                state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                                state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                            }
                            RawPointerAnchorReturnEffect::Independent => {}
                        }
                    }
                    if let Some(ctx) = self.ctx {
                        let result_place = self.place_desc(&Operand::Value(val_id), state);
                        for (field_index, field_name) in direct_raw_pointer_fields(ctx, self.func.value(val_id).ty) {
                            let mut field_place = result_place.clone();
                            field_place.projections.push(Projection::Field(field_index));
                            let Some(field_effect) = sum.raw_pointer_field_ret.get(&field_name) else {
                                state.raw_pointer_storage.insert(field_place.clone(), HashSet::from([RawPointerSource::Unknown]));
                                state.raw_pointer_anchor_storage.insert(field_place, HashSet::from([RawPointerAnchorSource::Unknown]));
                                continue;
                            };
                            let mut origins = HashSet::new();
                            match &field_effect.origin {
                                RawPointerReturnEffect::From(indices) => {
                                    for &index in indices {
                                        if let Some(arg) = args.get(index) {
                                            origins.extend(self.get_raw_direct_taints(state, arg));
                                            origins.extend(self.get_raw_carried_taints(state, arg));
                                        } else {
                                            origins.insert(RawPointerSource::Unknown);
                                        }
                                    }
                                    if origins.is_empty() { origins.insert(RawPointerSource::Unknown); }
                                }
                                RawPointerReturnEffect::Unknown => { origins.insert(RawPointerSource::Unknown); }
                                RawPointerReturnEffect::Independent => {}
                            }
                            state.raw_pointer_storage.insert(field_place.clone(), origins);

                            let mut anchors = HashSet::new();
                            match &field_effect.anchor {
                                RawPointerAnchorReturnEffect::From(sources) => for source in sources {
                                    match source {
                                        RawPointerAnchorSource::RawParam(index) => {
                                            if let Some(arg) = args.get(*index) {
                                                anchors.extend(self.raw_anchor_sources(state, arg));
                                            } else { anchors.insert(RawPointerAnchorSource::Unknown); }
                                        }
                                        RawPointerAnchorSource::OwnerField { param, field } => {
                                            if let Some(arg) = args.get(*param) {
                                                let owner_params: HashSet<_> = self.get_direct_taints(state, arg).into_iter()
                                                    .chain(self.get_carried_taints(state, arg))
                                                    .map(|taint| match taint { TaintSource::Direct(i) | TaintSource::Carried(i) => i }).collect();
                                                if owner_params.len() == 1 {
                                                    anchors.insert(RawPointerAnchorSource::OwnerField {
                                                        param: *owner_params.iter().next().unwrap(), field: field.clone(),
                                                    });
                                                } else { anchors.insert(RawPointerAnchorSource::Unknown); }
                                            } else { anchors.insert(RawPointerAnchorSource::Unknown); }
                                        }
                                        RawPointerAnchorSource::Unknown => { anchors.insert(RawPointerAnchorSource::Unknown); }
                                    }
                                },
                                RawPointerAnchorReturnEffect::Unknown => { anchors.insert(RawPointerAnchorSource::Unknown); }
                                RawPointerAnchorReturnEffect::Independent => {}
                            }
                            state.raw_pointer_anchor_storage.insert(field_place, anchors);
                        }
                    }
                } else {
                    // Unknown direct calls remain conservative. Extern raw
                    // pointer arguments are different: raw pointers carry no
                    // safe loan, so summarize their pointee access as call-
                    // scoped and their mutability from the declared type.
                    for arg in args.iter() {
                        if is_extern_call {
                            if let Some((access, ownership)) = self.raw_pointer_ffi_effect(arg) {
                                for taint in self.get_direct_taints(state, arg) {
                                    if let TaintSource::Direct(arg_idx) = taint {
                                        self.summary.args[arg_idx].access = self.summary.args[arg_idx].access.merge(&access);
                                        self.summary.args[arg_idx].escape = self.summary.args[arg_idx].escape.merge(&EscapeKind::CallOnly);
                                        self.summary.args[arg_idx].ownership = self.summary.args[arg_idx].ownership.merge(&ownership);
                                    }
                                }
                                continue;
                            }
                        }
                        for taint in self.get_direct_taints(state, arg) {
                            if let TaintSource::Direct(arg_idx) = taint {
                                self.summary.args[arg_idx].access = self.summary.args[arg_idx]
                                    .access
                                    .merge(&AccessKind::Unknown);
                                self.summary.args[arg_idx].escape = self.summary.args[arg_idx]
                                    .escape
                                    .merge(&EscapeKind::Unknown);
                                self.summary.args[arg_idx].ownership = self.summary.args[arg_idx]
                                    .ownership
                                    .merge(&OwnershipKind::Unknown);
                            }
                        }
                    }
                    if self.is_raw_pointer(self.func.value(val_id).ty) {
                        state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                        state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                        state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                        state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    }
                }
                // A declared validity relation can attach an existing safe
                // loan to a borrowed header even when its fields are raw. It
                // propagates only safe loan facts, never raw address taints.
                if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                    if let Some(contract) = self.ctx.and_then(|ctx| callee.symbol_id.and_then(|symbol| ctx.tables.fn_lifetime_contracts.get(&symbol))) {
                        if let Some(provenance) = &contract.return_provenance {
                            for index in provenance.indices() {
                                if let Some(argument) = args.get(*index as usize) {
                                    let direct = self.get_safe_direct_taints(state, argument);
                                    let carried = self.get_safe_carried_taints(state, argument);
                                    state.safe_direct.entry(val_id).or_default().extend(direct);
                                    state.safe_carried.entry(val_id).or_default().extend(carried);
                                }
                            }
                        }
                    }
                }
            }
            Instruction::CallIndirect { args, callee } | Instruction::CallClosure { args, closure: callee } => {
                // Fallback to conservative unknown
                for arg in args.iter() {
                    self.record_raw_pointee_access(arg, &AccessKind::Unknown, state);
                    for taint in self.get_direct_taints(state, arg) {
                        if let TaintSource::Direct(arg_idx) = taint {
                            self.summary.args[arg_idx].access = self.summary.args[arg_idx]
                                .access
                                .merge(&AccessKind::Unknown);
                            self.summary.args[arg_idx].escape = self.summary.args[arg_idx]
                                .escape
                                .merge(&EscapeKind::Unknown);
                            self.summary.args[arg_idx].ownership = self.summary.args[arg_idx]
                                .ownership
                                .merge(&OwnershipKind::Unknown);
                        }
                    }
                }
                // Opaque calls may return any validity loan in their arguments
                // or closure environment. Absence of a body is not independence.
                if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                    for carrier in args.iter().chain(std::iter::once(callee)) {
                        let direct = self.get_safe_direct_taints(state, carrier);
                        let carried = self.get_safe_carried_taints(state, carrier);
                        state.safe_direct.entry(val_id).or_default().extend(direct);
                        state.safe_carried.entry(val_id).or_default().extend(carried);
                    }
                }
                if self.is_raw_pointer(self.func.value(val_id).ty) {
                    state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                }
            }
            Instruction::MakeTraitObject { data_ptr, .. } => {
                self.add_direct_taint(state, val_id, data_ptr);
                self.add_carried_taint(state, val_id, data_ptr);
                self.add_safe_value_provenance(state, val_id, data_ptr);
                if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                    if let Operand::Value(source) = data_ptr {
                        let direct = state.safe_place_direct.get(source).cloned().unwrap_or_default();
                        let carried = state.safe_place_carried.get(source).cloned().unwrap_or_default();
                        state.safe_carried.entry(val_id).or_default().extend(direct);
                        state.safe_carried.entry(val_id).or_default().extend(carried);
                    }
                }
                if self.is_raw_pointer(self.func.value(val_id).ty) {
                    state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                }
                if let Operand::Value(b) = data_ptr {
                    state.aliases.insert(val_id, Operand::Value(*b));
                }
            }
            Instruction::MakeSlice { data_ptr, .. } => {
                if let Some(ctx) = self.ctx {
                    if let luna_semantic::SemanticType::Reference(_, mutability, element) = ctx.types.get(ctx.types.resolve(self.func.value(val_id).ty)) {
                        if matches!(ctx.types.get(ctx.types.resolve(*element)), luna_semantic::SemanticType::Slice(_)) {
                            let borrow = Instruction::Borrow { is_rw: *mutability == luna_semantic::ty::Mutability::Mutable, base: data_ptr.clone() };
                            self.transfer_instruction(val_id, &borrow, state);
                            return;
                        }
                    }
                }
                self.add_direct_taint(state, val_id, data_ptr);
                self.add_carried_taint(state, val_id, data_ptr);
                self.add_safe_value_provenance(state, val_id, data_ptr);
                if self.can_carry_safe_loan(self.func.value(val_id).ty) {
                    if let Operand::Value(source) = data_ptr {
                        let direct = state.safe_place_direct.get(source).cloned().unwrap_or_default();
                        let carried = state.safe_place_carried.get(source).cloned().unwrap_or_default();
                        state.safe_carried.entry(val_id).or_default().extend(direct);
                        state.safe_carried.entry(val_id).or_default().extend(carried);
                    }
                }
                if let Operand::Value(b) = data_ptr {
                    state.aliases.insert(val_id, Operand::Value(*b));
                }
            }
            Instruction::DropVirt { obj } => {
                for taint in self.get_direct_taints(state, obj) {
                    if let TaintSource::Direct(arg_idx) = taint {
                        self.summary.args[arg_idx].access = self.summary.args[arg_idx].access.merge(&AccessKind::Unknown);
                        self.summary.args[arg_idx].ownership = self.summary.args[arg_idx].ownership.merge(&OwnershipKind::Unknown);
                    }
                }
            }
            Instruction::CallVirt { obj, args, .. } => {
                let mut all_args = vec![obj];
                all_args.extend(args.iter());
                for arg in all_args {
                    for taint in self.get_direct_taints(state, arg) {
                        if let TaintSource::Direct(arg_idx) = taint {
                            self.summary.args[arg_idx].access = self.summary.args[arg_idx].access.merge(&AccessKind::Unknown);
                            self.summary.args[arg_idx].escape = self.summary.args[arg_idx].escape.merge(&EscapeKind::Unknown);
                            self.summary.args[arg_idx].ownership = self.summary.args[arg_idx].ownership.merge(&OwnershipKind::Unknown);
                        }
                    }
                }
                if self.is_raw_pointer(self.func.value(val_id).ty) {
                    state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                }
            }
            Instruction::Variant { args, .. } => {
                for arg in args {
                    self.add_direct_taint(state, val_id, arg);
                    self.add_carried_taint(state, val_id, arg);
                    self.add_safe_value_provenance(state, val_id, arg);
                }
            }
            Instruction::Extract { value, .. } | Instruction::Tag { value } => {
                self.add_direct_taint(state, val_id, value);
                self.add_carried_taint(state, val_id, value);
                self.add_safe_value_provenance(state, val_id, value);
            }
            Instruction::FieldPtr { base, .. } => {
                self.add_direct_taint(state, val_id, base);
                self.add_carried_taint(state, val_id, base);
                self.add_safe_place_provenance(state, val_id, base);
                self.add_raw_place_provenance(state, val_id, base);
                if let Operand::Value(b) = base {
                    state.aliases.insert(val_id, Operand::Value(*b));
                }
            }
            Instruction::Add { left, right }
            | Instruction::Sub { left, right }
            | Instruction::Mul { left, right }
            | Instruction::Div { left, right }
            | Instruction::Rem { left, right }
            | Instruction::Eq { left, right }
            | Instruction::LessThan { left, right }
            | Instruction::LessOrEq { left, right }
            | Instruction::GreaterThan { left, right }
            | Instruction::GreaterOrEq { left, right } => {
                self.add_direct_taint(state, val_id, left);
                self.add_direct_taint(state, val_id, right);
                self.add_carried_taint(state, val_id, left);
                self.add_carried_taint(state, val_id, right);
                self.add_safe_value_provenance(state, val_id, left);
                self.add_safe_value_provenance(state, val_id, right);
                if matches!(inst, Instruction::Add { .. } | Instruction::Sub { .. }) {
                    let mut raw_direct = self.get_raw_direct_taints(state, left);
                    raw_direct.extend(self.get_raw_direct_taints(state, right));
                    let mut raw_carried = self.get_raw_carried_taints(state, left);
                    raw_carried.extend(self.get_raw_carried_taints(state, right));
                    if !raw_direct.is_empty() {
                        state.raw_direct.entry(val_id).or_default().extend(raw_direct);
                    }
                    if !raw_carried.is_empty() {
                        state.raw_carried.entry(val_id).or_default().extend(raw_carried);
                    }
                    if self.is_raw_pointer(self.func.value(val_id).ty) {
                        self.propagate_raw_anchor(state, val_id, left);
                        self.propagate_raw_anchor(state, val_id, right);
                    }
                }
                if self.is_raw_pointer(self.func.value(val_id).ty) {
                    state.raw_direct.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_carried.entry(val_id).or_default().insert(RawPointerSource::Unknown);
                    state.raw_pointer_anchor_direct.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                    state.raw_pointer_anchor_carried.entry(val_id).or_default().insert(RawPointerAnchorSource::Unknown);
                }
            }
            Instruction::Cast { value, .. } => {
                self.add_direct_taint(state, val_id, value);
                self.add_carried_taint(state, val_id, value);
                self.add_safe_value_provenance(state, val_id, value);
                if let Operand::Value(src) = value {
                    let raw_direct = state.raw_direct.get(src).cloned().unwrap_or_default();
                    let raw_carried = state.raw_carried.get(src).cloned().unwrap_or_default();
                    state.raw_direct.entry(val_id).or_default().extend(raw_direct);
                    state.raw_carried.entry(val_id).or_default().extend(raw_carried);
                    if self.is_raw_pointer(self.func.value(val_id).ty) {
                        self.propagate_raw_anchor(state, val_id, value);
                        self.add_raw_place_provenance(state, val_id, value);
                        if self.can_carry_safe_loan(self.func.value(*src).ty) {
                            for taint in state.safe_direct.get(src).into_iter().flatten() {
                                state.raw_direct.entry(val_id).or_default().insert(match taint {
                                    TaintSource::Direct(index) => RawPointerSource::SafeDirect(*index),
                                    TaintSource::Carried(index) => RawPointerSource::SafeCarried(*index),
                                });
                            }
                            for taint in state.safe_carried.get(src).into_iter().flatten() {
                                state.raw_carried.entry(val_id).or_default().insert(match taint {
                                    TaintSource::Direct(index) => RawPointerSource::SafeDirect(*index),
                                    TaintSource::Carried(index) => RawPointerSource::SafeCarried(*index),
                                });
                            }
                        }
                    }
                }
                if let Operand::Value(b) = value {
                    state.aliases.insert(val_id, Operand::Value(*b));
                }
            }
            Instruction::PtrOffset { ptr, .. } => {
                self.add_direct_taint(state, val_id, ptr);
                self.add_carried_taint(state, val_id, ptr);
                self.add_safe_place_provenance(state, val_id, ptr);
                if self.is_raw_pointer(self.func.value(val_id).ty) {
                    if let Operand::Value(src) = ptr {
                        let raw_direct = state.raw_direct.get(src).cloned().unwrap_or_default();
                        let raw_carried = state.raw_carried.get(src).cloned().unwrap_or_default();
                        state.raw_direct.entry(val_id).or_default().extend(raw_direct);
                        state.raw_carried.entry(val_id).or_default().extend(raw_carried);
                        self.propagate_raw_anchor(state, val_id, ptr);
                    }
                }
                if let Operand::Value(b) = ptr {
                    state.aliases.insert(val_id, Operand::Value(*b));
                }
            }
            Instruction::SizeOf { .. } |
            Instruction::AlignOf { .. } => {}
            _ => {}
        }
    }

    fn transfer_terminator(&mut self, term: &Terminator, state: &mut TaintState) {
        if let Terminator::Ret { value: Some(val) } = term {
            if let Some(ctx) = self.ctx {
                let return_place = self.place_desc(val, state);
                for (field_index, field_name) in direct_raw_pointer_fields(ctx, self.func.ret_ty) {
                    let mut field_place = return_place.clone();
                    field_place.projections.push(Projection::Field(field_index));
                    // Absence is unknown, not independent: the field may have
                    // arrived from an opaque aggregate or another predecessor.
                    let origins = state.raw_pointer_storage.get(&field_place).cloned()
                        .unwrap_or_else(|| HashSet::from([RawPointerSource::Unknown]));
                    let anchors = state.raw_pointer_anchor_storage.get(&field_place).cloned()
                        .unwrap_or_else(|| HashSet::from([RawPointerAnchorSource::Unknown]));
                    let effect = RawPointerFieldReturnEffect {
                        origin: Self::raw_return_effect(&origins),
                        anchor: Self::raw_anchor_return_effect(&anchors),
                    };
                    self.summary.raw_pointer_field_ret.entry(field_name)
                        .and_modify(|previous| *previous = previous.merge(&effect))
                        .or_insert(effect);
                }
            }
            if self.is_raw_pointer(self.func.ret_ty) {
                let origins = self.get_raw_direct_taints(state, val)
                    .into_iter().chain(self.get_raw_carried_taints(state, val)).collect();
                let raw_effect = Self::raw_return_effect(&origins);
                self.summary.raw_pointer_ret = if self.saw_raw_pointer_return {
                    self.summary.raw_pointer_ret.merge(&raw_effect)
                } else {
                    raw_effect
                };
                self.saw_raw_pointer_return = true;
                let Operand::Value(value_id) = val else { return; };
                let mut anchor_sources = state.raw_pointer_anchor_direct.get(value_id).cloned().unwrap_or_default();
                anchor_sources.extend(state.raw_pointer_anchor_carried.get(value_id).into_iter().flatten().cloned());
                let anchor_effect = Self::raw_anchor_return_effect(&anchor_sources);
                self.summary.raw_pointer_anchor_ret = if self.saw_raw_pointer_anchor_return {
                    self.summary.raw_pointer_anchor_ret.merge(&anchor_effect)
                } else {
                    anchor_effect
                };
                self.saw_raw_pointer_anchor_return = true;
                return;
            }
            let ret_can_borrow = if let Some(ctx) = self.ctx {
                let sem_ty = self.func.ret_ty;
                ctx.may_carry_validity_borrow(sem_ty)
            } else {
                true
            };
            if !ret_can_borrow {
                debug_assert!(
                    self.get_safe_direct_taints(state, val).is_empty()
                        && self.get_safe_carried_taints(state, val).is_empty(),
                    "non-safe-reference return unexpectedly carries a safe loan"
                );
                return;
            }

            let direct_taints = self.get_safe_direct_taints(state, val);
            let carried_taints = self.get_safe_carried_taints(state, val);
            
            if !direct_taints.is_empty() {
                let mut direct_args = Vec::new();
                let mut carried_args = Vec::new();
                for taint in direct_taints {
                    match taint {
                        TaintSource::Direct(i) => direct_args.push(i),
                        TaintSource::Carried(i) => carried_args.push(i),
                    }
                }
                
                direct_args.sort();
                carried_args.sort();
                
                if !direct_args.is_empty() {
                    self.summary.ret = self
                        .summary
                        .ret
                        .merge(&ReturnEffect::BorrowsFrom(direct_args.clone()));
                        
                    for arg_idx in direct_args {
                        self.summary.args[arg_idx].ownership = self.summary.args[arg_idx]
                            .ownership
                            .merge(&OwnershipKind::Consume);
                    }
                }
                
                if !carried_args.is_empty() {
                    self.summary.ret = self
                        .summary
                        .ret
                        .merge(&ReturnEffect::BorrowsCarried(carried_args.clone()));
                        
                    // Notice: We don't necessarily Consume the base pointer,
                    // but the borrow checker will handle the carried loan.
                }
            }
            if !carried_taints.is_empty() {
                let mut direct_args = Vec::new();
                let mut carried_args = Vec::new();
                for taint in carried_taints {
                    match taint {
                        TaintSource::Direct(index) => direct_args.push(index),
                        TaintSource::Carried(index) => carried_args.push(index),
                    }
                }
                direct_args.sort_unstable(); direct_args.dedup();
                carried_args.sort_unstable(); carried_args.dedup();
                if !direct_args.is_empty() {
                    self.summary.ret = self.summary.ret.merge(&ReturnEffect::BorrowsFrom(direct_args));
                }
                if !carried_args.is_empty() {
                    self.summary.ret = self.summary.ret.merge(&ReturnEffect::BorrowsCarried(carried_args));
                }
            }
        }
    }
    fn merge(&mut self, dest: &mut TaintState, src: &TaintState) -> bool {
        if !dest.initialized {
            *dest = src.clone();
            dest.initialized = true;
            return true;
        }
        let mut changed = false;

        for (val, taints) in &src.direct {
            let dest_taints = dest.direct.entry(*val).or_default();
            let old_len = dest_taints.len();
            dest_taints.extend(taints);
            if dest_taints.len() != old_len {
                changed = true;
            }
        }
        
        for (val, taints) in &src.carried {
            let dest_taints = dest.carried.entry(*val).or_default();
            let old_len = dest_taints.len();
            dest_taints.extend(taints);
            if dest_taints.len() != old_len {
                changed = true;
            }
        }

        for (val, taints) in &src.safe_direct {
            let dest_taints = dest.safe_direct.entry(*val).or_default();
            let old_len = dest_taints.len();
            dest_taints.extend(taints);
            if dest_taints.len() != old_len {
                changed = true;
            }
        }

        for (val, taints) in &src.safe_carried {
            let dest_taints = dest.safe_carried.entry(*val).or_default();
            let old_len = dest_taints.len();
            dest_taints.extend(taints);
            if dest_taints.len() != old_len {
                changed = true;
            }
        }

        for (val, taints) in &src.safe_place_direct {
            let dest_taints = dest.safe_place_direct.entry(*val).or_default();
            let old_len = dest_taints.len();
            dest_taints.extend(taints);
            if dest_taints.len() != old_len {
                changed = true;
            }
        }

        for (val, taints) in &src.safe_place_carried {
            let dest_taints = dest.safe_place_carried.entry(*val).or_default();
            let old_len = dest_taints.len();
            dest_taints.extend(taints);
            if dest_taints.len() != old_len {
                changed = true;
            }
        }

        for (source, destination) in [
            (&src.raw_direct, &mut dest.raw_direct),
            (&src.raw_carried, &mut dest.raw_carried),
            (&src.raw_place_direct, &mut dest.raw_place_direct),
            (&src.raw_place_carried, &mut dest.raw_place_carried),
        ] {
            for (val, taints) in source {
                let destination_taints = destination.entry(*val).or_default();
                let old_len = destination_taints.len();
                destination_taints.extend(taints);
                if destination_taints.len() != old_len {
                    changed = true;
                }
            }
        }

        for (place, origins) in &src.raw_pointer_storage {
            let destination = dest.raw_pointer_storage.entry(place.clone()).or_default();
            let old_len = destination.len();
            destination.extend(origins.iter().copied());
            if destination.len() != old_len {
                changed = true;
            }
        }

        for (source, destination) in [
            (&src.raw_pointer_anchor_direct, &mut dest.raw_pointer_anchor_direct),
            (&src.raw_pointer_anchor_carried, &mut dest.raw_pointer_anchor_carried),
        ] {
            let keys: HashSet<_> = destination.keys().chain(source.keys()).copied().collect();
            for value in keys {
                let dest_has = destination.contains_key(&value);
                let src_has = source.contains_key(&value);
                let dest_anchors = destination.entry(value).or_default();
                let old_len = dest_anchors.len();
                if !dest_has || !src_has {
                    dest_anchors.insert(RawPointerAnchorSource::Unknown);
                }
                if let Some(anchors) = source.get(&value) {
                    dest_anchors.extend(anchors.iter().cloned());
                }
                if dest_anchors.len() != old_len { changed = true; }
            }
        }
        let anchor_places: HashSet<_> = dest.raw_pointer_anchor_storage.keys()
            .chain(src.raw_pointer_anchor_storage.keys()).cloned().collect();
        for place in anchor_places {
            let dest_has = dest.raw_pointer_anchor_storage.contains_key(&place);
            let src_has = src.raw_pointer_anchor_storage.contains_key(&place);
            let dest_anchors = dest.raw_pointer_anchor_storage.entry(place.clone()).or_default();
            let old_len = dest_anchors.len();
            if !dest_has || !src_has {
                dest_anchors.insert(RawPointerAnchorSource::Unknown);
            }
            if let Some(anchors) = src.raw_pointer_anchor_storage.get(&place) {
                dest_anchors.extend(anchors.iter().cloned());
            }
            if dest_anchors.len() != old_len { changed = true; }
        }

        changed
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use luna_mvir::{ValueOrigin, BasicBlock, GlobalId, LabelId, Terminator, ValueData};
    use luna_semantic::SemanticTypeId;

    fn make_test_func(insts: Vec<Instruction>, terminator: Terminator) -> Function {
        let mut func = Function {
            name: GlobalId {
                name: "test".to_string(),
                symbol_id: None,
            },
            arg_count: 0,
        link_name: None,
        param_types: vec![],
            is_extern: false,
            is_async: false,
            ret_ty: SemanticTypeId(0),
            blocks: vec![],
            values: vec![],
        };

        let mut block = BasicBlock {
            label: LabelId {
                name: "0".to_string(),
            },
            insts: vec![],
            terminator: Some(terminator),
        };

        for (i, inst) in insts.into_iter().enumerate() {
            func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
                inst,
                ty: SemanticTypeId(0),
            });
            block.insts.push(ValueId(i as u32));
        }

        func.blocks.push(block);
        func
    }

    #[test]
    fn test_1_no_access() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Alloca, // arg1
                Instruction::Load {
                    ptr: Operand::Value(ValueId(0)),
                }, // Read arg0
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0), ValueId(1)], None);
        assert_eq!(summary.args[0].access, AccessKind::Read);
        assert_eq!(summary.args[1].access, AccessKind::None); // arg1 untouched
        assert_eq!(summary.args[1].escape, EscapeKind::CallOnly);
    }

    #[test]
    fn test_2_read_alias() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Alloca, // v1 (ValueId 1)
                Instruction::Store {
                    ptr: Operand::Value(ValueId(1)),
                    value: Operand::Value(ValueId(0)),
                },
                Instruction::Load {
                    ptr: Operand::Value(ValueId(1)),
                }, // v2 (ValueId 3)
                Instruction::Load {
                    ptr: Operand::Value(ValueId(3)),
                }, // Load v2 -> triggers Read on arg0
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        assert_eq!(summary.args[0].access, AccessKind::Read);
    }

    #[test]
    fn test_3_write_alias() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Alloca, // v1
                Instruction::Store {
                    ptr: Operand::Value(ValueId(1)),
                    value: Operand::Value(ValueId(0)),
                },
                Instruction::Load {
                    ptr: Operand::Value(ValueId(1)),
                }, // v2
                Instruction::Store {
                    ptr: Operand::Value(ValueId(3)),
                    value: Operand::Number("1".to_string()),
                }, // Store to v2 -> triggers Write on arg0
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        // The store ptr: v1, value: arg0 causes arg0 to be tainted.
        // Then load v1 -> v2 causes v2 to be tainted with arg0.
        // Then store ptr: v2 -> this triggers Write on arg0.
        // However, the `store v1, arg0` ALSO triggers a Write effect on `v1` if `v1` were tracked.
        // But for `arg0`, `store v1, arg0` does not trigger Read/Write on `arg0` itself, it only taints `v1`.
        // Wait, why did the test fail with ReadWrite?
        // Ah, `Instruction::Load { ptr: v1 }` to create v2. The ptr `v1` is not tainted with `arg0`.
        // Wait, if `v1` is not tainted with `arg0`, `get_taints(v1)` is empty.
        // But `add_taint` for `load v1` makes `v2` inherit taints from `v1`.
        // If `v1` is tainted with `arg0`, then `v2` is tainted with `arg0`.
        // Why does it result in ReadWrite?
        // Let's re-examine `Instruction::Load { ptr: v1 }`.
        // Our transfer rule says: for arg_idx in get_taints(ptr), access = access.merge(Read).
        // Since `v1` is tainted with `arg0` (from `store v1, arg0`), `get_taints(v1)` contains `arg0`.
        // So `load v1` triggers a Read on `arg0`!
        // Then `store v2` triggers a Write on `arg0`.
        // Hence, Read merged with Write -> ReadWrite!
        assert_eq!(summary.args[0].access, AccessKind::Write);
    }

    #[test]
    fn test_4_branch_merge() {
        // arg0 -> read in branch 1, write in branch 2
        let mut func = Function {
            name: GlobalId { name: "test".to_string(), symbol_id: None },
            arg_count: 0,
        link_name: None,
        param_types: vec![],
            is_extern: false,
            is_async: false,
            ret_ty: SemanticTypeId(0),
            blocks: vec![],
            values: vec![],
        };

        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Alloca,
            ty: SemanticTypeId(0),
        }); // arg0 (0)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Load {
                ptr: Operand::Value(ValueId(0)),
            },
            ty: SemanticTypeId(0),
        }); // Read (1)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Store {
                ptr: Operand::Value(ValueId(0)),
                value: Operand::Number("1".to_string()),
            },
            ty: SemanticTypeId(0),
        }); // Write (2)

        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "entry".to_string(),
            },
            insts: vec![ValueId(0)],
            terminator: Some(Terminator::CondBr {
                condition: Operand::Boolean(true),
                true_target: LabelId {
                    name: "left".to_string(),
                },
                false_target: LabelId {
                    name: "right".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "left".to_string(),
            },
            insts: vec![ValueId(1)],
            terminator: Some(Terminator::Br {
                target: LabelId {
                    name: "end".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "right".to_string(),
            },
            insts: vec![ValueId(2)],
            terminator: Some(Terminator::Br {
                target: LabelId {
                    name: "end".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "end".to_string(),
            },
            insts: vec![],
            terminator: Some(Terminator::Ret { value: None }),
        });

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        assert_eq!(summary.args[0].access, AccessKind::ReadWrite); // Read merged with Write
    }

    #[test]
    fn test_5_loop_convergence() {
        // while cond { arg0 = arg0; read(arg0) }
        let mut func = Function {
            name: GlobalId { name: "test".to_string(), symbol_id: None },
            arg_count: 0,
        link_name: None,
        param_types: vec![],
            is_extern: false,
            is_async: false,
            ret_ty: SemanticTypeId(0),
            blocks: vec![],
            values: vec![],
        };

        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Alloca,
            ty: SemanticTypeId(0),
        }); // arg0 (0)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Alloca,
            ty: SemanticTypeId(0),
        }); // tmp (1)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Store {
                ptr: Operand::Value(ValueId(1)),
                value: Operand::Value(ValueId(0)),
            },
            ty: SemanticTypeId(0),
        }); // store arg0 -> tmp (2)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Load {
                ptr: Operand::Value(ValueId(1)),
            },
            ty: SemanticTypeId(0),
        }); // load tmp (3)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Load {
                ptr: Operand::Value(ValueId(3)),
            },
            ty: SemanticTypeId(0),
        }); // read(tmp) -> Read arg0 (4)

        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "entry".to_string(),
            },
            insts: vec![ValueId(0), ValueId(1), ValueId(2)],
            terminator: Some(Terminator::Br {
                target: LabelId {
                    name: "loop".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "loop".to_string(),
            },
            insts: vec![ValueId(3), ValueId(4)],
            terminator: Some(Terminator::CondBr {
                condition: Operand::Boolean(true),
                true_target: LabelId {
                    name: "loop".to_string(),
                },
                false_target: LabelId {
                    name: "end".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "end".to_string(),
            },
            insts: vec![],
            terminator: Some(Terminator::Ret { value: None }),
        });

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        // The store `store arg0 -> tmp` doesn't do a Read on `arg0`.
        // The `load tmp` causes `Read` on `arg0` because `tmp` is tainted with `arg0`.
        // Then `load v_loaded_from_tmp` also causes Read.
        assert_eq!(summary.args[0].access, AccessKind::Read);
    }

    #[test]
    fn test_6_multiple_aliases() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Alloca, // arg1
                Instruction::Alloca, // v (ValueId 2)
                Instruction::Store {
                    ptr: Operand::Value(ValueId(2)),
                    value: Operand::Value(ValueId(0)),
                }, // v taints arg0
                Instruction::Store {
                    ptr: Operand::Value(ValueId(2)),
                    value: Operand::Value(ValueId(1)),
                }, // v taints arg1
                Instruction::Load {
                    ptr: Operand::Value(ValueId(2)),
                }, // load v (ValueId 5)
                Instruction::Store {
                    ptr: Operand::Value(ValueId(5)),
                    value: Operand::Number("0".to_string()),
                }, // write v
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0), ValueId(1)], None);
        // Wait, store overwrites the value, but our simple taint tracking just accumulates taints for the pointer.
        // It's a conservative local alias tracker, so BOTH arg0 and arg1 will get the Write effect!
        // Also, the `load v` triggers a Read on whatever `v` points to (arg0 and arg1).
        // So the final access is ReadWrite for both.
        assert_eq!(summary.args[0].access, AccessKind::Write);
        assert_eq!(summary.args[1].access, AccessKind::Write);
    }

    #[test]
    fn test_7_unresolved_call_name_does_not_imply_ffi_effects() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Alloca, // arg1
                Instruction::CallDirect {
                    callee: GlobalId {
                        name: "extern_mutate".to_string(),
                        symbol_id: None,
                    },
                    args: vec![Operand::Value(ValueId(0)), Operand::Value(ValueId(1))],
                },
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0), ValueId(1)], None);
        // A function's spelling is not evidence that it is an extern. The
        // typed extern contract is applied only when semantic symbol metadata
        // identifies the callee as extern (covered by driver-level tests).
        assert_eq!(summary.args[0].access, AccessKind::Unknown);
        assert_eq!(summary.args[0].escape, EscapeKind::Unknown);
        assert_eq!(summary.args[1].access, AccessKind::Unknown);
    }

    #[test]
    fn test_8_global_pointer_escape() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Store {
                    ptr: Operand::Global(GlobalId {
                        name: "G".to_string(),
                        symbol_id: None,
                    }),
                    value: Operand::Value(ValueId(0)),
                },
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        assert_eq!(summary.args[0].access, AccessKind::None); // Storing the pointer doesn't write TO the pointer
        assert_eq!(summary.args[0].escape, EscapeKind::MayEscape); // But it does escape
    }

    #[test]
    fn test_9_return_borrows_from() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Borrow {
                    is_rw: false,
                    base: Operand::Value(ValueId(0)),
                },
            ],
            Terminator::Ret {
                value: Some(Operand::Value(ValueId(1))),
            },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        // Without semantic types, the formal may itself carry another loan.
        // Both source channels must survive rather than discarding the latter.
        assert_eq!(summary.ret, ReturnEffect::BorrowsBoth { direct: vec![0], carried: vec![0] });
    }

    #[test]
    fn test_10_conditional_return_provenance() {
        let mut func = Function {
            name: GlobalId { name: "test".to_string(), symbol_id: None },
            arg_count: 0,
        link_name: None,
        param_types: vec![],
            is_extern: false,
            is_async: false,
            ret_ty: SemanticTypeId(0),
            blocks: vec![],
            values: vec![],
        };

        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Alloca,
            ty: SemanticTypeId(0),
        }); // arg0 (0)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Alloca,
            ty: SemanticTypeId(0),
        }); // arg1 (1)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Borrow {
                is_rw: false,
                base: Operand::Value(ValueId(0)),
            },
            ty: SemanticTypeId(0),
        }); // (2)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Borrow {
                is_rw: false,
                base: Operand::Value(ValueId(1)),
            },
            ty: SemanticTypeId(0),
        }); // (3)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Alloca,
            ty: SemanticTypeId(0),
        }); // ret_val (4)

        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Store {
                ptr: Operand::Value(ValueId(4)),
                value: Operand::Value(ValueId(2)),
            },
            ty: SemanticTypeId(0),
        }); // (5)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Store {
                ptr: Operand::Value(ValueId(4)),
                value: Operand::Value(ValueId(3)),
            },
            ty: SemanticTypeId(0),
        }); // (6)
        func.values.push(ValueData { span: None, origin: ValueOrigin::Temporary,
            inst: Instruction::Load {
                ptr: Operand::Value(ValueId(4)),
            },
            ty: SemanticTypeId(0),
        }); // (7)

        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "entry".to_string(),
            },
            insts: vec![ValueId(0), ValueId(1), ValueId(2), ValueId(3), ValueId(4)],
            terminator: Some(Terminator::CondBr {
                condition: Operand::Boolean(true),
                true_target: LabelId {
                    name: "left".to_string(),
                },
                false_target: LabelId {
                    name: "right".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "left".to_string(),
            },
            insts: vec![ValueId(5)],
            terminator: Some(Terminator::Br {
                target: LabelId {
                    name: "end".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "right".to_string(),
            },
            insts: vec![ValueId(6)],
            terminator: Some(Terminator::Br {
                target: LabelId {
                    name: "end".to_string(),
                },
            }),
        });
        func.blocks.push(BasicBlock {
            label: LabelId {
                name: "end".to_string(),
            },
            insts: vec![ValueId(7)],
            terminator: Some(Terminator::Ret {
                value: Some(Operand::Value(ValueId(7))),
            }),
        });

        let summary = EffectInference::infer(&func, vec![ValueId(0), ValueId(1)], None);
        assert_eq!(summary.ret, ReturnEffect::BorrowsBoth { direct: vec![0, 1], carried: vec![0, 1] });
    }

    #[test]
    fn test_11_unknown_call_conservative() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::CallDirect {
                    callee: GlobalId {
                        name: "some_unknown".to_string(),
                        symbol_id: None,
                    },
                    args: vec![Operand::Value(ValueId(0))],
                },
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        assert_eq!(summary.args[0].access, AccessKind::Unknown);
        assert_eq!(summary.args[0].ownership, OwnershipKind::Unknown);
        assert_eq!(summary.args[0].escape, EscapeKind::Unknown);
    }

    #[test]
    fn test_12_arithmetic_does_not_consume() {
        let func = make_test_func(
            vec![
                Instruction::Alloca, // arg0
                Instruction::Add {
                    left: Operand::Value(ValueId(0)),
                    right: Operand::Number("1".to_string()),
                },
            ],
            Terminator::Ret { value: None },
        );

        let summary = EffectInference::infer(&func, vec![ValueId(0)], None);
        assert_eq!(summary.args[0].ownership, OwnershipKind::Copy); // Kept at default Copy, not Consume!
    }
}

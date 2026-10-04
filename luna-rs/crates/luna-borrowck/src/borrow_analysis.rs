use crate::dataflow::{DataflowAnalysis, DataflowEngine};
use crate::effect::{direct_raw_pointer_fields, CallEffectSummary, RawPointerAnchorReturnEffect, RawPointerAnchorSource, RawPointerReturnEffect, ReturnEffect};
use luna_common::{Diagnostic, DiagnosticCode};
use luna_mvir::{Function, GlobalId, Instruction, Operand, Terminator, ValueId, ValueOrigin};
use luna_semantic::ty::Mutability;
use luna_semantic::{SemanticContext, SemanticType};
use std::collections::{HashMap, HashSet};

pub(crate) fn type_has_borrow(ty: luna_semantic::SemanticTypeId, ctx: &SemanticContext) -> bool {
    match ctx.types.get(ty) {
        SemanticType::Reference(..) => true,
        SemanticType::Struct(_, args, fields) | SemanticType::Enum(_, args, fields) => {
            args.iter().any(|&a| type_has_borrow(a, ctx))
                || fields.iter().any(|&f| type_has_borrow(f, ctx))
        }
        SemanticType::Tuple(elems) => elems.iter().any(|&e| type_has_borrow(e, ctx)),
        _ => false,
    }
}

// --- Liveness Analysis ---

#[derive(Clone, Default, PartialEq, Eq)]
pub struct LivenessState {
    pub live: HashSet<ValueId>,
}

pub struct LivenessAnalyzer<'a> {
    pub func: &'a Function,
}

impl<'a> DataflowAnalysis<LivenessState> for LivenessAnalyzer<'a> {
    fn transfer_instruction(&mut self, val_id: ValueId, inst: &Instruction, state: &mut LivenessState) {
        // In backward dataflow, definition kills liveness
        state.live.remove(&val_id);

        // Uses make values live
        match inst {
            Instruction::Load { ptr } => {
                if let Operand::Value(v) = ptr { state.live.insert(*v); }
            }
            Instruction::MakeClosure { env_ptr, captures, .. } => {
                if let Operand::Value(v) = env_ptr { state.live.insert(*v); }
                for capture in captures { state.live.insert(capture.source); }
            }
            Instruction::MakeTraitObject { data_ptr, .. } => {
                if let Operand::Value(v) = data_ptr { state.live.insert(*v); }
            }
            Instruction::MakeSlice { data_ptr, len } => {
                if let Operand::Value(v) = data_ptr { state.live.insert(*v); }
                if let Operand::Value(v) = len { state.live.insert(*v); }
            }
            Instruction::DropVirt { obj } => {
                if let Operand::Value(v) = obj { state.live.insert(*v); }
            }
            Instruction::CallVirt { obj, args, .. } => {
                if let Operand::Value(v) = obj { state.live.insert(*v); }
                for arg in args {
                    if let Operand::Value(v) = arg { state.live.insert(*v); }
                }
            }
            Instruction::Store { ptr, value } | Instruction::StoreAnchored { ptr, value } => {
                if let Operand::Value(ptr_val) = ptr {
                    state.live.insert(*ptr_val);
                }
                if let Operand::Value(v) = value { state.live.insert(*v); }
            }
            Instruction::Borrow { base, .. } => {
                if let Operand::Value(v) = base { state.live.insert(*v); }
            }
            Instruction::CallDirect { args, .. } => {
                for arg in args {
                    if let Operand::Value(v) = arg { state.live.insert(*v); }
                }
            }
            Instruction::CallIndirect { callee, args } | Instruction::CallClosure { closure: callee, args } => {
                if let Operand::Value(v) = callee { state.live.insert(*v); }
                for arg in args {
                    if let Operand::Value(v) = arg { state.live.insert(*v); }
                }
            }
            Instruction::Add { left, right } | Instruction::Sub { left, right } | Instruction::Mul { left, right } | Instruction::Div { left, right } | Instruction::Rem { left, right } | Instruction::Eq { left, right } | Instruction::LessThan { left, right } | Instruction::LessOrEq { left, right } | Instruction::GreaterThan { left, right } | Instruction::GreaterOrEq { left, right } => {
                if let Operand::Value(v) = left { state.live.insert(*v); }
                if let Operand::Value(v) = right { state.live.insert(*v); }
            }
            Instruction::Drop { value, .. } => {
                if let Operand::Value(v) = value { state.live.insert(*v); }
            }
            Instruction::BoundsCheck { index, len } => {
                if let Operand::Value(v) = index { state.live.insert(*v); }
                if let Operand::Value(v) = len { state.live.insert(*v); }
            }
            Instruction::PtrOffset { ptr, offset } => {
                if let Operand::Value(v) = ptr { state.live.insert(*v); }
                if let Operand::Value(v) = offset { state.live.insert(*v); }
            }
            Instruction::Await { future } => {
                if let Operand::Value(v) = future { state.live.insert(*v); }
            }
            Instruction::Assign(op) | Instruction::Cast { value: op, .. } | Instruction::FieldPtr { base: op, .. } | Instruction::Extract { value: op, .. } | Instruction::Tag { value: op } => {
                if let Operand::Value(v) = op { state.live.insert(*v); }
            }
            Instruction::Alloca | _ => {}
        }
    }

    fn transfer_terminator(&mut self, term: &Terminator, state: &mut LivenessState) {
        match term {
            Terminator::Ret { value: Some(Operand::Value(v)) } => {
                state.live.insert(*v);
            }
            Terminator::CondBr { condition: Operand::Value(v), .. } => {
                state.live.insert(*v);
            }
            _ => {}
        }
    }

    fn merge(&mut self, dest: &mut LivenessState, src: &LivenessState) -> bool {
        let mut changed = false;
        for &val in &src.live {
            if dest.live.insert(val) {
                changed = true;
            }
        }
        changed
    }

    fn init_entry_state(&mut self, _func: &Function, _state: &mut LivenessState) {}
}

pub fn compute_liveness(func: &Function) -> (HashMap<String, LivenessState>, HashMap<ValueId, HashSet<ValueId>>, HashMap<ValueId, HashSet<ValueId>>, HashMap<String, HashSet<ValueId>>) {
    let mut analyzer = LivenessAnalyzer { func };
    // run_backward gives us the OUT state of each block
    let block_out = DataflowEngine::run_backward(func, &mut analyzer);
    
    // Now compute liveness BEFORE and AFTER each instruction
    let mut live_before = HashMap::new();
    let mut live_after = HashMap::new();
    let mut live_at_entry = HashMap::new();
    
    for block in &func.blocks {
        let mut current_state = block_out.get(&block.label.name).cloned().unwrap_or_default();
        if let Some(term) = &block.terminator {
            analyzer.transfer_terminator(term, &mut current_state);
        }
        
        for &val_id in block.insts.iter().rev() {
            live_after.insert(val_id, current_state.live.clone());
            analyzer.transfer_instruction(val_id, &func.value(val_id).inst, &mut current_state);
            live_before.insert(val_id, current_state.live.clone());
        }

        live_at_entry.insert(block.label.name.clone(), current_state.live.clone());
    }
    
    (block_out, live_before, live_after, live_at_entry)
}

// --- Borrow Analysis (NLL) ---

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Loan {
    pub id: ValueId,
    pub place: Operand,
    pub is_rw: bool,
}

fn type_has_raw_pointer(ty: luna_semantic::SemanticTypeId, ctx: &SemanticContext, seen: &mut HashSet<luna_semantic::SemanticTypeId>) -> bool {
    if !seen.insert(ty) { return false; }
    match ctx.types.get(ty) {
        SemanticType::Pointer(..) => true,
        SemanticType::Struct(_, args, fields) | SemanticType::Enum(_, args, fields) => {
            args.iter().chain(fields).any(|field| type_has_raw_pointer(*field, ctx, seen))
        }
        SemanticType::Tuple(fields) => fields.iter().any(|field| type_has_raw_pointer(*field, ctx, seen)),
        SemanticType::Array(inner, _) => type_has_raw_pointer(*inner, ctx, seen),
        _ => false,
    }
}

/// Address origin carried by a raw pointer. This is deliberately not a `Loan`:
/// raw pointers do not keep safe loans live. `Unknown` is an explicit
/// alternative so a CFG merge cannot discard an untracked/foreign origin and
/// accidentally certify the remaining known origin as exhaustive.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RawPointerOrigin {
    FromPlace { place: Operand, is_rw: bool },
    /// A known null pointer value is never promotable to a safe reference,
    /// even when carried in a type with an owner-anchor contract.
    Null,
    Unknown,
}

/// Type-level anchor evidence is a must-property, not an origin and not a
/// loan. `Unknown` remains explicit at all merges so one known branch cannot
/// certify an untracked branch.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum RawPointerAnchor {
    AnchoredTo(PlaceDesc),
    Unknown,
}

impl RawPointerOrigin {
    fn from_loan(loan: &Loan) -> Self {
        Self::FromPlace {
            place: loan.place.clone(),
            is_rw: loan.is_rw,
        }
    }

    fn as_loan(&self, id: ValueId, is_rw: bool) -> Option<Loan> {
        match self {
            Self::FromPlace { place, is_rw: origin_is_rw } if !is_rw || *origin_is_rw => {
                Some(Loan {
                    id,
                    place: place.clone(),
                    is_rw,
                })
            }
            Self::FromPlace { .. } | Self::Null | Self::Unknown => None,
        }
    }

    fn into_escaped_loan(&self, id: ValueId) -> Option<Loan> {
        match self {
            Self::FromPlace { place, is_rw } => Some(Loan {
                id,
                place: place.clone(),
                is_rw: *is_rw,
            }),
            Self::Null | Self::Unknown => None,
        }
    }

    fn is_local(&self, aliases: &HashMap<ValueId, Operand>, ending_locals: &HashSet<ValueId>) -> bool {
        match self {
            Self::FromPlace { place, .. } => {
                let mut current = place;
                while let Operand::Value(value) = current {
                    if let Some(alias) = aliases.get(value) {
                        current = alias;
                    } else {
                        break;
                    }
                }
                matches!(current, Operand::Value(value) if ending_locals.contains(value))
            }
            Self::Null | Self::Unknown => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Projection {
    Field(u32),
    Tuple(u32),
    Deref,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlaceDesc {
    pub root: Operand,
    pub projections: Vec<Projection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProvenanceSet {
    pub sources: HashSet<PlaceDesc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlapResult {
    DefinitelyOverlap,
    DefinitelyDisjoint,
    MayOverlap,
}

impl OverlapResult {
    pub fn is_conflict(&self) -> bool {
        matches!(self, OverlapResult::DefinitelyOverlap | OverlapResult::MayOverlap)
    }
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct BorrowStateData {
    /// Distinguishes a dataflow bottom block state from a real incoming state
    /// so must-anchor facts are not weakened on the first predecessor edge.
    initialized: bool,
    pub direct_provenance: HashMap<ValueId, HashSet<Loan>>,
    pub carried_provenance: HashMap<ValueId, HashSet<Loan>>,
    /// Candidate storage origins carried by raw pointers. These are not loans
    /// and are excluded from liveness/conflict checks until an explicit
    /// raw-to-safe `Borrow` promotes them.
    pub raw_pointer_provenance: HashMap<ValueId, HashSet<RawPointerOrigin>>,
    /// Analysis-local raw-pointer lineages. Unlike `raw_pointer_provenance`,
    /// these identify unknown raw values for temporary unsafe loans only.
    pub unsafe_raw_roots: HashMap<ValueId, HashSet<ValueId>>,
    pub raw_pointer_storage: HashMap<PlaceDesc, HashSet<RawPointerOrigin>>,
    pub unsafe_raw_root_storage: HashMap<PlaceDesc, HashSet<ValueId>>,
    /// Type-contract anchor facts are separate from raw address origins and
    /// from safe loans. They identify the logical owner whose lifetime bounds
    /// a pointer value after explicit unsafe establishment.
    pub raw_pointer_anchors: HashMap<ValueId, HashSet<RawPointerAnchor>>,
    /// Marks borrow-result values whose temporary safe loans originate at an
    /// unknown raw pointer. The value IDs are function-local analysis keys.
    pub unsafe_raw_root_loans: HashMap<ValueId, HashSet<ValueId>>,
    pub raw_pointer_anchor_storage: HashMap<PlaceDesc, HashSet<RawPointerAnchor>>,
    /// Raw pointers loaded from contract-declared fields require anchor
    /// evidence at any later raw-to-safe conversion.
    pub raw_pointer_requires_anchor: HashSet<ValueId>,
    pub raw_pointer_requires_anchor_storage: HashSet<PlaceDesc>,
    /// A raw-pointer field whose value was written through an opaque raw
    /// access cannot recover its type-level owner anchor from the field
    /// contract alone. The external writer did not establish that invariant.
    pub raw_pointer_anchor_invalidated_storage: HashSet<PlaceDesc>,
    /// A raw pointer stored in memory was overwritten through an opaque raw
    /// write. This kill marker prevents older fixed-point/CFG facts from
    /// resurrecting the pre-call pointer value at a later load.
    pub raw_pointer_value_invalidated_storage: HashSet<PlaceDesc>,
    pub field_provenance: HashMap<PlaceDesc, ProvenanceSet>,
    pub escaped_loans: HashSet<Loan>,
    pub aliases: HashMap<ValueId, Operand>,
    pub closure_captures: HashMap<ValueId, Vec<luna_semantic::CaptureMode>>,
    /// Capture facts follow the value stored in a canonical place, rather
    /// than the SSA identity of the address used by a particular load.
    pub closure_capture_storage: HashMap<PlaceDesc, Vec<luna_semantic::CaptureMode>>,
}

pub struct BorrowAnalyzer<'a> {
    pub diagnostics: Vec<Diagnostic>,
    live_before: HashMap<ValueId, HashSet<ValueId>>,
    live_after: HashMap<ValueId, HashSet<ValueId>>,
    live_at_entry: HashMap<String, HashSet<ValueId>>,
    loop_info: crate::cfg::LoopInfo,
    callee_summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
    ctx: Option<&'a SemanticContext>,
    safe_reference_values: HashSet<ValueId>,
    func: &'a Function,
    pub emit_diagnostics: bool,
}

impl<'a> BorrowAnalyzer<'a> {
    pub fn new(
        live_before: HashMap<ValueId, HashSet<ValueId>>,
        callee_summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
        ctx: Option<&'a SemanticContext>,
        func: &'a Function,
    ) -> Self {
        let loop_info = crate::cfg::analyze_loops(func);
        let safe_reference_values = reference_value_ids(&func.values, ctx);
        Self {
            diagnostics: Vec::new(),
            live_before,
            live_after: HashMap::new(),
            live_at_entry: HashMap::new(),
            loop_info,
            callee_summaries,
            ctx,
            safe_reference_values,
            func,
            emit_diagnostics: false,
        }
    }

    pub fn with_live_after(mut self, live_after: HashMap<ValueId, HashSet<ValueId>>) -> Self {
        self.live_after = live_after;
        self
    }

    pub fn with_live_at_entry(mut self, live_at_entry: HashMap<String, HashSet<ValueId>>) -> Self {
        self.live_at_entry = live_at_entry;
        self
    }

    pub fn analyze(
        func: &'a Function,
        summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
        ctx: Option<&'a SemanticContext>,
    ) -> Vec<Diagnostic> {
        Self::analyze_with_shadow(func, summaries, ctx).0
    }

    pub fn analyze_with_shadow(
        func: &'a Function,
        summaries: Option<&'a HashMap<GlobalId, CallEffectSummary>>,
        ctx: Option<&'a SemanticContext>,
    ) -> (Vec<Diagnostic>, Vec<crate::region_bridge::ShadowComparison>) {
        let (_, live_before, live_after, live_at_entry) = compute_liveness(func);
        let mut analyzer = Self::new(live_before, summaries, ctx, func)
            .with_live_after(live_after)
            .with_live_at_entry(live_at_entry);
        let block_states = DataflowEngine::run_forward(func, &mut analyzer);

        // Build Region context, solve, and realize for shadow checking
        let loop_info = crate::cfg::analyze_loops(func);
        let region_context = crate::region_bridge::RegionBorrowContext::build(func, &loop_info, ctx);
        let region_solution = luna_semantic::region::solve_region_graph(&region_context.graph);
        let region_realization = match luna_semantic::region::realize_regions(&region_solution, &region_context.facts) {
            Ok(r) => Some(r),
            Err(_) => None,
        };

        let mut shadow_comparisons = Vec::new();

        // Second pass to emit diagnostics with final computed state
        analyzer.emit_diagnostics = true;
        for block in &func.blocks {
            let mut current_state = block_states.get(&block.label.name).cloned().unwrap_or_default();
            for &val_id in &block.insts {
                let val_data = func.value(val_id);
                let initial_diag_count = analyzer.diagnostics.len();
                analyzer.transfer_instruction(val_id, &val_data.inst, &mut current_state);
                let emitted_diag = analyzer.diagnostics.get(initial_diag_count..).and_then(|slice| slice.first()).cloned();

                // Shadow check on carrier uses / live carriers
                if let Some(realization) = &region_realization {
                    let bridge = crate::region_bridge::RegionBorrowBridge::new(
                        &region_context,
                        &region_solution,
                        realization,
                    ).with_aliases(current_state.aliases.clone());

                    // Authoritative Region Engine check on call-site outlives preconditions (REGION-02A)
                    match &val_data.inst {
                        Instruction::CallDirect { callee, args } => {
                            let mut callee_sym_id = callee.symbol_id;
                            if callee_sym_id.is_none() {
                                if let Some(ctx) = analyzer.ctx {
                                    if let Some(sym_id) = ctx.symbol_table.lookup(&callee.name, luna_semantic::symbol::ScopeId(0)) {
                                        callee_sym_id = Some(sym_id);
                                    }
                                }
                            }
                            if let Some(sym_id) = callee_sym_id {
                                if let Some(ctx) = analyzer.ctx {
                                    if let Some(contract) = ctx.tables.fn_lifetime_contracts.get(&sym_id) {
                                        let has_receiver = contract.outlives_constraints.iter().any(|c| {
                                            matches!(c.longer, luna_semantic::CanonicalContractSubject::SelfVal)
                                                || matches!(c.shorter, luna_semantic::CanonicalContractSubject::SelfVal)
                                        });
                                        for obligation in contract.instantiate_call_preconditions(false) {
                                            let longer_idx = match obligation.longer_subject {
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::SelfVal,
                                                ) => 0,
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::Param(i),
                                                ) => if has_receiver { (i + 1) as usize } else { i as usize },
                                                _ => usize::MAX,
                                            };
                                            let shorter_idx = match obligation.shorter_subject {
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::SelfVal,
                                                ) => 0,
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::Param(i),
                                                ) => if has_receiver { (i + 1) as usize } else { i as usize },
                                                _ => usize::MAX,
                                            };
                                            if longer_idx < args.len() && shorter_idx < args.len() {
                                                if let Some(diag) = bridge.diagnose_call_outlives(
                                                    &obligation.longer_subject,
                                                    &args[longer_idx],
                                                    &obligation.shorter_subject,
                                                    &args[shorter_idx],
                                                    &current_state,
                                                    val_data.span.clone(),
                                                ) {
                                                    if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                                        analyzer.diagnostics.push(diag);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Instruction::CallVirt { obj, method_idx, args } => {
                            let mut trait_method_sym = None;
                            if let Some(ctx) = analyzer.ctx {
                                if let Operand::Value(obj_val) = obj {
                                    let obj_ty = analyzer.func.values[obj_val.0 as usize].ty;
                                    let dyn_trait_sym = match ctx.types.get(obj_ty) {
                                        SemanticType::DynTrait(sym) => Some(*sym),
                                        SemanticType::Reference(_, _, inner) => {
                                            if let SemanticType::DynTrait(sym) = ctx.types.get(*inner) {
                                                Some(*sym)
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    };
                                    if let Some(trait_sym) = dyn_trait_sym {
                                        if let Some(method_syms) = ctx.tables.trait_methods.get(&trait_sym) {
                                            if let Some(&m_sym) = method_syms.get(*method_idx as usize) {
                                                trait_method_sym = Some(m_sym);
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(m_sym) = trait_method_sym {
                                if let Some(ctx) = analyzer.ctx {
                                    if let Some(contract) = ctx.tables.fn_lifetime_contracts.get(&m_sym) {
                                        for obligation in contract.instantiate_call_preconditions(true) {
                                            let longer_op = match obligation.longer_subject {
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::SelfVal,
                                                ) => Some(obj),
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::Param(i),
                                                ) => args.get(i as usize),
                                                _ => None,
                                            };
                                            let shorter_op = match obligation.shorter_subject {
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::SelfVal,
                                                ) => Some(obj),
                                                luna_semantic::region::LifetimeSubject::Root(
                                                    luna_semantic::region::LifetimeSubjectRoot::Param(i),
                                                ) => args.get(i as usize),
                                                _ => None,
                                            };
                                            if let (Some(l_op), Some(s_op)) = (longer_op, shorter_op) {
                                                if let Some(diag) = bridge.diagnose_call_outlives(
                                                    &obligation.longer_subject,
                                                    l_op,
                                                    &obligation.shorter_subject,
                                                    s_op,
                                                    &current_state,
                                                    val_data.span.clone(),
                                                ) {
                                                    if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                                        analyzer.diagnostics.push(diag);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Instruction::Store { ptr, value } | Instruction::StoreAnchored { ptr, value } => {
                            let mut field_store_info = None;
                            if let Operand::Value(ptr_v) = ptr {
                                if (ptr_v.0 as usize) < analyzer.func.values.len() {
                                    if let Instruction::FieldPtr { base, field_idx, .. } = &analyzer.func.values[ptr_v.0 as usize].inst {
                                        field_store_info = Some((base.clone(), *field_idx));
                                    }
                                }
                                if field_store_info.is_none() {
                                    let resolved = analyzer.resolve_alias(ptr, &current_state);
                                    if let Operand::Value(rv) = resolved {
                                        if (rv.0 as usize) < analyzer.func.values.len() {
                                            if let Instruction::FieldPtr { base, field_idx, .. } = &analyzer.func.values[rv.0 as usize].inst {
                                                field_store_info = Some((base.clone(), *field_idx));
                                            }
                                        }
                                    }
                                }
                            }

                            let effective_span = val_data.span.clone().or_else(|| {
                                if let Operand::Value(vv) = value {
                                    if (vv.0 as usize) < analyzer.func.values.len() {
                                        analyzer.func.values[vv.0 as usize].span.clone()
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            });

                            if let Some((base_op, field_idx)) = field_store_info {
                                let mut field_contract = None;
                                if let Some(ctx) = analyzer.ctx {
                                    let resolved_base_op = analyzer.resolve_alias(&base_op, &current_state);
                                    let base_vid = match resolved_base_op {
                                        Operand::Value(bv) => Some(*bv),
                                        _ => match &base_op {
                                            Operand::Value(bv) => Some(*bv),
                                            _ => None,
                                        },
                                    };
                                    if let Some(b_vid) = base_vid {
                                        if (b_vid.0 as usize) < analyzer.func.values.len() {
                                            let base_ty = analyzer.func.values[b_vid.0 as usize].ty;
                                            let resolved_ty = ctx.types.resolve(base_ty);
                                            let struct_sym = match ctx.types.get(resolved_ty) {
                                                SemanticType::Struct(s, ..) => Some(*s),
                                                SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                                                    if let SemanticType::Struct(s, ..) = ctx.types.get(*inner) {
                                                        Some(*s)
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => None,
                                            };
                                            if let Some(s_sym) = struct_sym {
                                                if let Some(contract) = ctx.tables.type_lifetime_contracts.get(&s_sym) {
                                                    field_contract = Some(contract);
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(contract) = field_contract {
                                    let field_is_constrained = contract.outlives_constraints.iter().any(|c| {
                                        match &c.longer {
                                            luna_semantic::CanonicalTypeLifetimeSubject::Field(fp) => {
                                                fp.0.first().copied() == Some(field_idx as u16)
                                            }
                                            _ => false,
                                        }
                                    });
                                    if field_is_constrained {
                                        if let Some(diag) = bridge.diagnose_field_store_outlives(
                                            field_idx,
                                            value,
                                            &base_op,
                                            &current_state,
                                            effective_span.clone(),
                                        ) {
                                            if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                                analyzer.diagnostics.push(diag);
                                            }
                                        }
                                    }
                                }
                            } else {
                                let mut agg_contract = None;
                                if let Some(ctx) = analyzer.ctx {
                                    let mut check_tys = Vec::new();
                                    if let Operand::Value(vv) = value {
                                        if (vv.0 as usize) < analyzer.func.values.len() {
                                            check_tys.push(analyzer.func.values[vv.0 as usize].ty);
                                        }
                                    }
                                    if let Operand::Value(pv) = ptr {
                                        if (pv.0 as usize) < analyzer.func.values.len() {
                                            check_tys.push(analyzer.func.values[pv.0 as usize].ty);
                                        }
                                    }
                                    for ty in check_tys {
                                        let resolved_ty = ctx.types.resolve(ty);
                                        let struct_sym = match ctx.types.get(resolved_ty) {
                                            SemanticType::Struct(s, ..) => Some(*s),
                                            SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                                                if let SemanticType::Struct(s, ..) = ctx.types.get(*inner) {
                                                    Some(*s)
                                                } else {
                                                    None
                                                }
                                            }
                                            _ => None,
                                        };
                                        if let Some(s_sym) = struct_sym {
                                            if let Some(contract) = ctx.tables.type_lifetime_contracts.get(&s_sym) {
                                                agg_contract = Some(contract);
                                                break;
                                            }
                                        }
                                    }
                                }

                                if let Some(contract) = agg_contract {
                                    if let Some(diag) = bridge.diagnose_type_contract_instance(
                                        contract,
                                        value,
                                        ptr,
                                        &current_state,
                                        effective_span.clone(),
                                    ) {
                                        if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                            analyzer.diagnostics.push(diag);
                                        }
                                    }
                                }
                            }
                        }
                        Instruction::Assign(op) => {
                            let mut agg_contract = None;
                            if let Some(ctx) = analyzer.ctx {
                                if (val_id.0 as usize) < analyzer.func.values.len() {
                                    let ty = analyzer.func.values[val_id.0 as usize].ty;
                                    let resolved_ty = ctx.types.resolve(ty);
                                    let struct_sym = match ctx.types.get(resolved_ty) {
                                        SemanticType::Struct(s, ..) => Some(*s),
                                        SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                                            if let SemanticType::Struct(s, ..) = ctx.types.get(*inner) {
                                                Some(*s)
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    };
                                    if let Some(s_sym) = struct_sym {
                                        if let Some(contract) = ctx.tables.type_lifetime_contracts.get(&s_sym) {
                                            agg_contract = Some(contract);
                                        }
                                    }
                                }
                            }
                            if let Some(contract) = agg_contract {
                                if let Some(diag) = bridge.diagnose_type_contract_instance(
                                    contract,
                                    op,
                                    &Operand::Value(val_id),
                                    &current_state,
                                    val_data.span.clone(),
                                ) {
                                    if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                        analyzer.diagnostics.push(diag);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }

                    if let Some(live_carriers) = analyzer.live_after.get(&val_id) {
                        for &carrier in live_carriers {
                            let provenance = analyzer.carrier_provenance_places(carrier, &current_state);
                            if !provenance.is_empty() {
                                let shadow_verdict = bridge.check_carrier_use(carrier, &provenance, val_id);
                                let legacy_verdict = if let Some(diag) = &emitted_diag {
                                    crate::region_bridge::LegacyVerdict::Rejected {
                                        diagnostic_code: format!("{:?}", diag.code),
                                        message: diag.message.clone(),
                                    }
                                } else {
                                    crate::region_bridge::LegacyVerdict::Allowed
                                };
                                let comparison = bridge.evaluate_comparison(
                                    format!("inst_{}", val_id.0),
                                    shadow_verdict,
                                    legacy_verdict,
                                );
                                shadow_comparisons.push(comparison);
                            }
                        }
                    }
                }
            }

            if let Some(term) = &block.terminator {
                // Authoritative Region Engine check on function return / escape
                if let Some(realization) = &region_realization {
                    let bridge = crate::region_bridge::RegionBorrowBridge::new(
                        &region_context,
                        &region_solution,
                        realization,
                    ).with_aliases(current_state.aliases.clone());

                    if let Terminator::Ret { value: Some(Operand::Value(val)) } = term {
                        if !analyzer.unsafe_raw_loans_for_carrier(*val, &current_state).is_empty() {
                            analyzer.emit_raw_escape_diagnostic(
                                *val,
                                "a reference derived from unknown raw provenance cannot be returned, directly or inside an aggregate",
                            );
                        }
                                                let mut ret_loans = analyzer.carrier_provenance_places(*val, &current_state);
let val_op = Operand::Value(*val);
                        let resolved_op = analyzer.resolve_alias(&val_op, &current_state);
                        if let Operand::Value(res_v) = resolved_op {
                            if *res_v != *val {
                                ret_loans.extend(analyzer.carrier_provenance_places(*res_v, &current_state));
                            }
                        }

                        let closure_modes = current_state.closure_captures.get(val)
                            .or_else(|| match resolved_op {
                                Operand::Value(source) => current_state.closure_captures.get(source),
                                _ => None,
                            }).cloned();
                        let is_ref_ret = if let Some(ctx) = analyzer.ctx {
                            type_has_borrow(func.ret_ty, ctx)
                        } else {
                            false
                        } || (closure_modes.is_some() && !ret_loans.is_empty());
                        let is_direct_ref_ret = if let Some(ctx) = analyzer.ctx {
                            matches!(ctx.types.get(func.ret_ty), SemanticType::Reference(..))
                        } else {
                            false
                        };
                        let fn_sym_id = func.name.symbol_id.or_else(|| {
                            analyzer.ctx.and_then(|c| c.symbol_table.lookup(&func.name.name, luna_semantic::symbol::ScopeId(0)))
                        });
                        let declared_contract = fn_sym_id.and_then(|sym| {
                            analyzer.ctx.and_then(|c| c.tables.fn_lifetime_contracts.get(&sym))
                        });

                        if let Some(diag) = bridge.diagnose_return(
                            *val,
                            &ret_loans,
                            closure_modes.as_deref(),
                            func.value(*val).span.clone(),
                            declared_contract,
                            is_ref_ret,
                            is_direct_ref_ret,
                        ) {
                            if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                analyzer.diagnostics.push(diag);
                            }
                        }
                    }
                }

                analyzer.transfer_terminator(term, &mut current_state);

                let successors = match term {
                    Terminator::Br { target } => vec![target.name.clone()],
                    Terminator::CondBr { true_target, false_target, .. } => {
                        vec![true_target.name.clone(), false_target.name.clone()]
                    }
                    _ => vec![],
                };
                for succ in successors {
                    let mut edge_state = current_state.clone();
                    analyzer.transfer_edge(&block.label.name, &succ, &mut edge_state);

                    // Authoritative Region Engine check on carriers crossing this dynamic boundary
                    if let Some(realization) = &region_realization {
                        let bridge = crate::region_bridge::RegionBorrowBridge::new(
                            &region_context,
                            &region_solution,
                            realization,
                        ).with_aliases(current_state.aliases.clone());

                        let entry_live = analyzer.live_at_entry.get(&succ);
                        let ending_locals = analyzer.loop_info.ending_lifetime_locals(&block.label.name, &succ);

                        if !ending_locals.is_empty() {
                            for (carrier, loans) in &current_state.direct_provenance {
                                let is_carrier_live = entry_live.map_or(false, |live| live.contains(carrier));
                                if is_carrier_live {
                                    let provenance: Vec<Operand> = loans.iter().map(|l| l.place.clone()).collect();
                                    let loan_span = loans.iter().next().and_then(|l| {
                                        if (l.id.0 as usize) < func.values.len() {
                                            func.values[l.id.0 as usize].span.clone()
                                        } else {
                                            None
                                        }
                                    });

                                    let shadow_verdict = bridge.check_boundary_crossing(*carrier, &provenance, &block.label.name, &succ);

                                    if let Some(diag) = bridge.diagnose_boundary_crossing(
                                        *carrier,
                                        &provenance,
                                        &block.label.name,
                                        &succ,
                                        loan_span,
                                    ) {
                                        if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                            analyzer.diagnostics.push(diag.clone());
                                        }
                                    }

                                    let legacy_verdict = match &shadow_verdict {
                                        crate::region_bridge::ShadowRegionVerdict::Invalid(failure) => {
                                            let d = failure.clone().into_diagnostic();
                                            crate::region_bridge::LegacyVerdict::Rejected {
                                                diagnostic_code: format!("{:?}", d.code),
                                                message: d.message,
                                            }
                                        }
                                        _ => crate::region_bridge::LegacyVerdict::Allowed,
                                    };
                                    let comparison = bridge.evaluate_comparison(
                                        format!("edge_{}_{}_carrier_{}", block.label.name, succ, carrier.0),
                                        shadow_verdict,
                                        legacy_verdict,
                                    );
                                    shadow_comparisons.push(comparison);
                                }
                            }

                            for (carrier, loans) in &current_state.carried_provenance {
                                let is_carrier_live = entry_live.map_or(false, |live| live.contains(carrier));
                                if is_carrier_live {
                                    let provenance: Vec<Operand> = loans.iter().map(|l| l.place.clone()).collect();
                                    let loan_span = loans.iter().next().and_then(|l| {
                                        if (l.id.0 as usize) < func.values.len() {
                                            func.values[l.id.0 as usize].span.clone()
                                        } else {
                                            None
                                        }
                                    });

                                    let shadow_verdict = bridge.check_boundary_crossing(*carrier, &provenance, &block.label.name, &succ);

                                    if let Some(diag) = bridge.diagnose_boundary_crossing(
                                        *carrier,
                                        &provenance,
                                        &block.label.name,
                                        &succ,
                                        loan_span,
                                    ) {
                                        if !analyzer.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                                            analyzer.diagnostics.push(diag.clone());
                                        }
                                    }

                                    let legacy_verdict = match &shadow_verdict {
                                        crate::region_bridge::ShadowRegionVerdict::Invalid(failure) => {
                                            let d = failure.clone().into_diagnostic();
                                            crate::region_bridge::LegacyVerdict::Rejected {
                                                diagnostic_code: format!("{:?}", d.code),
                                                message: d.message,
                                            }
                                        }
                                        _ => crate::region_bridge::LegacyVerdict::Allowed,
                                    };
                                    let comparison = bridge.evaluate_comparison(
                                        format!("edge_{}_{}_carrier_{}", block.label.name, succ, carrier.0),
                                        shadow_verdict,
                                        legacy_verdict,
                                    );
                                    shadow_comparisons.push(comparison);
                                }
                            }
                        }
                    }
                }
            }
        }

        (analyzer.diagnostics, shadow_comparisons)
    }

    fn resolve_alias<'b>(&self, op: &'b Operand, state: &'b BorrowStateData) -> &'b Operand {
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

    /// MVIR lvalue addresses use the pointee type, rather than a pointer type.
    /// An indexed lvalue formed from a raw data pointer is nevertheless an
    /// address derived from that pointer; it must preserve that value's origin
    /// until the explicit Borrow boundary. This is not a scalar data dependency.
    fn is_raw_derived_index_address(&self, value: ValueId) -> bool {
        let mut current = value;
        let mut visited = HashSet::new();
        while visited.insert(current) {
            let Some(data) = self.func.values.get(current.0 as usize) else { return false };
            let Instruction::PtrOffset { ptr: Operand::Value(source), .. } = &data.inst else { return false };
            let Some(source_data) = self.func.values.get(source.0 as usize) else { return false };
            if self.ctx.is_some_and(|ctx| matches!(ctx.types.get(source_data.ty), SemanticType::Pointer(..))) {
                return true;
            }
            current = *source;
        }
        false
    }

    /// A field reached through a safe reference belongs to its referent, not
    /// to the local slot holding the reference value. Preserve the real parent
    /// loans when creating a projected reborrow.
    fn reference_projection_loans(&self, place: &Operand, state: &BorrowStateData) -> Option<HashSet<Loan>> {
        let &Operand::Value(mut value) = place else { return None };
        let mut visited = HashSet::new();
        while visited.insert(value) {
            let Some(data) = self.func.values.get(value.0 as usize) else { return None };
            let Instruction::FieldPtr { base: Operand::Value(parent), .. } = &data.inst else { return None };
            let parent_data = self.func.values.get(parent.0 as usize)?;
            if !matches!(parent_data.inst, Instruction::Alloca | Instruction::FieldPtr { .. })
                && self.ctx.is_some_and(|ctx| matches!(ctx.types.get(parent_data.ty), SemanticType::Reference(..)))
            {
                let mut loans = HashSet::new();
                loans.extend(state.direct_provenance.get(parent).into_iter().flatten().cloned());
                loans.extend(state.carried_provenance.get(parent).into_iter().flatten().cloned());
                return (!loans.is_empty()).then_some(loans);
            }
            value = *parent;
        }
        None
    }

    fn is_projection_through_reference(&self, place: &Operand) -> bool {
        let &Operand::Value(mut value) = place else { return false };
        let mut visited = HashSet::new();
        while visited.insert(value) {
            let Some(data) = self.func.values.get(value.0 as usize) else { return false };
            let Instruction::FieldPtr { base: Operand::Value(parent), .. } = &data.inst else { return false };
            let Some(parent_data) = self.func.values.get(parent.0 as usize) else { return false };
            if self.ctx.is_some_and(|ctx| matches!(ctx.types.get(parent_data.ty), SemanticType::Reference(..))) {
                return true;
            }
            value = *parent;
        }
        false
    }

    fn carrier_provenance_places(&self, carrier: ValueId, state: &BorrowStateData) -> Vec<Operand> {
        let mut places = Vec::new();
        let mut add_loans = |loans: &HashSet<Loan>| {
            for l in loans {
                if !self.is_projection_through_reference(&l.place) {
                    places.push(l.place.clone());
                }
            }
        };
        if let Some(loans) = state.direct_provenance.get(&carrier) {
            add_loans(loans);
        }
        if let Some(loans) = state.carried_provenance.get(&carrier) {
            add_loans(loans);
        }
        places
    }

    fn call_reborrow_places(&self, argument: &Operand, state: &BorrowStateData) -> HashSet<Operand> {
        let mut places = HashSet::new();
        if let Operand::Value(value) = argument {
            if self.ctx.is_some_and(|ctx| ctx.types.contains_safe_reference(self.func.value(*value).ty)) {
                places.extend(state.direct_provenance.get(value).into_iter().flatten().map(|loan| loan.place.clone()));
                places.extend(state.carried_provenance.get(value).into_iter().flatten().map(|loan| loan.place.clone()));
            }
        }
        if places.is_empty() { places.insert(argument.clone()); }
        places
    }

    fn declared_anchor_owner(&self, field_ptr: ValueId, state: &BorrowStateData) -> Option<PlaceDesc> {
        let ctx = self.ctx?;
        let Instruction::FieldPtr { base, field_name: Some(field_name), .. } =
            &self.func.values.get(field_ptr.0 as usize)?.inst else { return None; };
        let base_value = match self.resolve_alias(base, state) {
            Operand::Value(v) => *v,
            _ => return None,
        };
        let mut owner_ty = ctx.types.resolve(self.func.values.get(base_value.0 as usize)?.ty);
        loop {
            match ctx.types.get(owner_ty) {
                SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => {
                    owner_ty = ctx.types.resolve(*inner);
                }
                SemanticType::Struct(owner_sym, ..) => {
                    let contract = ctx.tables.raw_storage_anchor_contracts.get(owner_sym)?;
                    if !contract.field_names.iter().any(|name| name == field_name) {
                        return None;
                    }
                    return Some(self.compute_place_desc(base, state));
                }
                _ => return None,
            }
        }
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

    fn raw_pointer_mutability(&self, arg: &Operand) -> Option<Mutability> {
        let ctx = self.ctx?;
        let Operand::Value(value) = arg else { return None; };
        match ctx.types.get(self.func.value(*value).ty) {
            SemanticType::Pointer(mutability, _) => Some(mutability.clone()),
            _ => None,
        }
    }

    /// Apply the memory access implied by a raw-pointer FFI parameter to its
    /// known pointee origins for this call only. Raw pointer origins remain a
    /// separate domain and are never inserted into `escaped_loans` here.
    fn check_ffi_raw_pointer_access(
        &mut self,
        arg: &Operand,
        call_id: ValueId,
        state: &BorrowStateData,
    ) -> Vec<PlaceDesc> {
        let Some(mutability) = self.raw_pointer_mutability(arg) else { return Vec::new(); };
        let Operand::Value(pointer) = arg else { return Vec::new(); };
        let origins = state.raw_pointer_provenance.get(pointer).cloned().unwrap_or_default();
        let mut written_places = Vec::new();
        for origin in origins {
            let RawPointerOrigin::FromPlace { place, .. } = origin else { continue; };
            self.check_access(&place, false, call_id, state);
            if mutability == Mutability::Mutable {
                self.check_access(&place, true, call_id, state);
                let target = self.compute_place_desc(&place, state);
                written_places.push(target);
            }
        }
        written_places
    }

    fn places_may_overlap(a: &PlaceDesc, b: &PlaceDesc) -> bool {
        if a.root != b.root { return false; }
        for (left, right) in a.projections.iter().zip(&b.projections) {
            match (left, right) {
                (Projection::Field(a), Projection::Field(b)) if a != b => return false,
                (Projection::Tuple(a), Projection::Tuple(b)) if a != b => return false,
                (Projection::Field(_) | Projection::Tuple(_), Projection::Field(_) | Projection::Tuple(_)) => {}
                _ => return true,
            }
        }
        // Equal places and a prefix/containing aggregate overlap.
        true
    }

    /// A raw `*rw` FFI argument may replace the value stored at its pointee.
    /// Forget stored-value facts (especially raw pointer origin/anchor facts)
    /// after the call; never derive them from the slot address.
    fn invalidate_ffi_written_facts(&self, written: &PlaceDesc, state: &mut BorrowStateData) {
        state.raw_pointer_anchor_invalidated_storage.insert(written.clone());
        state.raw_pointer_value_invalidated_storage.insert(written.clone());
        for (place, origins) in &mut state.raw_pointer_storage {
            if Self::places_may_overlap(place, written) {
                origins.clear();
                origins.insert(RawPointerOrigin::Unknown);
                state.raw_pointer_value_invalidated_storage.insert(place.clone());
            }
        }
        for (place, anchors) in &mut state.raw_pointer_anchor_storage {
            if Self::places_may_overlap(place, written) {
                anchors.clear();
                anchors.insert(RawPointerAnchor::Unknown);
                state.raw_pointer_anchor_invalidated_storage.insert(place.clone());
            }
        }
        state.raw_pointer_requires_anchor_storage.retain(|place| !Self::places_may_overlap(place, written));
        state.unsafe_raw_root_storage.retain(|place, _| !Self::places_may_overlap(place, written));
        state.field_provenance.retain(|place, _| !Self::places_may_overlap(place, written));
    }

    fn is_null_pointer_value(&self, value: &Operand) -> bool {
        fn visit(func: &Function, value: &Operand, depth: usize) -> bool {
            if depth > 16 { return false; }
            match value {
                Operand::Number(number) => number.trim().parse::<i128>().is_ok_and(|number| number == 0),
                Operand::Value(value_id) => match func.values.get(value_id.0 as usize).map(|v| &v.inst) {
                    Some(Instruction::Null { .. }) => true,
                    Some(Instruction::Cast { value, .. })
                    | Some(Instruction::Assign(value)) => visit(func, value, depth + 1),
                    _ => false,
                },
                _ => false,
            }
        }
        visit(self.func, value, 0)
    }

    fn anchor_owner_allows_mutable_borrow(&self, owner: &PlaceDesc, state: &BorrowStateData) -> bool {
        let Some(ctx) = self.ctx else { return false; };
        let Operand::Value(owner_value) = self.resolve_alias(&owner.root, state) else { return true; };
        match ctx.types.get(self.func.values.get(owner_value.0 as usize).map(|v| v.ty).unwrap_or(ctx.types.error_id())) {
            SemanticType::Reference(_, mutability, _) | SemanticType::Pointer(mutability, _) => {
                *mutability == Mutability::Mutable
            }
            // Without an explicit reference/pointer capability on the owner
            // place, borrow analysis cannot prove a mutable logical receiver.
            // Do not infer it merely from the raw field's `*rw` type.
            _ => false,
        }
    }

    fn invalidate_raw_pointer_anchors_for_owner(
        &self,
        owner: &PlaceDesc,
        except_value: Option<ValueId>,
        state: &mut BorrowStateData,
    ) {
        for (value, anchors) in &mut state.raw_pointer_anchors {
            if Some(*value) == except_value { continue; }
            if anchors.iter().any(|anchor| matches!(anchor, RawPointerAnchor::AnchoredTo(anchor_owner) if anchor_owner == owner)) {
                anchors.retain(|anchor| !matches!(anchor, RawPointerAnchor::AnchoredTo(anchor_owner) if anchor_owner == owner));
                anchors.insert(RawPointerAnchor::Unknown);
            }
        }
        for anchors in state.raw_pointer_anchor_storage.values_mut() {
            if anchors.iter().any(|anchor| matches!(anchor, RawPointerAnchor::AnchoredTo(anchor_owner) if anchor_owner == owner)) {
                anchors.retain(|anchor| !matches!(anchor, RawPointerAnchor::AnchoredTo(anchor_owner) if anchor_owner == owner));
                anchors.insert(RawPointerAnchor::Unknown);
            }
        }
    }

    fn copy_raw_pointer_storage_facts(&self, source: &PlaceDesc, destination: &PlaceDesc, state: &mut BorrowStateData) {
        if source == destination { return; }
        let raw_origins: Vec<_> = state.raw_pointer_storage.iter()
            .filter(|(place, _)| place.root == source.root && place.projections.starts_with(&source.projections))
            .map(|(place, facts)| {
                let mut target = destination.clone();
                target.projections.extend_from_slice(&place.projections[source.projections.len()..]);
                (target, facts.clone())
            }).collect();
        let raw_roots: Vec<_> = state.unsafe_raw_root_storage.iter()
            .filter(|(place, _)| place.root == source.root && place.projections.starts_with(&source.projections))
            .map(|(place, roots)| {
                let mut target = destination.clone();
                target.projections.extend_from_slice(&place.projections[source.projections.len()..]);
                (target, roots.clone())
            }).collect();
        let anchors: Vec<_> = state.raw_pointer_anchor_storage.iter()
            .filter(|(place, _)| place.root == source.root && place.projections.starts_with(&source.projections))
            .map(|(place, facts)| {
                let mut target = destination.clone();
                target.projections.extend_from_slice(&place.projections[source.projections.len()..]);
                let rebased = facts.iter().map(|anchor| match anchor {
                    RawPointerAnchor::AnchoredTo(owner)
                        if owner.root == source.root && owner.projections.starts_with(&source.projections) => {
                            let mut new_owner = destination.clone();
                            new_owner.projections.extend_from_slice(&owner.projections[source.projections.len()..]);
                            RawPointerAnchor::AnchoredTo(new_owner)
                        }
                    other => other.clone(),
                }).collect();
                (target, rebased)
            }).collect();
        let requires: Vec<_> = state.raw_pointer_requires_anchor_storage.iter()
            .filter(|place| place.root == source.root && place.projections.starts_with(&source.projections))
            .map(|place| {
                let mut target = destination.clone();
                target.projections.extend_from_slice(&place.projections[source.projections.len()..]);
                target
            }).collect();
        for (place, facts) in raw_origins { state.raw_pointer_storage.insert(place, facts); }
        for (place, roots) in raw_roots { state.unsafe_raw_root_storage.insert(place, roots); }
        for (place, facts) in anchors { state.raw_pointer_anchor_storage.insert(place, facts); }
        for place in requires { state.raw_pointer_requires_anchor_storage.insert(place); }
    }

    fn raw_value_roots(&self, state: &BorrowStateData, value: &Operand) -> HashSet<ValueId> {
        let Operand::Value(value_id) = value else { return HashSet::new(); };
        if let Some(roots) = state.unsafe_raw_roots.get(value_id) {
            return roots.clone();
        }
        let resolved = self.resolve_alias(value, state);
        if let Operand::Value(resolved_id) = resolved {
            state.unsafe_raw_roots.get(resolved_id).cloned()
                .unwrap_or_default()
        } else {
            HashSet::new()
        }
    }

    /// Convert a raw-value lineage into a disjoint analysis-only root for
    /// local unsafe loans. Synthetic IDs are outside this function's MVIR
    /// value range, so place normalization cannot follow a pointer load's
    /// alias back to the storage slot that held the pointer value.
    fn unsafe_raw_root_place(&self, root: ValueId) -> Operand {
        let synthetic = (self.func.values.len() as u32)
            .checked_add(root.0)
            .expect("function-local unsafe raw root identity overflow");
        Operand::Value(ValueId(synthetic))
    }

    fn unsafe_raw_loans_for_carrier(&self, carrier: ValueId, state: &BorrowStateData) -> HashSet<Loan> {
        let mut found = HashSet::new();
        let mut pending = vec![carrier];
        let mut seen = HashSet::new();
        while let Some(value) = pending.pop() {
            if !seen.insert(value) { continue; }
            found.extend(state.direct_provenance.get(&value).into_iter().flatten()
                .filter(|loan| state.unsafe_raw_root_loans.contains_key(&loan.id)).cloned());
            found.extend(state.carried_provenance.get(&value).into_iter().flatten()
                .filter(|loan| state.unsafe_raw_root_loans.contains_key(&loan.id)).cloned());
            if let Some(Operand::Value(alias)) = state.aliases.get(&value) { pending.push(*alias); }
        }
        found
    }

    fn is_function_local_storage(&self, place: &Operand, state: &BorrowStateData) -> bool {
        let mut current = match self.resolve_alias(place, state) {
            Operand::Value(value) => Some(*value),
            _ => None,
        };
        let mut seen = HashSet::new();
        while let Some(value) = current {
            if !seen.insert(value) { return false; }
            match self.func.values.get(value.0 as usize).map(|data| &data.inst) {
                Some(Instruction::Alloca) => return true,
                Some(Instruction::FieldPtr { base, .. }) => {
                    current = match self.resolve_alias(base, state) {
                        Operand::Value(base) => Some(*base),
                        _ => None,
                    };
                }
                _ => return false,
            }
        }
        false
    }

    fn active_loans(&self, val_id: ValueId, state: &BorrowStateData) -> HashSet<Loan> {
        let mut active = state.escaped_loans.clone();
        let mut queue = Vec::new();
        
        if let Some(live_set) = self.live_before.get(&val_id) {
            for &v in live_set {
                queue.push(v);
            }
        }
        
        let mut visited = std::collections::HashSet::new();

        while let Some(v) = queue.pop() {
            if visited.insert(v) {
                if let Some(loans) = state.direct_provenance.get(&v) {
                    for loan in loans {
                        active.insert(loan.clone());
                        let resolved = self.resolve_alias(&loan.place, state);
                        if let Operand::Value(place_v) = resolved {
                            queue.push(*place_v);
                        }
                    }
                }
                if let Some(loans) = state.carried_provenance.get(&v) {
                    for loan in loans {
                        active.insert(loan.clone());
                        let resolved = self.resolve_alias(&loan.place, state);
                        if let Operand::Value(place_v) = resolved {
                            queue.push(*place_v);
                        }
                    }
                }
            }
        }
        active
    }

    fn active_loans_after(&self, val_id: ValueId, state: &BorrowStateData) -> HashSet<Loan> {
        let mut active = state.escaped_loans.clone();
        let mut queue = Vec::new();
        
        if let Some(live_set) = self.live_after.get(&val_id) {
            for &v in live_set {
                queue.push(v);
            }
        }
        
        let mut visited = std::collections::HashSet::new();

        while let Some(v) = queue.pop() {
            if visited.insert(v) {
                if let Some(loans) = state.direct_provenance.get(&v) {
                    for loan in loans {
                        active.insert(loan.clone());
                        let resolved = self.resolve_alias(&loan.place, state);
                        if let Operand::Value(place_v) = resolved {
                            queue.push(*place_v);
                        }
                    }
                }
                if let Some(loans) = state.carried_provenance.get(&v) {
                    for loan in loans {
                        active.insert(loan.clone());
                        let resolved = self.resolve_alias(&loan.place, state);
                        if let Operand::Value(place_v) = resolved {
                            queue.push(*place_v);
                        }
                    }
                }
            }
        }
        active
    }

    fn is_external_loan(&self, loan: &Loan, state: &BorrowStateData) -> bool {
        // Only the loan that enters this function as a reference parameter is
        // external. A loan created locally (including one returned by a call)
        // may point at that same parameter, but it is still a live reborrow
        // which must prevent conflicting accesses through the parent value.
        // Classifying by `loan.place` incorrectly erased such reborrows merely
        // because their storage ultimately came from an external reference.
        let Some(val_data) = self.func.values.get(loan.id.0 as usize) else {
            return false;
        };
        match &val_data.origin {
            ValueOrigin::Global => true,
            ValueOrigin::Parameter(_) => self.ctx.is_some_and(|ctx| {
                matches!(ctx.types.get(val_data.ty), SemanticType::Reference(..))
            }),
            ValueOrigin::Local | ValueOrigin::Temporary => false,
        }
    }

    pub fn compute_place_desc(&self, op: &Operand, state: &BorrowStateData) -> PlaceDesc {
        compute_place_desc(op, &self.func.values, Some(&state.aliases), None)
    }

    fn compute_overlap_place_desc(&self, op: &Operand, state: &BorrowStateData) -> PlaceDesc {
        compute_place_desc(
            op,
            &self.func.values,
            Some(&state.aliases),
            Some(&self.safe_reference_values),
        )
    }

    pub fn compute_provenance_sources(&self, value: &Operand, state: &BorrowStateData) -> ProvenanceSet {
        let mut sources = HashSet::new();
        let mut check_ops = vec![value.clone()];
        let mut visited = HashSet::new();
        
        while let Some(op) = check_ops.pop() {
            let desc = self.compute_place_desc(&op, state);
            if let Operand::Value(v) = op {
                if !visited.insert(v) {
                    continue;
                }
                if (v.0 as usize) < self.func.values.len() {
                    let inst = &self.func.values[v.0 as usize].inst;
                    match inst {
                        Instruction::Borrow { base, .. } => {
                            sources.insert(self.compute_place_desc(base, state));
                        }
                        Instruction::Assign(base) | Instruction::Cast { value: base, .. } => {
                            check_ops.push(base.clone());
                        }
                        _ => {}
                    }
                }
                if let Some(alias) = state.aliases.get(&v) {
                    check_ops.push(alias.clone());
                }
                if let Some(loans) = state.direct_provenance.get(&v) {
                    for loan in loans {
                        sources.insert(self.compute_place_desc(&loan.place, state));
                    }
                }
                if let Some(loans) = state.carried_provenance.get(&v) {
                    for loan in loans {
                        sources.insert(self.compute_place_desc(&loan.place, state));
                    }
                }
            } else {
                sources.insert(desc);
            }
        }
        
        if sources.is_empty() {
            sources.insert(self.compute_place_desc(value, state));
        }
        
        ProvenanceSet { sources }
    }
    
    fn check_overlap(&self, p1: &Operand, p2: &Operand, state: &BorrowStateData) -> OverlapResult {
        let desc1 = self.compute_overlap_place_desc(p1, state);
        let desc2 = self.compute_overlap_place_desc(p2, state);
        
        if desc1.root != desc2.root {
            return OverlapResult::DefinitelyDisjoint;
        }
        
        let min_len = std::cmp::min(desc1.projections.len(), desc2.projections.len());
        for i in 0..min_len {
            match (&desc1.projections[i], &desc2.projections[i]) {
                (Projection::Field(f1), Projection::Field(f2)) => {
                    if f1 != f2 {
                        return OverlapResult::DefinitelyDisjoint;
                    }
                }
                (Projection::Tuple(t1), Projection::Tuple(t2)) => {
                    if t1 != t2 {
                        return OverlapResult::DefinitelyDisjoint;
                    }
                }
                (Projection::Deref, Projection::Deref) => {}
                _ => {
                    return OverlapResult::MayOverlap;
                }
            }
        }

        // The storage slot holding a reference is distinct from the place it
        // designates. A path which crosses a dereference boundary cannot
        // overlap a prefix that stops at the reference slot.
        if desc1.projections.len() != desc2.projections.len() {
            let longer = if desc1.projections.len() > desc2.projections.len() {
                &desc1.projections
            } else {
                &desc2.projections
            };
            if matches!(longer.get(min_len), Some(Projection::Deref)) {
                return OverlapResult::DefinitelyDisjoint;
            }
        }

        OverlapResult::DefinitelyOverlap
    }

    fn check_access(&mut self, place: &Operand, is_write: bool, val_id: ValueId, state: &BorrowStateData) {
        if !self.emit_diagnostics { return; }
        let resolved_place = self.resolve_alias(place, state);
        let place_name = print_operand_name(resolved_place);
        let active = self.active_loans(val_id, state);
        
        for loan in &active {
            if self.is_external_loan(loan, state) {
                continue;
            }
            // If the place being accessed is itself a reference holding this loan,
            // it is accessing through the borrow, not conflicting with it.
            if let Operand::Value(pv) = place {
                if let Some(prov) = state.direct_provenance.get(pv) {
                    if prov.contains(loan) {
                        continue;
                    }
                }
                if let Some(prov) = state.carried_provenance.get(pv) {
                    if prov.contains(loan) {
                        continue;
                    }
                }
            }
            if self.check_overlap(&loan.place, place, state).is_conflict() {
                let _resolved_loan_place = self.resolve_alias(&loan.place, state);
                let desc2 = self.compute_place_desc(place, state);
                let desc1 = self.compute_place_desc(&loan.place, state);
                if loan.is_rw {
                    let mut diag = Diagnostic::error(format!(
                        "Cannot access '{}' (root: {:?}) because it is borrowed as &rw (root: {:?})",
                        place_name, desc2.root, desc1.root
                    ))
                    .with_code(DiagnosticCode::BorrowConflict);
                    diag.span = self.func.values[val_id.0 as usize].span.clone();
                    if !self.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                        self.diagnostics.push(diag);
                    }
                } else if is_write {
                    let mut diag = Diagnostic::error(format!(
                        "Cannot write to '{}' because it is borrowed as &",
                        place_name
                    ))
                    .with_code(DiagnosticCode::BorrowConflict);
                    diag.span = self.func.values[val_id.0 as usize].span.clone();
                    if !self.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                        self.diagnostics.push(diag);
                    }
                }
            }
        }
    }

    fn issue_loan(&mut self, place: &Operand, is_rw: bool, val_id: ValueId, state: &mut BorrowStateData) {
        let resolved_place = self.resolve_alias(place, state);
        let place_name = print_operand_name(resolved_place);
        let active = self.active_loans(val_id, state);
        
        if self.emit_diagnostics {
            for loan in &active {
                if self.is_external_loan(loan, state) {
                    continue;
                }
                if self.check_overlap(&loan.place, place, state).is_conflict() {
                    if loan.is_rw {
                        let mut diag = Diagnostic::error(format!(
                            "Cannot borrow '{}' as {} because it is already borrowed as &rw",
                            place_name,
                            if is_rw { "&rw" } else { "&" }
                        ))
                        .with_code(DiagnosticCode::BorrowConflict);
                        diag.span = self.func.values[val_id.0 as usize].span.clone();
                        if !self.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                            self.diagnostics.push(diag);
                        }
                    } else if is_rw {
                        let mut diag = Diagnostic::error(format!(
                            "Cannot borrow '{}' as &rw because it is already borrowed as &",
                            place_name
                        ))
                        .with_code(DiagnosticCode::BorrowConflict);
                        diag.span = self.func.values[val_id.0 as usize].span.clone();
                        if !self.diagnostics.iter().any(|d| d.message == diag.message && d.span == diag.span) {
                            self.diagnostics.push(diag);
                        }
                    }
                }
            }
        }

        let new_loan = Loan {
            id: val_id,
            place: place.clone(),
            is_rw,
        };
        
        state.direct_provenance.entry(val_id).or_default().insert(new_loan);
    }

    fn issue_unsafe_root_loan(&mut self, root: ValueId, is_rw: bool, val_id: ValueId, state: &BorrowStateData) {
        if !self.emit_diagnostics { return; }
        for loan in self.active_loans(val_id, state) {
            if state.unsafe_raw_root_loans.get(&loan.id).is_some_and(|roots| roots.contains(&root)) && (loan.is_rw || is_rw) {
                let existing = if loan.is_rw { "&rw" } else { "&" };
                let requested = if is_rw { "&rw" } else { "&" };
                let mut diag = Diagnostic::error(format!(
                    "Cannot create {requested} from this raw pointer while its UnsafeRawRoot is already borrowed as {existing}"
                )).with_code(DiagnosticCode::BorrowConflict);
                diag.span = self.func.values[val_id.0 as usize].span.clone();
                if !self.diagnostics.iter().any(|old| old.message == diag.message && old.span == diag.span) {
                    self.diagnostics.push(diag);
                }
            }
        }
    }

    fn emit_raw_escape_diagnostic(&mut self, val_id: ValueId, message: &str) {
        if !self.emit_diagnostics { return; }
        let mut diag = Diagnostic::error(format!("UnsafeRawRoot loan cannot escape: {message}"))
            .with_code(DiagnosticCode::LocalBorrowEscape);
        diag.span = self.func.values.get(val_id.0 as usize).and_then(|value| value.span.clone());
        if !self.diagnostics.iter().any(|old| old.message == diag.message && old.span == diag.span) {
            self.diagnostics.push(diag);
        }
    }

    /// Returns the semantic safe-reference contract for one direct-call
    /// parameter when it can be proven from the canonical function type.
    /// MVIR may lower both safe references and raw pointers to pointer-shaped
    /// values, so call escape checking must inspect the callee's source-level
    /// signature rather than the lowered argument representation.
    fn direct_call_safe_ref_param(
        &self,
        callee: &GlobalId,
        arg_index: usize,
    ) -> Option<(Mutability, bool)> {
        let ctx = self.ctx?;
        let symbol = callee.symbol_id?;
        let fn_ty = ctx.tables.symbol_types.get(&symbol).copied()?;
        let fn_ty = ctx.types.resolve(fn_ty);
        let SemanticType::Function { params, return_type } = ctx.types.get(fn_ty) else {
            return None;
        };
        let param_ty = ctx.types.resolve(*params.get(arg_index)?);
        let SemanticType::Reference(_, mutability, _) = ctx.types.get(param_ty) else {
            return None;
        };

        // A potentially reference-bearing result may carry this temporary
        // UnsafeRawRoot loan out of the call. Generic/projection result types
        // deliberately count as potentially carrying a loan.
        let result_may_carry_safe_loan = ctx.types.contains_safe_reference(*return_type);
        Some((mutability.clone(), !result_may_carry_safe_loan))
    }
}

pub fn reference_value_ids(
    func_values: &[luna_mvir::ValueData],
    ctx: Option<&SemanticContext>,
) -> HashSet<ValueId> {
    let Some(ctx) = ctx else { return HashSet::new(); };
    func_values.iter().enumerate().filter_map(|(index, value)| {
        let ty = ctx.types.resolve(value.ty);
        matches!(ctx.types.get(ty), SemanticType::Reference(..))
            .then_some(ValueId(index as u32))
    }).collect()
}

pub fn compute_place_desc(
    op: &Operand,
    func_values: &[luna_mvir::ValueData],
    aliases: Option<&HashMap<ValueId, Operand>>,
    safe_reference_values: Option<&HashSet<ValueId>>,
) -> PlaceDesc {
    let mut curr_val = if let Operand::Value(v) = op { Some(*v) } else { None };
    let mut path = Vec::new();
    let mut root_val = None;
    
    while let Some(v) = curr_val {
        if (v.0 as usize) >= func_values.len() {
            root_val = Some(v);
            break;
        }
        let inst = &func_values[v.0 as usize].inst;
        match inst {
            Instruction::FieldPtr { base, field_idx, .. } => {
                path.push(Projection::Field(*field_idx));
                if let Operand::Value(base_v) = base {
                    curr_val = Some(*base_v);
                } else { break; }
            }
            Instruction::Extract { value, .. } => {
                path.push(Projection::Unknown);
                if let Operand::Value(base_v) = value {
                    curr_val = Some(*base_v);
                } else { break; }
            }
            Instruction::Load { ptr } => {
                let is_local_storage = if let Operand::Value(base_v) = ptr {
                    let mut check_v = Some(*base_v);
                    let mut is_local = false;
                    while let Some(cv) = check_v {
                        if (cv.0 as usize) >= func_values.len() { break; }
                        match &func_values[cv.0 as usize].inst {
                            Instruction::Alloca => {
                                is_local = true;
                                break;
                            }
                            Instruction::FieldPtr { base, .. } => {
                                if let Operand::Value(bv) = base {
                                    check_v = Some(*bv);
                                } else { break; }
                            }
                            _ => break,
                        }
                    }
                    is_local
                } else {
                    false
                };

                if safe_reference_values.is_some_and(|values| values.contains(&v)) || !is_local_storage {
                    path.push(Projection::Deref);
                }
                if let Operand::Value(base_v) = ptr {
                    curr_val = Some(*base_v);
                } else { break; }
            }
            Instruction::MakeSlice { data_ptr, .. } | Instruction::PtrOffset { ptr: data_ptr, .. } => {
                path.push(Projection::Unknown);
                if let Operand::Value(base_v) = data_ptr {
                    curr_val = Some(*base_v);
                } else { break; }
            }
            Instruction::Assign(base) | Instruction::Cast { value: base, .. } => {
                if let Operand::Value(base_v) = base {
                    curr_val = Some(*base_v);
                } else { break; }
            }
            Instruction::Add { left, right } | Instruction::Sub { left, right } => {
                path.push(Projection::Unknown);
                if let Operand::Value(base_v) = left {
                    curr_val = Some(*base_v);
                } else if let Operand::Value(base_v) = right {
                    curr_val = Some(*base_v);
                } else { break; }
            }
            _ => {
                root_val = Some(v);
                break;
            }
        }
    }
    
    path.reverse();
    
    let mut final_root = root_val.map(Operand::Value).unwrap_or_else(|| op.clone());
    if let Some(alias_map) = aliases {
        while let Operand::Value(v) = final_root {
            if let Some(alias) = alias_map.get(&v) {
                final_root = alias.clone();
            } else {
                break;
            }
        }
    }
    
    PlaceDesc {
        root: final_root,
        projections: path,
    }
}

fn print_operand_name(op: &Operand) -> String {
    match op {
        Operand::Value(val) => format!("%v{}", val.0),
        Operand::Global(id) => format!("@g{}", id.name.clone()),
        Operand::Block(id) => format!("block_{}", id.0),
        Operand::Number(n) => n.clone(),
        Operand::Boolean(b) => b.to_string(),
        Operand::StringRef(s) => format!("\"{}\"", s),
        Operand::Char(c) => format!("'{}'", c),
        Operand::Float { text, .. } => text.clone(),
    }
}

impl<'a> DataflowAnalysis<BorrowStateData> for BorrowAnalyzer<'a> {
    fn init_entry_state(&mut self, func: &Function, state: &mut BorrowStateData) {
        state.initialized = true;
        for (i, val_data) in func.values.iter().enumerate() {
            if let ValueOrigin::Parameter(_) = val_data.origin {
                let val_id = ValueId(i as u32);
                if self.ctx.is_some_and(|ctx| matches!(ctx.types.get(val_data.ty), SemanticType::Pointer(..))) {
                    // A raw-pointer parameter has an origin, but it is not
                    // known to this function until a caller supplies evidence.
                    state.raw_pointer_provenance.entry(val_id).or_default().insert(RawPointerOrigin::Unknown);
                    state.raw_pointer_anchors.entry(val_id).or_default().insert(RawPointerAnchor::Unknown);
                    state.unsafe_raw_roots.entry(val_id).or_default().insert(val_id);
                    if matches!(val_data.inst, Instruction::Alloca) {
                        state.unsafe_raw_root_storage.insert(
                            PlaceDesc { root: Operand::Value(val_id), projections: Vec::new() },
                            HashSet::from([val_id]),
                        );
                    }
                }
                let is_ref = if let Some(ctx) = self.ctx {
                    matches!(ctx.types.get(val_data.ty), SemanticType::Reference(..))
                } else {
                    false
                };
                if is_ref {
                    let is_rw = if let Some(ctx) = self.ctx {
                        matches!(
                            ctx.types.get(val_data.ty),
                            SemanticType::Reference(_, luna_semantic::ty::Mutability::Mutable, _)
                        )
                    } else {
                        false
                    };
                    let loan = Loan {
                        id: val_id,
                        place: Operand::Value(val_id),
                        is_rw,
                    };
                    state.direct_provenance.entry(val_id).or_default().insert(loan);
                } else if let Some(ctx) = self.ctx {
                    if type_has_borrow(val_data.ty, ctx) {
                        let val_id = ValueId(i as u32);
                        let loan = Loan {
                            id: val_id,
                            place: Operand::Value(val_id),
                            is_rw: false,
                        };
                        state.carried_provenance.entry(val_id).or_default().insert(loan);
                    }
                }
            }
        }
    }

    fn transfer_instruction(&mut self, val_id: ValueId, inst: &Instruction, state: &mut BorrowStateData) {
        match inst {
            Instruction::MakeClosure { env_ptr, captures, .. } => {
                self.check_access(env_ptr, false, val_id, state);
                state.closure_captures.insert(val_id, captures.iter().map(|capture| capture.mode).collect());
                for capture in captures {
                    if matches!(capture.mode, luna_semantic::CaptureMode::SharedBorrow | luna_semantic::CaptureMode::MutableBorrow) {
                        let source = Operand::Value(capture.source);
                        self.issue_loan(&source, capture.mode == luna_semantic::CaptureMode::MutableBorrow, val_id, state);
                    } else {
                        // Moving a reference or a borrowed aggregate into an
                        // environment transfers its existing referent loans.
                        // It does not borrow the old capture storage itself.
                        let mut loans = state.direct_provenance.get(&capture.source).cloned().unwrap_or_default();
                        loans.extend(state.carried_provenance.get(&capture.source).into_iter().flatten().cloned());
                        state.carried_provenance.entry(val_id).or_default().extend(loans);
                    }
                }
            }
            Instruction::MakeTraitObject { data_ptr, .. } => {
                self.check_access(data_ptr, false, val_id, state);
                if let Operand::Value(dp_v) = data_ptr {
                    let resolved = self.resolve_alias(data_ptr, state);
                    let check_val = if let Operand::Value(rv) = resolved { *rv } else { *dp_v };
                    state.aliases.insert(val_id, Operand::Value(check_val));
                    if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                        state.direct_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.direct_provenance.get(dp_v).cloned() {
                        state.direct_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.carried_provenance.get(dp_v).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                }
            }
            Instruction::MakeSlice { data_ptr, len } => {
                self.check_access(data_ptr, false, val_id, state);
                self.check_access(len, false, val_id, state);
                if let Operand::Value(dp_v) = data_ptr {
                    let resolved = self.resolve_alias(data_ptr, state);
                    let check_val = if let Operand::Value(rv) = resolved { *rv } else { *dp_v };
                    state.aliases.insert(val_id, Operand::Value(check_val));
                    if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                        state.direct_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.direct_provenance.get(dp_v).cloned() {
                        state.direct_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.carried_provenance.get(dp_v).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                }
            }
            Instruction::DropVirt { obj } => {
                self.check_access(obj, false, val_id, state);
            }
            Instruction::CallVirt { obj, method_idx, args } => {
                let mut is_write = false;
                let mut trait_method_sym = None;
                if let Some(ctx) = self.ctx {
                    if let Operand::Value(obj_v) = obj {
                        let obj_ty_id = ctx.types.resolve(self.func.values[obj_v.0 as usize].ty);
                        let obj_ty = ctx.types.get(obj_ty_id);
                        let dyn_trait_sym = match obj_ty {
                            SemanticType::DynTrait(sym) => Some(*sym),
                            SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => {
                                if let SemanticType::DynTrait(sym) = ctx.types.get(*inner) {
                                    Some(*sym)
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };
                        if let Some(trait_sym) = dyn_trait_sym {
                            if let Some(method_syms) = ctx.tables.trait_methods.get(&trait_sym) {
                                if let Some(&m_sym) = method_syms.get(*method_idx as usize) {
                                    trait_method_sym = Some(m_sym);
                                    if let Some(&m_ty) = ctx.tables.symbol_types.get(&m_sym) {
                                        if let SemanticType::Function { params, .. } = ctx.types.get(m_ty) {
                                            if let Some(&first_param) = params.first() {
                                                if matches!(ctx.types.get(first_param), SemanticType::Reference(_, luna_semantic::ty::Mutability::Mutable, _)) {
                                                    is_write = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.check_access(obj, is_write, val_id, state);
                for arg in args {
                    self.check_access(arg, false, val_id, state);
                }

                let is_future_ret = if let Some(ctx) = self.ctx {
                    matches!(ctx.types.get(self.func.values[val_id.0 as usize].ty), SemanticType::Future(..))
                } else {
                    false
                };

                if is_future_ret {
                    for op in std::iter::once(obj).chain(args.iter()) {
                        if let Operand::Value(op_v) = op {
                            let is_ref_bearing = if let Some(ctx) = self.ctx {
                                let ty_id = self.func.values[op_v.0 as usize].ty;
                                matches!(ctx.types.get(ty_id), SemanticType::Reference(..) | SemanticType::Pointer(..))
                            } else {
                                true
                            };
                            if is_ref_bearing {
                                let resolved = self.resolve_alias(op, state);
                                let resolved_v = if let Operand::Value(rv) = resolved { *rv } else { *op_v };
                                if let Some(prov) = state.direct_provenance.get(&resolved_v).cloned() {
                                    state.carried_provenance.entry(val_id).or_default().extend(prov);
                                }
                                if let Some(prov) = state.carried_provenance.get(&resolved_v).cloned() {
                                    state.carried_provenance.entry(val_id).or_default().extend(prov);
                                }
                                if let Some(prov) = state.direct_provenance.get(op_v).cloned() {
                                    state.carried_provenance.entry(val_id).or_default().extend(prov);
                                }
                                if let Some(prov) = state.carried_provenance.get(op_v).cloned() {
                                    state.carried_provenance.entry(val_id).or_default().extend(prov);
                                }
                            }
                        }
                    }
                }

                // Phase 3.6C: Apply CanonicalLifetimeContract from trait method
                if let Some(m_sym) = trait_method_sym {
                    if let Some(ctx) = self.ctx {
                        if let Some(contract) = ctx.tables.fn_lifetime_contracts.get(&m_sym) {
                            let all_args: Vec<&Operand> = std::iter::once(obj).chain(args.iter()).collect();

                            // Propagate return provenance from all_args
                            if let Some(prov) = &contract.return_provenance {
                                for &param_idx in prov.indices() {
                                    let idx = param_idx as usize;
                                    if idx < all_args.len() {
                                        if let Operand::Value(arg_v) = all_args[idx] {
                                            if let Some(p) = state.direct_provenance.get(arg_v).cloned() {
                                                state.direct_provenance.entry(val_id).or_default().extend(p);
                                            }
                                            if let Some(p) = state.carried_provenance.get(arg_v).cloned() {
                                                state.direct_provenance.entry(val_id).or_default().extend(p);
                                            }
                                        }

                                        let ret_ty = self.func.values[val_id.0 as usize].ty;
                                        let resolved_ret_ty = ctx.types.resolve(ret_ty);
                                        if let SemanticType::Struct(s_sym, ..) = ctx.types.get(resolved_ret_ty) {
                                            if let Some(type_contract) = ctx.tables.type_lifetime_contracts.get(s_sym) {
                                                for constraint in &type_contract.outlives_constraints {
                                                    if let luna_semantic::CanonicalTypeLifetimeSubject::Field(fp) = &constraint.longer {
                                                        if let Some(&f_idx) = fp.0.first() {
                                                            let mut field_place = self.compute_place_desc(&Operand::Value(val_id), state);
                                                            field_place.projections.push(Projection::Field(f_idx as u32));
                                                            let prov_sources = self.compute_provenance_sources(all_args[idx], state);
                                                            state.field_provenance.entry(field_place).or_default().sources.extend(prov_sources.sources);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Instruction::Store { ptr, value } | Instruction::StoreAnchored { ptr, value } => {
                self.check_access(value, false, val_id, state);
                self.check_access(ptr, true, val_id, state);
                let destination = self.compute_place_desc(ptr, state);
                let modes = match value {
                    Operand::Value(value) => state.closure_captures.get(value).cloned(),
                    _ => None,
                };
                state.closure_capture_storage.retain(|place, _| {
                    place.root != destination.root
                        || !place.projections.starts_with(&destination.projections)
                });
                if let Some(modes) = modes {
                    state.closure_capture_storage.insert(destination, modes);
                }
                if let Operand::Value(value_id) = value {
                    if !self.unsafe_raw_loans_for_carrier(*value_id, state).is_empty()
                        && !self.is_function_local_storage(ptr, state)
                    {
                        self.emit_raw_escape_diagnostic(
                            *value_id,
                            "storing the reference through non-local storage would outlive the unsafe access domain",
                        );
                    }
                }
                
                // If ptr is a Value, it inherits provenance from value (for alias tracking in borrowck)
                // Actually, if we store a reference into a local, that local now holds the provenance.
                // In NLL, if we do `*ptr = value`, we are overwriting `ptr`. But wait, `Instruction::Store` in MVIR is usually `Store { ptr: %v1, value: %v2 }`.
                // If %v1 is an Alloca, %v1 now contains the provenance of %v2.
                // To keep it simple, if ptr is Operand::Value, we add provenance of value to ptr.
                if let Operand::Value(ptr_val) = ptr {
                    let mut resolved_ptr = *ptr_val;
                    let mut current_op = Operand::Value(*ptr_val);
                    while let Operand::Value(v) = current_op {
                        if let Some(alias) = state.aliases.get(&v) {
                            current_op = alias.clone();
                            if let Operand::Value(alias_v) = current_op {
                                resolved_ptr = alias_v;
                            }
                        } else {
                            break;
                        }
                    }
                    
                    let is_direct_alloca = *ptr_val == resolved_ptr
                        && (resolved_ptr.0 as usize) < self.func.values.len()
                        && matches!(self.func.values[resolved_ptr.0 as usize].inst, Instruction::Alloca);

                    let address_roots = self.raw_value_roots(state, value);
                    if !address_roots.is_empty() {
                        let dest = self.compute_place_desc(ptr, state);
                        if is_direct_alloca {
                            state.unsafe_raw_root_storage.insert(dest, address_roots);
                        } else {
                            state.unsafe_raw_root_storage.entry(dest).or_default().extend(address_roots);
                        }
                    }

                    let value_is_raw_pointer = if let Some(ctx) = self.ctx {
                        if let Operand::Value(value_id) = value {
                            matches!(ctx.types.get(self.func.value(*value_id).ty), SemanticType::Pointer(..))
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if value_is_raw_pointer {
                        let dest = self.compute_place_desc(ptr, state);
                        let dest_for_anchor_check = dest.clone();
                        let anchor_owner = self.declared_anchor_owner(*ptr_val, state);
                        let origins = if let Operand::Value(value_id) = value {
                            state.raw_pointer_provenance.get(value_id).cloned().unwrap_or_default()
                        } else {
                            HashSet::new()
                        };
                        let roots = self.raw_value_roots(state, value);
                        if is_direct_alloca || anchor_owner.is_some() {
                            state.raw_pointer_storage.insert(dest.clone(), origins);
                            state.unsafe_raw_root_storage.insert(dest.clone(), roots);
                        } else {
                            state.raw_pointer_storage.entry(dest.clone()).or_default().extend(origins);
                            state.unsafe_raw_root_storage.entry(dest.clone()).or_default().extend(roots);
                        }

                        let is_null = self.is_null_pointer_value(value);
                        let is_explicit_anchor_store = matches!(
                            self.func.values.get(val_id.0 as usize).map(|v| &v.inst),
                            Some(Instruction::StoreAnchored { .. })
                        );
                        if let Some(owner) = &anchor_owner {
                            // Existing raw pointer copies refer to the previous
                            // field value. Replacing that storage invalidates
                            // their owner/field proof; a subsequent load can
                            // obtain a fresh proof from the type contract.
                            let stored_pointer_value = match value { Operand::Value(value_id) => Some(*value_id), _ => None };
                            self.invalidate_raw_pointer_anchors_for_owner(owner, stored_pointer_value, state);
                        }
                        state.raw_pointer_requires_anchor_storage.remove(&dest);
                        if let Some(owner) = anchor_owner {
                            if is_null {
                                // Null is a valid sentinel for an empty owner,
                                // so retain the logical owner contract while
                                // its distinct raw-origin fact prevents promotion.
                                state.raw_pointer_anchor_storage.insert(
                                    dest.clone(),
                                    HashSet::from([RawPointerAnchor::AnchoredTo(owner.clone())]),
                                );
                                state.raw_pointer_requires_anchor_storage.insert(dest);
                            } else if is_explicit_anchor_store {
                                let anchors = HashSet::from([RawPointerAnchor::AnchoredTo(owner.clone())]);
                                state.raw_pointer_anchor_storage.insert(dest.clone(), anchors);
                                state.raw_pointer_requires_anchor_storage.insert(dest);
                            } else {
                                let rhs_anchors = if let Operand::Value(value_id) = value {
                                    state.raw_pointer_anchors.get(value_id).cloned()
                                        .unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]))
                                } else {
                                    HashSet::from([RawPointerAnchor::Unknown])
                                };
                                let matches_owner = rhs_anchors.len() == 1
                                    && rhs_anchors.contains(&RawPointerAnchor::AnchoredTo(owner.clone()));
                                if !matches_owner && self.emit_diagnostics {
                                    let mut diag = Diagnostic::error(
                                        "RawStorageAnchorMismatch: safe store does not prove the pointer is anchored to this owner",
                                    ).with_code(DiagnosticCode::RawStorageAnchorMismatch);
                                    diag.span = self.func.values[val_id.0 as usize].span.clone();
                                    self.diagnostics.push(diag);
                                }
                                if matches_owner {
                                    state.raw_pointer_anchor_storage.insert(dest.clone(), rhs_anchors);
                                    state.raw_pointer_requires_anchor_storage.insert(dest);
                                } else {
                                    state.raw_pointer_anchor_storage.insert(
                                        dest.clone(),
                                        HashSet::from([RawPointerAnchor::Unknown]),
                                    );
                                    state.raw_pointer_requires_anchor_storage.insert(dest);
                                }
                            }
                        } else {
                            let rhs_anchors = if let Operand::Value(value_id) = value {
                                state.raw_pointer_anchors.get(value_id).cloned()
                                    .unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]))
                            } else {
                                HashSet::from([RawPointerAnchor::Unknown])
                            };
                            state.raw_pointer_anchor_storage.insert(dest.clone(), rhs_anchors);
                            if let Operand::Value(value_id) = value {
                                if state.raw_pointer_requires_anchor.contains(value_id) {
                                    state.raw_pointer_requires_anchor_storage.insert(dest);
                                }
                            }
                        }
                        let established_anchor = state.raw_pointer_anchor_storage.get(&dest_for_anchor_check).is_some_and(|anchors| {
                            !anchors.is_empty()
                                && anchors.iter().all(|anchor| matches!(anchor, RawPointerAnchor::AnchoredTo(_)))
                        });
                        if established_anchor {
                            state.raw_pointer_anchor_invalidated_storage.remove(&dest_for_anchor_check);
                        }
                        state.raw_pointer_value_invalidated_storage.remove(&dest_for_anchor_check);
                    }

                    if let (Some(ctx), Operand::Value(value_id)) = (self.ctx, value) {
                        if !matches!(ctx.types.get(self.func.value(*value_id).ty), SemanticType::Pointer(..))
                            && type_has_raw_pointer(self.func.value(*value_id).ty, ctx, &mut HashSet::new())
                        {
                            let source = self.compute_place_desc(value, state);
                            let destination = self.compute_place_desc(ptr, state);
                            self.copy_raw_pointer_storage_facts(&source, &destination, state);
                        }
                    }

                    // Check if ptr is a FieldPtr (or alias to FieldPtr)
                    let mut field_ptr_info = None;
                    if (ptr_val.0 as usize) < self.func.values.len() {
                        if let Instruction::FieldPtr { base, field_idx, .. } = &self.func.values[ptr_val.0 as usize].inst {
                            field_ptr_info = Some((base.clone(), *field_idx));
                        }
                    }
                    if field_ptr_info.is_none() && (resolved_ptr.0 as usize) < self.func.values.len() {
                        if let Instruction::FieldPtr { base, field_idx, .. } = &self.func.values[resolved_ptr.0 as usize].inst {
                            field_ptr_info = Some((base.clone(), *field_idx));
                        }
                    }

                    if let Some((base_op, f_idx)) = field_ptr_info {
                        let resolved_base = match self.resolve_alias(&base_op, state) {
                            Operand::Value(bv) => *bv,
                            _ => match base_op {
                                Operand::Value(bv) => bv,
                                _ => ValueId(0),
                            },
                        };

                        let mut val_loans = HashSet::new();
                        if let Operand::Value(val_v) = value {
                            if let Some(prov) = state.direct_provenance.get(val_v) {
                                val_loans.extend(prov.iter().cloned());
                            }
                            if let Some(prov) = state.carried_provenance.get(val_v) {
                                val_loans.extend(prov.iter().cloned());
                            }
                            let mut curr = *val_v;
                            while let Some(alias) = state.aliases.get(&curr) {
                                if let Operand::Value(av) = alias {
                                    if let Some(prov) = state.direct_provenance.get(av) {
                                        val_loans.extend(prov.iter().cloned());
                                    }
                                    if let Some(prov) = state.carried_provenance.get(av) {
                                        val_loans.extend(prov.iter().cloned());
                                    }
                                    curr = *av;
                                } else {
                                    break;
                                }
                            }
                        }

                        // Update Place-based field provenance (ProvenanceSet of PlaceDesc)
                        let mut field_place_desc = self.compute_place_desc(&base_op, state);
                        field_place_desc.projections.push(Projection::Field(f_idx));
                        let prov_sources = self.compute_provenance_sources(value, state);
                        state.field_provenance.insert(field_place_desc, prov_sources);

                        state.carried_provenance.entry(resolved_base).or_default().extend(val_loans.clone());
                        if let Operand::Value(base_v) = base_op {
                            if base_v != resolved_base {
                                state.carried_provenance.entry(base_v).or_default().extend(val_loans.clone());
                            }
                        }
                    } else {
                        let dest_base_desc = self.compute_place_desc(ptr, state);
                        if is_direct_alloca {
                            // Strong update: overwrite previous provenance for this variable
                            state.direct_provenance.remove(&resolved_ptr);
                            state.carried_provenance.remove(&resolved_ptr);
                            state.field_provenance.retain(|k, _| k.root != dest_base_desc.root);

                            if let Operand::Value(val_v) = value {
                                if let Some(prov) = state.direct_provenance.get(val_v).cloned() {
                                    state.direct_provenance.insert(resolved_ptr, prov);
                                }
                                if let Some(prov) = state.carried_provenance.get(val_v).cloned() {
                                    state.carried_provenance.insert(resolved_ptr, prov);
                                }
                                let src_base_desc = self.compute_place_desc(value, state);
                                let mut to_copy = Vec::new();
                                for (place, prov_set) in &state.field_provenance {
                                    if place.root == src_base_desc.root && place.projections.starts_with(&src_base_desc.projections) {
                                        let mut new_place = dest_base_desc.clone();
                                        new_place.projections.extend_from_slice(&place.projections[src_base_desc.projections.len()..]);
                                        to_copy.push((new_place, prov_set.clone()));
                                    }
                                }
                                for (new_place, prov_set) in to_copy {
                                    state.field_provenance.insert(new_place, prov_set);
                                }
                            }
                        } else {
                            // Weak update: conservative accumulation for indirect / aggregate writes
                            if let Operand::Value(val_v) = value {
                                if let Some(prov) = state.direct_provenance.get(val_v).cloned() {
                                    state.direct_provenance.entry(resolved_ptr).or_default().extend(prov);
                                }
                                if let Some(prov) = state.carried_provenance.get(val_v).cloned() {
                                    state.carried_provenance.entry(resolved_ptr).or_default().extend(prov);
                                }
                                let src_base_desc = self.compute_place_desc(value, state);
                                let mut to_copy = Vec::new();
                                for (place, prov_set) in &state.field_provenance {
                                    if place.root == src_base_desc.root && place.projections.starts_with(&src_base_desc.projections) {
                                        let mut new_place = dest_base_desc.clone();
                                        new_place.projections.extend_from_slice(&place.projections[src_base_desc.projections.len()..]);
                                        to_copy.push((new_place, prov_set.clone()));
                                    }
                                }
                                for (new_place, prov_set) in to_copy {
                                    let entry = state.field_provenance.entry(new_place).or_default();
                                    entry.sources.extend(prov_set.sources);
                                }
                            }
                        }
                    }
                }
            }
            Instruction::Load { ptr } => {
                self.check_access(ptr, false, val_id, state);
                let source = self.compute_place_desc(ptr, state);
                if let Some(modes) = state.closure_capture_storage.get(&source).cloned() {
                    state.closure_captures.insert(val_id, modes);
                } else {
                    state.closure_captures.remove(&val_id);
                }
                if let Operand::Value(ptr_val) = ptr {
                    let place = self.compute_place_desc(&Operand::Value(*ptr_val), state);
                    if let Some(roots) = state.unsafe_raw_root_storage.get(&place).cloned() {
                        state.unsafe_raw_roots.entry(val_id).or_default().extend(roots);
                    }
                }
                // Load from a pointer inherits its provenance if loaded value can hold references
                let is_ref_like = if let Some(ctx) = self.ctx {
                    if (val_id.0 as usize) < self.func.values.len() {
                        matches!(
                            ctx.types.get(self.func.values[val_id.0 as usize].ty),
                            SemanticType::Reference(..)
                                | SemanticType::Pointer(..)
                                | SemanticType::Future(..)
                                | SemanticType::Struct(..)
                                | SemanticType::Enum(..)
                                | SemanticType::Tuple(..)
                                | SemanticType::Slice(..)
                                | SemanticType::Closure(..)
                        )
                    } else {
                        true
                    }
                } else {
                    true
                };
                let loads_raw_pointer = if let Some(ctx) = self.ctx {
                    matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                } else {
                    false
                };
                if loads_raw_pointer {
                    let place = if let Operand::Value(ptr_val) = ptr {
                        Some(self.compute_place_desc(&Operand::Value(*ptr_val), state))
                    } else {
                        None
                    };
                    let mut origins = HashSet::new();
                    if let Some(place) = &place {
                        let invalidated = state.raw_pointer_value_invalidated_storage.iter()
                            .any(|written| Self::places_may_overlap(written, place));
                        if invalidated {
                            origins.insert(RawPointerOrigin::Unknown);
                        } else if let Some(stored_origins) = state.raw_pointer_storage.get(place) {
                            // The stored pointer value's origin is distinct
                            // from the origin of the address used to load it.
                            origins.extend(stored_origins.iter().cloned());
                        } else {
                            // Built-in slices are safe borrowed fat values
                            // whose first field is defined to be the element
                            // data pointer. Extracting that field preserves
                            // the safe reference's actual origin; unlike an
                            // arbitrary user struct pointer field, this is a
                            // language-defined representation relation, not
                            // provenance inferred from the field-slot address.
                            let slice_data_origin = if let Operand::Value(field_ptr) = ptr {
                                match self.func.values.get(field_ptr.0 as usize).map(|value| &value.inst) {
                                    Some(Instruction::FieldPtr { base, field_idx: 0, .. }) => {
                                        let base_ty = if let Operand::Value(base_value) = base {
                                            self.ctx.map(|ctx| ctx.types.get(self.func.value(*base_value).ty))
                                        } else { None };
                                        let is_slice_data = base_ty.is_some_and(|ty| match ty {
                                            SemanticType::Slice(_) => true,
                                            SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                                                self.ctx.is_some_and(|ctx| matches!(ctx.types.get(*inner), SemanticType::Slice(_)))
                                            }
                                            _ => false,
                                        });
                                        if is_slice_data {
                                            if let Operand::Value(base_value) = base {
                                                let resolved = self.resolve_alias(base, state);
                                                let source_value = match resolved {
                                                    Operand::Value(value) => *value,
                                                    _ => *base_value,
                                                };
                                                let mut derived = HashSet::new();
                                                derived.extend(state.direct_provenance.get(&source_value).into_iter().flatten().map(RawPointerOrigin::from_loan));
                                                derived.extend(state.carried_provenance.get(&source_value).into_iter().flatten().map(RawPointerOrigin::from_loan));
                                                derived
                                            } else {
                                                HashSet::new()
                                            }
                                        } else {
                                            HashSet::new()
                                        }
                                    }
                                    _ => HashSet::new(),
                                }
                            } else {
                                HashSet::new()
                            };
                            if slice_data_origin.is_empty() {
                                // For all ordinary pointer fields, the
                                // address operand identifies the field slot,
                                // not the pointer value stored there. Without
                                // a stored-value fact, origin stays unknown.
                                origins.insert(RawPointerOrigin::Unknown);
                            } else {
                                origins.extend(slice_data_origin);
                            }
                        }
                    }
                    // The Load result is the SSA pointer value. Replace any
                    // stale facts left by an earlier dataflow iteration;
                    // invalidation must not union the pre-FFI value back in.
                    state.raw_pointer_provenance.insert(val_id, origins);
                    let stored_roots = place.as_ref()
                        .and_then(|place| state.unsafe_raw_root_storage.get(place)).cloned();
                    // A load's result is the pointer value. If no stored
                    // lineage survives (including after an opaque FFI write),
                    // give that value a fresh function-local root instead of
                    // reusing the address/identity of the slot that held it.
                    state.unsafe_raw_roots.insert(
                        val_id,
                        stored_roots.unwrap_or_else(|| HashSet::from([val_id])),
                    );
                    let anchors = if let Operand::Value(ptr_val) = ptr {
                        let contract_owner = self.declared_anchor_owner(*ptr_val, state);
                        let anchor_was_invalidated = place.as_ref().is_some_and(|place| {
                            state.raw_pointer_anchor_invalidated_storage.iter()
                                .any(|invalidated| Self::places_may_overlap(invalidated, place))
                        });
                        let stored_anchors = place.as_ref()
                            .and_then(|place| state.raw_pointer_anchor_storage.get(place)).cloned();
                        let requires_anchor = contract_owner.is_some() || place.as_ref()
                            .is_some_and(|place| state.raw_pointer_requires_anchor_storage.contains(place));
                        if requires_anchor {
                            state.raw_pointer_requires_anchor.insert(val_id);
                        }
                        let has_local_anchor_evidence = stored_anchors.as_ref().is_some_and(|anchors| {
                            anchors.iter().any(|anchor| matches!(anchor, RawPointerAnchor::AnchoredTo(_)))
                        });
                        if anchor_was_invalidated {
                            HashSet::from([RawPointerAnchor::Unknown])
                        } else if has_local_anchor_evidence {
                            stored_anchors.unwrap_or_default()
                        } else if let Some(owner) = contract_owner {
                            // Unknown/missing per-value facts do not erase the
                            // declared type invariant. Explicit local anchors
                            // (including a conflicting owner) still take
                            // priority; an uncontracted pointer field has no
                            // such reconstruction path.
                            HashSet::from([RawPointerAnchor::AnchoredTo(owner)])
                        } else {
                            stored_anchors.unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]))
                        }
                    } else {
                        HashSet::from([RawPointerAnchor::Unknown])
                    };
                    // This load is the semantic source for the SSA pointer
                    // value. Replace stale placeholder facts for that value:
                    // a prior Unknown must not be unioned with a contract-
                    // reconstructed anchor and thereby erase the unique
                    // owner proof.
                    state.raw_pointer_anchors.insert(val_id, anchors);
                }
                if is_ref_like {
                    if let Operand::Value(ptr_val) = ptr {
                        let resolved = self.resolve_alias(ptr, state);
                        let check_val = if let Operand::Value(rv) = resolved { *rv } else { *ptr_val };
                        state.aliases.insert(val_id, Operand::Value(check_val));
                        if !loads_raw_pointer {
                        if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.direct_provenance.get(ptr_val).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(ptr_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        }

                        // A scalar raw pointer load transfers the pointer value's
                        // own origin and anchor above. The slot's aggregate
                        // field facts describe different storage.
                        let dest_desc = self.compute_place_desc(&Operand::Value(val_id), state);
                        let src_desc = self.compute_place_desc(ptr, state);
                        if !loads_raw_pointer && dest_desc != src_desc {
                            let mut to_copy = Vec::new();
                            for (place, prov_set) in &state.field_provenance {
                                if place.root == src_desc.root && place.projections.starts_with(&src_desc.projections) {
                                    let mut new_place = dest_desc.clone();
                                    new_place.projections.extend_from_slice(&place.projections[src_desc.projections.len()..]);
                                    to_copy.push((new_place, prov_set.clone()));
                                }
                            }
                            for (new_place, prov_set) in to_copy {
                                state.field_provenance.entry(new_place).or_default().sources.extend(prov_set.sources);
                            }
                            if let Some(ctx) = self.ctx {
                                if type_has_raw_pointer(self.func.value(val_id).ty, ctx, &mut HashSet::new()) {
                                    // Aggregate loads transfer facts for embedded raw fields.
                                    // Copying that subtree through a scalar pointer can grow
                                    // Deref projections indefinitely across a CFG loop.
                                    self.copy_raw_pointer_storage_facts(&src_desc, &dest_desc, state);
                                }
                            }
                        }
                    }
                }
            }
            Instruction::CallDirect { args, callee, .. } => {
                for arg in args {
                    self.check_access(arg, false, val_id, state);
                }
                
                let is_future_ret = if let Some(ctx) = self.ctx {
                    matches!(ctx.types.get(self.func.values[val_id.0 as usize].ty), SemanticType::Future(..))
                } else {
                    false
                };

                let mut applied_summary = None;
                if let Some(map) = self.callee_summaries {
                    if let Some(sum) = map.get(callee) {
                        applied_summary = Some(sum.clone());
                    }
                }
                let is_extern_call = self.direct_call_is_extern(callee);
                let mut ffi_written_places = Vec::new();
                if callee.name == "register_callback" {
                }
                
                if let Some(sum) = applied_summary {
                    for (i, arg) in args.iter().enumerate() {
                        if i < sum.args.len() {
                            let arg_effect = &sum.args[i];
                            if is_extern_call && self.raw_pointer_mutability(arg).is_some() {
                                ffi_written_places.extend(self.check_ffi_raw_pointer_access(arg, val_id, state));
                                continue;
                            }
                            if let Operand::Value(arg_value) = arg {
                                if !self.unsafe_raw_loans_for_carrier(*arg_value, state).is_empty()
                                    && (sum.is_opaque || matches!(
                                        arg_effect.escape,
                                        crate::effect::EscapeKind::MayEscape | crate::effect::EscapeKind::Unknown
                                    ))
                                {
                                    self.emit_raw_escape_diagnostic(
                                        *arg_value,
                                        "the callee has no proven non-escaping parameter effect",
                                    );
                                }
                            }
                            
                            // Check specific access (write access if required)
                            if arg_effect.access == crate::effect::AccessKind::Write || arg_effect.access == crate::effect::AccessKind::ReadWrite {
                                self.check_access(arg, true, val_id, state);
                            }
                            
                            // Handle escapes
                            if !is_future_ret && arg_effect.escape == crate::effect::EscapeKind::MayEscape {
                                if let Operand::Value(arg_v) = arg {
                                    if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                        state.escaped_loans.extend(prov);
                                    }
                                    if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                                        state.escaped_loans.extend(prov);
                                    }
                                    if let Some(origins) = state.raw_pointer_provenance.get(arg_v) {
                                        state.escaped_loans.extend(origins.iter().filter_map(|origin| origin.into_escaped_loan(*arg_v)));
                                    }
                                }
                            }
                        }
                    }
                    
                    // Return provenance
                    match &sum.ret {
                        ReturnEffect::BorrowsFrom(indices) => {
                            for &idx in indices {
                                if idx < args.len() {
                                    if let Operand::Value(arg_v) = &args[idx] {
                                        if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                                        }
                                    }

                                    // A returned reference is a fresh
                                    // call-site reborrow of its actual source,
                                    // not merely another spelling of the
                                    // caller's incoming parameter loan. Keep
                                    // the inherited provenance above, but add
                                    // this local loan so it remains live and
                                    // blocks parent mutation until the result
                                    // dies.
                                    if let Some(ctx) = self.ctx {
                                        if let SemanticType::Reference(_, mutability, _) =
                                            ctx.types.get(self.func.value(val_id).ty)
                                        {
                                            // Reborrow the actual referent, not the
                                            // caller's local slot holding a reference.
                                            for place in self.call_reborrow_places(&args[idx], state) {
                                                state.direct_provenance.entry(val_id).or_default().insert(Loan {
                                                    id: val_id, place,
                                                    is_rw: *mutability == luna_semantic::ty::Mutability::Mutable,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ReturnEffect::BorrowsCarried(indices) => {
                            for &idx in indices {
                                if idx < args.len() {
                                    if let Operand::Value(arg_v) = &args[idx] {
                                        if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                                        }
                                        if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                                        }
                                    }

                                    // `BorrowsCarried` describes how the
                                    // function body found the returned loan;
                                    // it does not mean the call-site result is
                                    // just the original parameter value. A
                                    // direct reference result is still a new
                                    // reborrow whose liveness must freeze its
                                    // parent while the result is live.
                                    if let Some(ctx) = self.ctx {
                                        if let SemanticType::Reference(_, mutability, _) =
                                            ctx.types.get(self.func.value(val_id).ty)
                                        {
                                            for place in self.call_reborrow_places(&args[idx], state) {
                                                state.direct_provenance.entry(val_id).or_default().insert(Loan {
                                                    id: val_id, place,
                                                    is_rw: *mutability == luna_semantic::ty::Mutability::Mutable,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    if self.ctx.map(|ctx| matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))).unwrap_or(false) {
                        if let RawPointerReturnEffect::From(indices) = &sum.raw_pointer_ret {
                            for &idx in indices {
                                if let Some(Operand::Value(arg_val)) = args.get(idx) {
                                    if let Some(origins) = state.raw_pointer_provenance.get(arg_val).cloned() {
                                        state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                                    }
                                    let roots = self.raw_value_roots(state, &Operand::Value(*arg_val));
                                    state.unsafe_raw_roots.entry(val_id).or_default().extend(roots);
                                    let anchors = state.raw_pointer_anchors.get(arg_val).cloned()
                                        .unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]));
                                    state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                                    if state.raw_pointer_requires_anchor.contains(arg_val) {
                                        state.raw_pointer_requires_anchor.insert(val_id);
                                    }
                                }
                            }
                        } else if matches!(sum.raw_pointer_ret, RawPointerReturnEffect::Unknown) {
                            // An imprecise summary cannot certify a unique
                            // lineage. Retain every raw argument candidate so
                            // later local loans conservatively overlap; the
                            // promotion gate rejects a multi-root set.
                            for arg in args {
                                if let Operand::Value(arg_val) = arg {
                                    if self.ctx.is_some_and(|ctx| matches!(ctx.types.get(self.func.value(*arg_val).ty), SemanticType::Pointer(..))) {
                                        let roots = self.raw_value_roots(state, arg);
                                        state.unsafe_raw_roots.entry(val_id).or_default().extend(roots);
                                    }
                                }
                            }
                        }
                        match &sum.raw_pointer_anchor_ret {
                            RawPointerAnchorReturnEffect::From(sources) => {
                                for source in sources {
                                    match source {
                                        RawPointerAnchorSource::RawParam(index) => {
                                            if let Some(Operand::Value(arg_val)) = args.get(*index) {
                                                let anchors = state.raw_pointer_anchors.get(arg_val).cloned()
                                                    .unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]));
                                                state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                                                if state.raw_pointer_requires_anchor.contains(arg_val) {
                                                    state.raw_pointer_requires_anchor.insert(val_id);
                                                }
                                            } else {
                                                state.raw_pointer_anchors.entry(val_id).or_default().insert(RawPointerAnchor::Unknown);
                                            }
                                        }
                                        RawPointerAnchorSource::OwnerField { param, .. } => {
                                            if let Some(arg) = args.get(*param) {
                                                let owner = self.compute_place_desc(arg, state);
                                                state.raw_pointer_anchors.entry(val_id).or_default()
                                                    .insert(RawPointerAnchor::AnchoredTo(owner));
                                                state.raw_pointer_requires_anchor.insert(val_id);
                                            } else {
                                                state.raw_pointer_anchors.entry(val_id).or_default().insert(RawPointerAnchor::Unknown);
                                            }
                                        }
                                        RawPointerAnchorSource::Unknown => {
                                            state.raw_pointer_anchors.entry(val_id).or_default().insert(RawPointerAnchor::Unknown);
                                        }
                                    }
                                }
                            }
                            RawPointerAnchorReturnEffect::Unknown => {
                                state.raw_pointer_anchors.entry(val_id).or_default().insert(RawPointerAnchor::Unknown);
                            }
                            RawPointerAnchorReturnEffect::Independent => {}
                        }
                    }
                    if let Some(ctx) = self.ctx {
                        let result_ty = self.func.value(val_id).ty;
                        let result_place = self.compute_place_desc(&Operand::Value(val_id), state);
                        for (field_index, field_name) in direct_raw_pointer_fields(ctx, result_ty) {
                            let mut field_place = result_place.clone();
                            field_place.projections.push(Projection::Field(field_index));
                            let Some(field_effect) = sum.raw_pointer_field_ret.get(&field_name) else {
                                state.raw_pointer_storage.insert(field_place.clone(), HashSet::from([RawPointerOrigin::Unknown]));
                                state.raw_pointer_anchor_storage.insert(field_place, HashSet::from([RawPointerAnchor::Unknown]));
                                continue;
                            };
                            let mut origins = HashSet::new();
                            match &field_effect.origin {
                                RawPointerReturnEffect::From(indices) => for &index in indices {
                                    if let Some(Operand::Value(arg_value)) = args.get(index) {
                                        origins.extend(state.raw_pointer_provenance.get(arg_value).into_iter().flatten().cloned());
                                    }
                                },
                                RawPointerReturnEffect::Unknown => { origins.insert(RawPointerOrigin::Unknown); }
                                RawPointerReturnEffect::Independent => {}
                            }
                            if matches!(field_effect.origin, RawPointerReturnEffect::From(_)) && origins.is_empty() {
                                origins.insert(RawPointerOrigin::Unknown);
                            }
                            state.raw_pointer_storage.insert(field_place.clone(), origins);

                            // A declared type contract remains the authority for
                            // owner-relative fields returned from source or .llib.
                            // It is not inferred from the field slot address.
                            let contracted = if let SemanticType::Struct(symbol, ..) = ctx.types.get(ctx.types.resolve(result_ty)) {
                                ctx.tables.raw_storage_anchor_contracts.get(symbol)
                                    .is_some_and(|contract| contract.field_names.contains(&field_name))
                            } else { false };
                            if contracted { continue; }
                            let mut anchors = HashSet::new();
                            match &field_effect.anchor {
                                RawPointerAnchorReturnEffect::From(sources) => for source in sources {
                                    match source {
                                        RawPointerAnchorSource::RawParam(index) => {
                                            if let Some(Operand::Value(arg_value)) = args.get(*index) {
                                                anchors.extend(state.raw_pointer_anchors.get(arg_value).into_iter().flatten().cloned());
                                            } else { anchors.insert(RawPointerAnchor::Unknown); }
                                        }
                                        RawPointerAnchorSource::OwnerField { param, .. } => {
                                            if let Some(arg) = args.get(*param) {
                                                anchors.insert(RawPointerAnchor::AnchoredTo(self.compute_place_desc(arg, state)));
                                            } else { anchors.insert(RawPointerAnchor::Unknown); }
                                        }
                                        RawPointerAnchorSource::Unknown => { anchors.insert(RawPointerAnchor::Unknown); }
                                    }
                                },
                                RawPointerAnchorReturnEffect::Unknown => { anchors.insert(RawPointerAnchor::Unknown); }
                                RawPointerAnchorReturnEffect::Independent => {}
                            }
                            state.raw_pointer_anchor_storage.insert(field_place, anchors);
                        }
                    }
                } else {
                    // Opaque call conservative fallback using CE6 ABI rules
                    let mut callee_sym_id = None;
                    if let Some(sym_id) = callee.symbol_id {
                        callee_sym_id = Some(sym_id);
                    }

                    for (arg_pos, arg) in args.iter().enumerate() {
                        if is_extern_call && self.raw_pointer_mutability(arg).is_some() {
                            ffi_written_places.extend(self.check_ffi_raw_pointer_access(arg, val_id, state));
                            continue;
                        }
                        let safe_ref_param = self.direct_call_safe_ref_param(callee, arg_pos);
                        if let Operand::Value(arg_value) = arg {
                            if !self.unsafe_raw_loans_for_carrier(*arg_value, state).is_empty()
                                && !safe_ref_param.as_ref().is_some_and(|(_, non_escaping)| *non_escaping)
                            {
                                self.emit_raw_escape_diagnostic(
                                    *arg_value,
                                    "a direct callee has neither a body-derived non-escaping summary nor a safe-reference-only call contract",
                                );
                            }
                        }
                        let mut escape_kind = crate::effect::EscapeKind::Unknown;
                        let mut access_kind = crate::effect::AccessKind::ReadWrite;
                        let mut has_sync_noescape = false;

                        if let Some(sym_id) = callee_sym_id {
                            if let Some(ctx) = self.ctx {
                                if let Some(noescapes) = ctx.tables.ffi_sync_noescape.get(&sym_id) {
                                    if arg_pos < noescapes.len() {
                                        has_sync_noescape = noescapes[arg_pos];
                                    }
                                }
                            }
                        }

                        if let Operand::Value(val_id) = arg {
                            if let Some(ctx) = self.ctx {
                                let val_data = self.func.value(*val_id);
                                if let Some((mutability, non_escaping)) = &safe_ref_param {
                                    if *non_escaping {
                                        escape_kind = crate::effect::EscapeKind::NoEscape;
                                        access_kind = if *mutability == Mutability::Mutable {
                                            crate::effect::AccessKind::ReadWrite
                                        } else {
                                            crate::effect::AccessKind::Read
                                        };
                                    }
                                }
                                if escape_kind == crate::effect::EscapeKind::Unknown
                                    && let Instruction::Borrow { is_rw, .. } = val_data.inst
                                {
                                    if matches!(ctx.types.get(val_data.ty), SemanticType::Pointer(..)) {
                                        escape_kind = if has_sync_noescape {
                                            crate::effect::EscapeKind::NoEscape
                                        } else {
                                            crate::effect::EscapeKind::MayEscape
                                        };
                                    } else {
                                        escape_kind = crate::effect::EscapeKind::NoEscape;
                                    }
                                    access_kind = if is_rw {
                                        crate::effect::AccessKind::ReadWrite
                                    } else {
                                        crate::effect::AccessKind::Read
                                    };
                                } else {
                                    use luna_semantic::ty::SemanticType;
                                    match ctx.types.get(val_data.ty) {
                                        SemanticType::Primitive(_) | SemanticType::Void | SemanticType::Never => {
                                            escape_kind = crate::effect::EscapeKind::NoEscape;
                                            access_kind = crate::effect::AccessKind::None;
                                        }
                                        SemanticType::Struct(_, _, _) | SemanticType::Enum(_, _, _) | SemanticType::Tuple(_) | SemanticType::Array(_, _) | SemanticType::Slice(_) | SemanticType::Future(_) => {
                                            // Passed by value (or opaque copy), no memory effect on the caller's aliasing
                                            escape_kind = crate::effect::EscapeKind::NoEscape;
                                            access_kind = crate::effect::AccessKind::None;
                                        }
                                        SemanticType::Reference(_, is_mut, _) => {
                                            // Safe Luna reference contract
                                            escape_kind = crate::effect::EscapeKind::NoEscape;
                                            access_kind = if *is_mut == luna_semantic::ty::Mutability::Mutable { 
                                                crate::effect::AccessKind::ReadWrite 
                                            } else { 
                                                crate::effect::AccessKind::Read 
                                            };
                                        }
                                    SemanticType::Pointer(is_mut, _) => {
                                        // Conservative C pointer contract
                                        escape_kind = if has_sync_noescape {
                                            crate::effect::EscapeKind::NoEscape
                                        } else {
                                            crate::effect::EscapeKind::MayEscape
                                        };
                                        access_kind = if *is_mut == luna_semantic::ty::Mutability::Mutable { 
                                            crate::effect::AccessKind::ReadWrite 
                                        } else { 
                                            crate::effect::AccessKind::Read 
                                        };
                                    }
                                    _ => {}
                                }
                                }
                            }
                        } else {
                            escape_kind = crate::effect::EscapeKind::NoEscape;
                            access_kind = crate::effect::AccessKind::None;
                        }

                        if !is_future_ret && (escape_kind == crate::effect::EscapeKind::MayEscape || escape_kind == crate::effect::EscapeKind::Unknown) {
                            if let Operand::Value(arg_v) = arg {
                                if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                    state.escaped_loans.extend(prov);
                                } else {
                                }
                                if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                                    state.escaped_loans.extend(prov);
                                }
                                if let Some(origins) = state.raw_pointer_provenance.get(arg_v) {
                                    state.escaped_loans.extend(origins.iter().filter_map(|origin| origin.into_escaped_loan(*arg_v)));
                                }
                            }
                        }

                        if access_kind == crate::effect::AccessKind::Read || access_kind == crate::effect::AccessKind::ReadWrite {
                            self.check_access(arg, false, val_id, state);
                        }
                        if access_kind == crate::effect::AccessKind::Write || access_kind == crate::effect::AccessKind::ReadWrite {
                            self.check_access(arg, true, val_id, state);
                        }
                    }
                }

                for written_place in ffi_written_places {
                    self.invalidate_ffi_written_facts(&written_place, state);
                }

                // Apply CanonicalLifetimeContract provenance if declared on callee
                let mut callee_sym_id = callee.symbol_id;
                if callee_sym_id.is_none() {
                    if let Some(ctx) = self.ctx {
                        if let Some(sym_id) = ctx.symbol_table.lookup(&callee.name, luna_semantic::symbol::ScopeId(0)) {
                            callee_sym_id = Some(sym_id);
                        }
                    }
                }
                if let Some(sym_id) = callee_sym_id {
                    if let Some(ctx) = self.ctx {
                        if let Some(contract) = ctx.tables.fn_lifetime_contracts.get(&sym_id) {
                            if let Some(prov) = &contract.return_provenance {
                                for &param_idx in prov.indices() {
                                    let idx = param_idx as usize;
                                    if idx < args.len() {
                                        if let Operand::Value(arg_v) = &args[idx] {
                                            if let Some(p) = state.direct_provenance.get(arg_v).cloned() {
                                                state.direct_provenance.entry(val_id).or_default().extend(p);
                                            }
                                            if let Some(p) = state.carried_provenance.get(arg_v).cloned() {
                                                state.direct_provenance.entry(val_id).or_default().extend(p);
                                            }
                                        }

                                        // Propagate referent provenance sources to reference fields of returned struct
                                        let ret_ty = self.func.values[val_id.0 as usize].ty;
                                        let resolved_ret_ty = ctx.types.resolve(ret_ty);
                                        if let SemanticType::Struct(s_sym, ..) = ctx.types.get(resolved_ret_ty) {
                                            if let Some(type_contract) = ctx.tables.type_lifetime_contracts.get(s_sym) {
                                                for constraint in &type_contract.outlives_constraints {
                                                    if let luna_semantic::CanonicalTypeLifetimeSubject::Field(fp) = &constraint.longer {
                                                        if let Some(&f_idx) = fp.0.first() {
                                                            let mut field_place = self.compute_place_desc(&Operand::Value(val_id), state);
                                                            field_place.projections.push(Projection::Field(f_idx as u32));
                                                            let prov_sources = self.compute_provenance_sources(&args[idx], state);
                                                            state.field_provenance.entry(field_place).or_default().sources.extend(prov_sources.sources);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // FutureLoan propagation for CallDirect:
                if is_future_ret {
                    for arg in args {
                        if let Operand::Value(arg_v) = arg {
                            let resolved = self.resolve_alias(arg, state);
                            let resolved_v = if let Operand::Value(rv) = resolved { *rv } else { *arg_v };
                            if let Some(prov) = state.direct_provenance.get(&resolved_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                            if let Some(prov) = state.carried_provenance.get(&resolved_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                            if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                            if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                        }
                    }
                }
            }
            Instruction::CallIndirect { args, callee } | Instruction::CallClosure { args, closure: callee } => {
                self.check_access(callee, false, val_id, state);
                for arg in args {
                    self.check_access(arg, false, val_id, state);
                }
                
                let is_future_ret = if let Some(ctx) = self.ctx {
                    matches!(ctx.types.get(self.func.values[val_id.0 as usize].ty), SemanticType::Future(..))
                } else {
                    false
                };

                // Pre-compute callback effects to union into this call
                let mut cb_worst_access = crate::effect::AccessKind::None;
                let mut cb_worst_escape = crate::effect::EscapeKind::NoEscape;

                for arg in args {
                    if let Operand::Global(cb_gid) = arg {
                        if let Some(summaries) = self.callee_summaries {
                            if let Some(cb_sum) = summaries.get(cb_gid) {
                                for cb_arg in &cb_sum.args {
                                    cb_worst_access = cb_worst_access.merge(&cb_arg.access);
                                    cb_worst_escape = cb_worst_escape.merge(&cb_arg.escape);
                                }
                            } else {
                                cb_worst_access = crate::effect::AccessKind::Unknown;
                                cb_worst_escape = crate::effect::EscapeKind::Unknown;
                            }
                        }
                    } else if let Operand::Value(arg_v) = arg {
                        if let Some(ctx) = self.ctx {
                            let val_data = self.func.value(*arg_v);
                            if let luna_semantic::ty::SemanticType::Function { .. } = ctx.types.get(val_data.ty) {
                                cb_worst_access = crate::effect::AccessKind::Unknown;
                                cb_worst_escape = crate::effect::EscapeKind::Unknown;
                            }
                        }
                    }
                }

                for arg in args.iter() {
                    let mut escape_kind = crate::effect::EscapeKind::Unknown;
                    let mut access_kind = crate::effect::AccessKind::ReadWrite;

                    if let Operand::Value(arg_v) = arg {
                        if let Some(ctx) = self.ctx {
                            let val_data = self.func.value(*arg_v);
                            use luna_semantic::ty::SemanticType;
                            match ctx.types.get(val_data.ty) {
                                SemanticType::Primitive(_) | SemanticType::Void | SemanticType::Never => {
                                    escape_kind = crate::effect::EscapeKind::NoEscape;
                                    access_kind = crate::effect::AccessKind::None;
                                }
                                SemanticType::Struct(_, _, _) | SemanticType::Enum(_, _, _) | SemanticType::Tuple(_) | SemanticType::Array(_, _) | SemanticType::Slice(_) | SemanticType::Future(_) => {
                                    escape_kind = crate::effect::EscapeKind::NoEscape;
                                    access_kind = crate::effect::AccessKind::None;
                                }
                                SemanticType::Reference(_, is_mut, _) => {
                                    escape_kind = crate::effect::EscapeKind::NoEscape;
                                    access_kind = if *is_mut == luna_semantic::ty::Mutability::Mutable { 
                                        crate::effect::AccessKind::ReadWrite 
                                    } else { 
                                        crate::effect::AccessKind::Read 
                                    };
                                }
                                SemanticType::Pointer(is_mut, _) => {
                                    escape_kind = crate::effect::EscapeKind::MayEscape;
                                    access_kind = if *is_mut == luna_semantic::ty::Mutability::Mutable { 
                                        crate::effect::AccessKind::ReadWrite 
                                    } else { 
                                        crate::effect::AccessKind::Read 
                                    };
                                }
                                _ => {}
                            }
                        }
                    } else {
                        escape_kind = crate::effect::EscapeKind::NoEscape;
                        access_kind = crate::effect::AccessKind::None;
                    }

                    // Union the callback's worst effect into this argument's access and escape
                    access_kind = access_kind.merge(&cb_worst_access);
                    if escape_kind != crate::effect::EscapeKind::Unknown {
                         escape_kind = escape_kind.merge(&cb_worst_escape);
                    }

                    if let Operand::Value(arg_value) = arg {
                        if !self.unsafe_raw_loans_for_carrier(*arg_value, state).is_empty()
                            && matches!(escape_kind, crate::effect::EscapeKind::MayEscape | crate::effect::EscapeKind::Unknown)
                        {
                            self.emit_raw_escape_diagnostic(
                                *arg_value,
                                "an indirect or opaque callee is not proven non-escaping",
                            );
                        }
                    }

                    if !is_future_ret && (escape_kind == crate::effect::EscapeKind::MayEscape || escape_kind == crate::effect::EscapeKind::Unknown) {
                        if let Operand::Value(arg_v) = arg {
                            if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                state.escaped_loans.extend(prov);
                            }
                            if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                                state.escaped_loans.extend(prov);
                            }
                            if let Some(origins) = state.raw_pointer_provenance.get(arg_v) {
                                state.escaped_loans.extend(origins.iter().filter_map(|origin| origin.into_escaped_loan(*arg_v)));
                            }
                        }
                    }

                    if access_kind == crate::effect::AccessKind::Read || access_kind == crate::effect::AccessKind::ReadWrite {
                        self.check_access(arg, false, val_id, state);
                    }
                    if access_kind == crate::effect::AccessKind::Write || access_kind == crate::effect::AccessKind::ReadWrite {
                        self.check_access(arg, true, val_id, state);
                    }
                }

                // FutureLoan propagation for CallIndirect:
                if is_future_ret {
                    for arg in args {
                        if let Operand::Value(arg_v) = arg {
                            let resolved = self.resolve_alias(arg, state);
                            let resolved_v = if let Operand::Value(rv) = resolved { *rv } else { *arg_v };
                            if let Some(prov) = state.direct_provenance.get(&resolved_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                            if let Some(prov) = state.carried_provenance.get(&resolved_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                            if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                            if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                                state.carried_provenance.entry(val_id).or_default().extend(prov);
                            }
                        }
                    }
                }
            }
            Instruction::Borrow { is_rw, base } => {
                let is_base_ptr_or_ref = if let Operand::Value(b_v) = base {
                    let is_storage_address = self.func.values.get(b_v.0 as usize).is_some_and(|value| {
                        matches!(value.inst, Instruction::Alloca | Instruction::FieldPtr { .. })
                    });
                    self.is_raw_derived_index_address(*b_v) || (!is_storage_address && self.ctx.is_some_and(|ctx| {
                        matches!(
                            ctx.types.get(self.func.value(*b_v).ty),
                            SemanticType::Pointer(..) | SemanticType::Reference(..)
                        )
                    }))
                } else {
                    false
                };

                if !is_base_ptr_or_ref {
                    // Issue a loan on the variable/storage being borrowed
                    self.issue_loan(base, *is_rw, val_id, state);
                }

                let result_is_raw_pointer = self.ctx.is_some_and(|ctx| {
                    matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                });
                if result_is_raw_pointer {
                    let mut origins = HashSet::new();
                    if !is_base_ptr_or_ref {
                        origins.extend(state.direct_provenance.get(&val_id).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        origins.extend(state.carried_provenance.get(&val_id).into_iter().flatten().map(RawPointerOrigin::from_loan));
                    } else if let Operand::Value(base_value) = base {
                        let base_ty = self.ctx.map(|ctx| ctx.types.get(self.func.value(*base_value).ty));
                        if base_ty.is_some_and(|ty| matches!(ty, SemanticType::Pointer(..))) {
                            origins.extend(state.raw_pointer_provenance.get(base_value).into_iter().flatten().cloned());
                            let roots = self.raw_value_roots(state, base);
                            state.unsafe_raw_roots.entry(val_id).or_default().extend(roots);
                            if let Some(anchors) = state.raw_pointer_anchors.get(base_value).cloned() {
                                state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                            }
                        } else {
                            origins.extend(state.direct_provenance.get(base_value).into_iter().flatten().map(RawPointerOrigin::from_loan));
                            origins.extend(state.carried_provenance.get(base_value).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        }
                    }
                    if origins.is_empty() {
                        origins.insert(RawPointerOrigin::Unknown);
                    }
                    state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                }

                if let (true, Operand::Value(base_val)) = (is_base_ptr_or_ref, base) {
                    let base_is_raw_pointer = self.is_raw_derived_index_address(*base_val) || self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(*base_val).ty), SemanticType::Pointer(..))
                    }).unwrap_or(false);
                    let result_is_safe_reference = self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Reference(..))
                    }).unwrap_or(false);
                    if base_is_raw_pointer && result_is_safe_reference {
                        let origins = state.raw_pointer_provenance.get(base_val).cloned().unwrap_or_default();
                        let origins_are_known_and_compatible = !origins.is_empty()
                            && origins.iter().all(|origin| origin.as_loan(val_id, *is_rw).is_some());
                        if state.raw_pointer_requires_anchor.contains(base_val) {
                            let anchor = state.raw_pointer_anchors.get(base_val).and_then(|anchors| {
                                if anchors.len() == 1 {
                                    anchors.iter().find_map(|anchor| match anchor {
                                        RawPointerAnchor::AnchoredTo(owner) => Some(owner.clone()),
                                        RawPointerAnchor::Unknown => None,
                                    })
                                } else { None }
                            });
                            let raw_pointer_can_satisfy_mutability = self.ctx.map(|ctx| {
                                matches!(
                                    ctx.types.get(self.func.value(*base_val).ty),
                                    SemanticType::Pointer(Mutability::Mutable, _) if *is_rw
                                ) || (!*is_rw && matches!(
                                    ctx.types.get(self.func.value(*base_val).ty),
                                    SemanticType::Pointer(..)
                                ))
                            }).unwrap_or(false)
                                && (!*is_rw || anchor.as_ref().map_or(false, |owner| {
                                    self.anchor_owner_allows_mutable_borrow(owner, state)
                                }));
                            let may_be_null = origins.contains(&RawPointerOrigin::Null);
                            if let Some(owner) = anchor.filter(|_| raw_pointer_can_satisfy_mutability && !may_be_null) {
                                // The declared anchor invariant (established at
                                // an explicit unsafe boundary or trusted as a
                                // public type contract) is the compatibility
                                // evidence for this conversion. Raw address
                                // origin remains a separate fact and may be
                                // Unknown; it is never rewritten into the
                                // logical owner or used as the owner loan.
                                state.direct_provenance.entry(val_id).or_default().insert(Loan {
                                    id: val_id,
                                    place: owner.root,
                                    is_rw: *is_rw,
                                });
                            } else if self.emit_diagnostics {
                                let mut diag = Diagnostic::error(
                                    "RawStorageAnchorViolation: raw-to-safe reference conversion requires one valid declared anchor and compatible mutability",
                                ).with_code(DiagnosticCode::RawStorageAnchorViolation);
                                diag.span = self.func.values[val_id.0 as usize].span.clone();
                                if !self.diagnostics.iter().any(|existing| existing.message == diag.message && existing.span == diag.span) {
                                    self.diagnostics.push(diag);
                                }
                            }
                        } else if origins_are_known_and_compatible {
                            for origin in origins {
                                if let Some(loan) = origin.as_loan(val_id, *is_rw) {
                                    state.direct_provenance.entry(val_id).or_default().insert(loan);
                                }
                            }
                        } else if !origins.is_empty()
                            && origins.iter().all(|origin| matches!(origin, RawPointerOrigin::Unknown))
                            && !state.raw_pointer_requires_anchor.contains(base_val)
                        {
                            // Unknown raw origin may establish a temporary
                            // safe loan at this unsafe raw-to-safe boundary.
                            // It remains an analysis-local root and therefore
                            // cannot satisfy `life_from` or escape the function.
                            let roots = state.unsafe_raw_roots.get(base_val).cloned()
                                .unwrap_or_else(|| HashSet::from([*base_val]));
                            if roots.len() == 1 {
                                let root = *roots.iter().next().unwrap();
                                let loan = Loan {
                                    id: val_id,
                                    place: self.unsafe_raw_root_place(root),
                                    is_rw: *is_rw,
                                };
                                self.issue_unsafe_root_loan(root, *is_rw, val_id, state);
                                state.unsafe_raw_root_loans.entry(val_id).or_default().insert(root);
                                state.direct_provenance.entry(val_id).or_default().insert(loan);
                            } else if self.emit_diagnostics {
                                self.emit_raw_escape_diagnostic(val_id, "ambiguous unknown raw-pointer lineage cannot form a safe reference");
                            }
                        } else if !state.raw_pointer_requires_anchor.contains(base_val) {
                            self.emit_raw_escape_diagnostic(
                                val_id,
                                "a merged or missing raw-pointer origin is not sufficient to create a safe reference",
                            );
                        }
                    }
                }
                
                // If base has provenance, the borrow carries it
                if let Operand::Value(base_v) = base {
                    let mut current = *base_v;
                    let mut resolved_base = *base_v;
                    while let Some(alias) = state.aliases.get(&current) {
                        if let Operand::Value(alias_v) = alias {
                            resolved_base = *alias_v;
                            current = *alias_v;
                        } else { break; }
                    }
                    let raw_pointer_base = self.is_raw_derived_index_address(*base_v) || self.ctx.is_some_and(|ctx| {
                        matches!(ctx.types.get(self.func.value(*base_v).ty), SemanticType::Pointer(..))
                    });
                    if !is_base_ptr_or_ref || !raw_pointer_base {
                        if let Some(prov) = state.direct_provenance.get(&resolved_base).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&resolved_base).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.direct_provenance.get(base_v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(base_v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(parents) = self.reference_projection_loans(base, state) {
                            state.carried_provenance.entry(val_id).or_default().extend(parents);
                        }
                    }
                    state.aliases.insert(val_id, Operand::Value(*base_v));
                }
            }
            Instruction::Variant { args, .. } => {
                for arg in args {
                    self.check_access(arg, false, val_id, state);
                    if let Operand::Value(arg_v) = arg {
                        let resolved = self.resolve_alias(arg, state);
                        let check_val = if let Operand::Value(rv) = resolved { *rv } else { *arg_v };
                        if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.direct_provenance.get(arg_v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(arg_v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                    }
                }
            }
            Instruction::Extract { value, field_idx, .. } => {
                self.check_access(value, false, val_id, state);
                if let Operand::Value(v) = value {
                    let resolved = self.resolve_alias(value, state);
                    let check_val = if let Operand::Value(rv) = resolved { *rv } else { *v };
                    if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.direct_provenance.get(v).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                    if let Some(prov) = state.carried_provenance.get(v).cloned() {
                        state.carried_provenance.entry(val_id).or_default().extend(prov);
                    }
                    let mut field_place = self.compute_place_desc(value, state);
                    field_place.projections.push(Projection::Field(*field_idx));
                    let dest_desc = self.compute_place_desc(&Operand::Value(val_id), state);
                    if dest_desc != field_place {
                        let mut to_copy = Vec::new();
                        for (place, prov_set) in &state.field_provenance {
                            if place.root == field_place.root && place.projections.starts_with(&field_place.projections) {
                                let mut new_place = dest_desc.clone();
                                new_place.projections.extend_from_slice(&place.projections[field_place.projections.len()..]);
                                to_copy.push((new_place, prov_set.clone()));
                            }
                        }
                        for (new_place, prov_set) in to_copy {
                            state.field_provenance.entry(new_place).or_default().sources.extend(prov_set.sources);
                        }
                    }
                }
            }
            Instruction::Tag { .. } => {}
            Instruction::FieldPtr { base, field_idx: _, .. } => {
                if let Operand::Value(b) = base {
                    let resolved = self.resolve_alias(base, state);
                    let check_val = if let Operand::Value(rv) = resolved { *rv } else { *b };
                    state.aliases.insert(val_id, Operand::Value(check_val));
                    let is_raw_pointer = self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                    }).unwrap_or(false);
                    if is_raw_pointer {
                        let mut origins = HashSet::new();
                        origins.extend(state.direct_provenance.get(&check_val).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        origins.extend(state.direct_provenance.get(b).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        origins.extend(state.carried_provenance.get(b).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                    } else {
                        if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.direct_provenance.get(b).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(b).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                    }
                }
            }
            Instruction::BoundsCheck { index, len } => {
                self.check_access(index, false, val_id, state);
                self.check_access(len, false, val_id, state);
            }
            Instruction::PtrOffset { ptr, offset } => {
                self.check_access(offset, false, val_id, state);
                self.check_access(ptr, false, val_id, state);
                if let Operand::Value(p) = ptr {
                    let resolved = self.resolve_alias(ptr, state);
                    let check_val = if let Operand::Value(rv) = resolved { *rv } else { *p };
                    state.aliases.insert(val_id, Operand::Value(check_val));
                    let is_raw_pointer = self.is_raw_derived_index_address(val_id) || self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                    }).unwrap_or(false);
                    if is_raw_pointer {
                        let mut origins = state.raw_pointer_provenance.get(&check_val).cloned().unwrap_or_default();
                        origins.extend(state.raw_pointer_provenance.get(p).cloned().unwrap_or_default());
                        state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                        let mut anchors = state.raw_pointer_anchors.get(&check_val).cloned().unwrap_or_default();
                        anchors.extend(state.raw_pointer_anchors.get(p).cloned().unwrap_or_default());
                        if anchors.is_empty() { anchors.insert(RawPointerAnchor::Unknown); }
                        state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                        if state.raw_pointer_requires_anchor.contains(&check_val) || state.raw_pointer_requires_anchor.contains(p) {
                            state.raw_pointer_requires_anchor.insert(val_id);
                        }
                    } else {
                        if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                    }
                }
            }
            Instruction::Add { left, right, .. }
            | Instruction::Sub { left, right, .. } => {
                self.check_access(left, false, val_id, state);
                self.check_access(right, false, val_id, state);
                let mut address_roots = self.raw_value_roots(state, left);
                address_roots.extend(self.raw_value_roots(state, right));
                if !address_roots.is_empty() {
                    state.unsafe_raw_roots.entry(val_id).or_default().extend(address_roots);
                }
                if matches!(inst, Instruction::Add { .. } | Instruction::Sub { .. }) {
                    let mut origins = HashSet::new();
                    let mut anchors = HashSet::new();
                    for operand in [left, right] {
                        if let Operand::Value(value) = operand {
                            origins.extend(state.raw_pointer_provenance.get(value).cloned().unwrap_or_default());
                            anchors.extend(state.raw_pointer_anchors.get(value).cloned().unwrap_or_default());
                        }
                    }
                    state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                    if anchors.is_empty() { anchors.insert(RawPointerAnchor::Unknown); }
                    state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                    if [left, right].iter().any(|operand| matches!(operand, Operand::Value(value) if state.raw_pointer_requires_anchor.contains(value))) {
                        state.raw_pointer_requires_anchor.insert(val_id);
                    }
                }
                for op in [left, right] {
                    if let Operand::Value(v) = op {
                        let resolved = self.resolve_alias(op, state);
                        let check_val = if let Operand::Value(rv) = resolved { *rv } else { *v };
                        if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.direct_provenance.get(v).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if check_val != *v
                            || !state.direct_provenance.get(&check_val).map_or(true, |s| s.is_empty())
                            || !state.carried_provenance.get(&check_val).map_or(true, |s| s.is_empty())
                        {
                            state.aliases.insert(val_id, Operand::Value(check_val));
                        }
                    }
                }
            }
            Instruction::Mul { left, right, .. }
            | Instruction::Div { left, right, .. }
            | Instruction::Rem { left, right, .. }
            | Instruction::Eq { left, right, .. }
            | Instruction::LessThan { left, right, .. }
            | Instruction::LessOrEq { left, right, .. }
            | Instruction::GreaterThan { left, right, .. }
            | Instruction::GreaterOrEq { left, right, .. } => {
                self.check_access(left, false, val_id, state);
                self.check_access(right, false, val_id, state);
            }
            Instruction::Drop { value, .. } => {
                self.check_access(value, true, val_id, state);
            }
            Instruction::Await { future } => {
                self.check_access(future, false, val_id, state);
                if self.emit_diagnostics {
                    let active = self.active_loans_after(val_id, state);
                    for loan in &active {
                        if !self.is_external_loan(loan, state) {
                            let resolved = self.resolve_alias(&loan.place, state);
                            let place_name = print_operand_name(resolved);
                            let mut diag = Diagnostic::error(format!(
                                "Borrow of local variable '{}' is held across an 'await' point [E_ASYNC_LOCAL_BORROW_ACROSS_AWAIT]. Self-referential futures are not permitted.",
                                place_name
                            )).with_code(DiagnosticCode::AsyncBorrowAcrossAwait);
                            diag.span = self.func.values[val_id.0 as usize].span.clone();
                            if !self.diagnostics.iter().any(|d| d.message == diag.message) {
                                self.diagnostics.push(diag);
                            }
                        }
                    }
                }
            }
            Instruction::Assign(op) => {
                if let Operand::Value(v) = op {
                    if let Some(modes) = state.closure_captures.get(v).cloned() {
                        state.closure_captures.insert(val_id, modes);
                    }
                    let address_roots = self.raw_value_roots(state, op);
                    if !address_roots.is_empty() {
                        state.unsafe_raw_roots.entry(val_id).or_default().extend(address_roots);
                    }
                    state.aliases.insert(val_id, Operand::Value(*v));
                    let is_raw_pointer = self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                    }).unwrap_or(false);
                    if is_raw_pointer {
                        if let Some(origins) = state.raw_pointer_provenance.get(v).cloned() {
                            state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                        }
                        let anchors = state.raw_pointer_anchors.get(v).cloned()
                            .unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]));
                        state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                        if state.raw_pointer_requires_anchor.contains(v) {
                            state.raw_pointer_requires_anchor.insert(val_id);
                        }
                    } else {
                        if let Some(prov) = state.direct_provenance.get(v).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                    }
                    let dest_desc = self.compute_place_desc(&Operand::Value(val_id), state);
                    let src_desc = self.compute_place_desc(op, state);
                    if dest_desc != src_desc {
                        let mut to_copy = Vec::new();
                        for (place, prov_set) in &state.field_provenance {
                            if place.root == src_desc.root && place.projections.starts_with(&src_desc.projections) {
                                let mut new_place = dest_desc.clone();
                                new_place.projections.extend_from_slice(&place.projections[src_desc.projections.len()..]);
                                to_copy.push((new_place, prov_set.clone()));
                            }
                        }
                        for (new_place, prov_set) in to_copy {
                            state.field_provenance.entry(new_place).or_default().sources.extend(prov_set.sources);
                        }
                        if !is_raw_pointer {
                            if let Some(ctx) = self.ctx {
                                if type_has_raw_pointer(self.func.value(val_id).ty, ctx, &mut HashSet::new()) {
                                    self.copy_raw_pointer_storage_facts(&src_desc, &dest_desc, state);
                                }
                            }
                        }
                    }
                }
            }
            Instruction::Cast { value, .. } => {
                let destination_is_raw = self.ctx.map(|ctx| {
                    matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                }).unwrap_or(false);
                if destination_is_raw && self.is_null_pointer_value(value) {
                    state.raw_pointer_provenance.entry(val_id).or_default().insert(RawPointerOrigin::Null);
                }
                let address_roots = self.raw_value_roots(state, value);
                if !address_roots.is_empty() {
                    state.unsafe_raw_roots.entry(val_id).or_default().extend(address_roots);
                }
                if let Operand::Value(v) = value {
                    let resolved = self.resolve_alias(value, state);
                    let check_val = if let Operand::Value(rv) = resolved { *rv } else { *v };
                    state.aliases.insert(val_id, Operand::Value(check_val));
                    let source_is_safe_reference = self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(*v).ty), SemanticType::Reference(..))
                    }).unwrap_or(false);
                    let source_is_raw = self.ctx.map(|ctx| {
                        matches!(ctx.types.get(self.func.value(*v).ty), SemanticType::Pointer(..))
                    }).unwrap_or(false);
                    let source_has_raw_origin = state.raw_pointer_provenance.contains_key(&check_val)
                        || state.raw_pointer_provenance.contains_key(v);
                    if destination_is_raw && source_is_safe_reference {
                        // A reference-to-raw cast denotes the place borrowed by
                        // the safe reference. Do not confuse that with any raw
                        // pointer value that happens to be stored in the place
                        // address used by lowering as an alias.
                        let mut origins = HashSet::new();
                        origins.extend(state.direct_provenance.get(v).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        origins.extend(state.carried_provenance.get(v).into_iter().flatten().map(RawPointerOrigin::from_loan));
                        state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                    } else if source_has_raw_origin {
                        let mut origins = state.raw_pointer_provenance.get(&check_val).cloned().unwrap_or_default();
                        origins.extend(state.raw_pointer_provenance.get(v).cloned().unwrap_or_default());
                        state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                        let anchors = state.raw_pointer_anchors.get(&check_val).cloned()
                            .or_else(|| state.raw_pointer_anchors.get(v).cloned())
                            .unwrap_or_else(|| HashSet::from([RawPointerAnchor::Unknown]));
                        state.raw_pointer_anchors.entry(val_id).or_default().extend(anchors);
                        if state.raw_pointer_requires_anchor.contains(&check_val) || state.raw_pointer_requires_anchor.contains(v) {
                            state.raw_pointer_requires_anchor.insert(val_id);
                        }
                    } else if source_is_raw {
                        // A raw pointer cast to an integer without tracked origin
                        // stays unknown; it must not inherit a safe loan.
                    } else {
                        let destination_is_raw = self.ctx.map(|ctx| {
                            matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))
                        }).unwrap_or(false);
                        if destination_is_raw {
                            let mut origins = HashSet::new();
                            origins.extend(state.direct_provenance.get(&check_val).into_iter().flatten().map(RawPointerOrigin::from_loan));
                            origins.extend(state.carried_provenance.get(&check_val).into_iter().flatten().map(RawPointerOrigin::from_loan));
                            origins.extend(state.direct_provenance.get(v).into_iter().flatten().map(RawPointerOrigin::from_loan));
                            origins.extend(state.carried_provenance.get(v).into_iter().flatten().map(RawPointerOrigin::from_loan));
                            state.raw_pointer_provenance.entry(val_id).or_default().extend(origins);
                        } else {
                        if let Some(prov) = state.direct_provenance.get(&check_val).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(&check_val).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.direct_provenance.get(v).cloned() {
                            state.direct_provenance.entry(val_id).or_default().extend(prov);
                        }
                        if let Some(prov) = state.carried_provenance.get(v).cloned() {
                            state.carried_provenance.entry(val_id).or_default().extend(prov);
                        }
                        }
                    }
                    let dest_desc = self.compute_place_desc(&Operand::Value(val_id), state);
                    let src_desc = self.compute_place_desc(value, state);
                    if dest_desc != src_desc {
                        let mut to_copy = Vec::new();
                        for (place, prov_set) in &state.field_provenance {
                            if place.root == src_desc.root && place.projections.starts_with(&src_desc.projections) {
                                let mut new_place = dest_desc.clone();
                                new_place.projections.extend_from_slice(&place.projections[src_desc.projections.len()..]);
                                to_copy.push((new_place, prov_set.clone()));
                            }
                        }
                        for (new_place, prov_set) in to_copy {
                            state.field_provenance.entry(new_place).or_default().sources.extend(prov_set.sources);
                        }
                    }
                }
            }
            Instruction::Alloca | _ => {}
        }

        // A raw-pointer value with no positive origin evidence is explicitly
        // unknown. Keeping that fact in the lattice is essential at joins.
        if self.ctx.is_some_and(|ctx| matches!(ctx.types.get(self.func.value(val_id).ty), SemanticType::Pointer(..))) {
            state.raw_pointer_provenance.entry(val_id).or_insert_with(|| {
                HashSet::from([RawPointerOrigin::Unknown])
            });
            if !state.unsafe_raw_roots.contains_key(&val_id) {
                let roots = match inst {
                    Instruction::Assign(value) | Instruction::Cast { value, .. } => self.raw_value_roots(state, value),
                    Instruction::Add { left, right } | Instruction::Sub { left, right } => {
                        let mut roots = self.raw_value_roots(state, left);
                        roots.extend(self.raw_value_roots(state, right));
                        roots
                    }
                    Instruction::CallIndirect { args, .. } | Instruction::CallClosure { args, .. } => {
                        let mut roots = HashSet::new();
                        for arg in args {
                            if let Operand::Value(arg_val) = arg {
                                if self.ctx.is_some_and(|ctx| matches!(ctx.types.get(self.func.value(*arg_val).ty), SemanticType::Pointer(..))) {
                                    roots.extend(self.raw_value_roots(state, arg));
                                }
                            }
                        }
                        roots
                    }
                    _ => HashSet::new(),
                };
                state.unsafe_raw_roots.insert(val_id, if roots.is_empty() { HashSet::from([val_id]) } else { roots });
            }
            state.raw_pointer_anchors.entry(val_id).or_insert_with(|| {
                HashSet::from([RawPointerAnchor::Unknown])
            });
        }
    }

    fn transfer_terminator(&mut self, _term: &Terminator, _state: &mut BorrowStateData) {
        // Lifetime and return escape checks are authoritatively owned by the Region Engine
        // (RegionBorrowBridge::diagnose_return).
    }

    fn transfer_edge(&mut self, from: &str, to: &str, state: &mut BorrowStateData) {
        let ending_locals = self.loop_info.ending_lifetime_locals(from, to);
        if !ending_locals.is_empty() {
            // Step 1: Lifetime validity checks across dynamic boundaries are authoritatively
            // owned by the Region Engine (RegionBorrowBridge::diagnose_boundary_crossing).


            // Step 2: Kill all loans of iteration-local places across this lifetime exit edge
            let aliases_snapshot = state.aliases.clone();
            let is_local_loan = |loan: &Loan, aliases: &HashMap<ValueId, Operand>| -> bool {
                let mut current = &loan.place;
                while let Operand::Value(v) = current {
                    if let Some(alias) = aliases.get(v) {
                        current = alias;
                    } else {
                        break;
                    }
                }
                if let Operand::Value(pv) = current {
                    ending_locals.contains(pv)
                } else {
                    false
                }
            };
            let is_local_raw_origin = |origin: &RawPointerOrigin| {
                origin.is_local(&aliases_snapshot, &ending_locals)
            };

            for loans in state.direct_provenance.values_mut() {
                loans.retain(|loan| !is_local_loan(loan, &aliases_snapshot));
            }
            for loans in state.carried_provenance.values_mut() {
                loans.retain(|loan| !is_local_loan(loan, &aliases_snapshot));
            }
            for origins in state.raw_pointer_provenance.values_mut() {
                origins.retain(|origin| !is_local_raw_origin(origin));
            }
            for roots in state.unsafe_raw_roots.values_mut() {
                roots.retain(|root| !ending_locals.contains(root));
            }
            for anchors in state.raw_pointer_anchors.values_mut() {
                anchors.retain(|anchor| match anchor {
                    RawPointerAnchor::AnchoredTo(place) => !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root)),
                    RawPointerAnchor::Unknown => true,
                });
                if anchors.is_empty() { anchors.insert(RawPointerAnchor::Unknown); }
            }
            state.escaped_loans.retain(|loan| !is_local_loan(loan, &aliases_snapshot));

            // Step 3: Kill all provenance held by ending local carriers
            for local_val in &ending_locals {
                state.direct_provenance.remove(local_val);
                state.carried_provenance.remove(local_val);
                state.unsafe_raw_root_loans.remove(local_val);
                state.raw_pointer_provenance.remove(local_val);
                state.unsafe_raw_roots.remove(local_val);
                state.raw_pointer_anchors.remove(local_val);
                state.raw_pointer_requires_anchor.remove(local_val);
                state.aliases.remove(local_val);
            }
            state.raw_pointer_storage.retain(|place, _| {
                !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root))
            });
            state.unsafe_raw_root_storage.retain(|place, _| {
                !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root))
            });
            state.raw_pointer_anchor_storage.retain(|place, _| {
                !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root))
            });
            state.raw_pointer_anchor_invalidated_storage.retain(|place| {
                !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root))
            });
            state.raw_pointer_value_invalidated_storage.retain(|place| {
                !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root))
            });
            state.raw_pointer_requires_anchor_storage.retain(|place| {
                !matches!(place.root, Operand::Value(root) if ending_locals.contains(&root))
            });
        }

        // Fixed-point pruning: prune carriers that are dead at the target block entry
        if let Some(entry_live) = self.live_at_entry.get(to) {
            state.direct_provenance.retain(|carrier, loans| {
                if loans.is_empty() { return false; }
                let is_local = (carrier.0 as usize) < self.func.values.len()
                    && matches!(self.func.values[carrier.0 as usize].origin, ValueOrigin::Local | ValueOrigin::Temporary);
                if is_local {
                    entry_live.contains(carrier)
                } else {
                    true
                }
            });
            state.carried_provenance.retain(|carrier, loans| {
                if loans.is_empty() { return false; }
                let is_local = (carrier.0 as usize) < self.func.values.len()
                    && matches!(self.func.values[carrier.0 as usize].origin, ValueOrigin::Local | ValueOrigin::Temporary);
                if is_local {
                    entry_live.contains(carrier)
                } else {
                    true
                }
            });
            state.raw_pointer_provenance.retain(|carrier, origins| {
                if origins.is_empty() { return false; }
                let is_local = (carrier.0 as usize) < self.func.values.len()
                    && matches!(self.func.values[carrier.0 as usize].origin, ValueOrigin::Local | ValueOrigin::Temporary);
                if is_local { entry_live.contains(carrier) } else { true }
            });
        }
    }

    fn merge(&mut self, dest: &mut BorrowStateData, src: &BorrowStateData) -> bool {
        if !dest.initialized {
            *dest = src.clone();
            dest.initialized = true;
            return true;
        }
        let mut changed = false;
        for (v, loans) in &src.direct_provenance {
            let dest_loans = dest.direct_provenance.entry(*v).or_default();
            for loan in loans {
                if dest_loans.insert(loan.clone()) {
                    changed = true;
                }
            }
        }
        for (v, loans) in &src.carried_provenance {
            let dest_loans = dest.carried_provenance.entry(*v).or_default();
            for loan in loans {
                if dest_loans.insert(loan.clone()) {
                    changed = true;
                }
            }
        }
        for (loan_value, roots) in &src.unsafe_raw_root_loans {
            let dest_roots = dest.unsafe_raw_root_loans.entry(*loan_value).or_default();
            let old_len = dest_roots.len();
            dest_roots.extend(roots.iter().copied());
            if dest_roots.len() != old_len { changed = true; }
        }
        for (v, origins) in &src.raw_pointer_provenance {
            let dest_origins = dest.raw_pointer_provenance.entry(*v).or_default();
            let old_len = dest_origins.len();
            dest_origins.extend(origins.iter().cloned());
            if dest_origins.len() != old_len {
                changed = true;
            }
        }
        for (v, roots) in &src.unsafe_raw_roots {
            let dest_roots = dest.unsafe_raw_roots.entry(*v).or_default();
            let old_len = dest_roots.len();
            dest_roots.extend(roots.iter().copied());
            if dest_roots.len() != old_len { changed = true; }
        }
        for (place, origins) in &src.raw_pointer_storage {
            let dest_origins = dest.raw_pointer_storage.entry(place.clone()).or_default();
            let old_len = dest_origins.len();
            dest_origins.extend(origins.iter().cloned());
            if dest_origins.len() != old_len {
                changed = true;
            }
        }
        for (place, roots) in &src.unsafe_raw_root_storage {
            let dest_roots = dest.unsafe_raw_root_storage.entry(place.clone()).or_default();
            let old_len = dest_roots.len();
            dest_roots.extend(roots.iter().copied());
            if dest_roots.len() != old_len { changed = true; }
        }
        // Anchor storage is a must-fact. A store on one incoming edge and no
        // fact on another must become AnchoredTo(A) | Unknown, never merely A.
        let anchor_storage_keys: HashSet<_> = dest.raw_pointer_anchor_storage.keys()
            .chain(src.raw_pointer_anchor_storage.keys()).cloned().collect();
        for place in anchor_storage_keys {
            let dest_entry = dest.raw_pointer_anchor_storage.entry(place.clone()).or_default();
            let old_len = dest_entry.len();
            if !src.raw_pointer_anchor_storage.contains_key(&place) {
                dest_entry.insert(RawPointerAnchor::Unknown);
            } else if !dest_entry.is_empty() {
                // Union with all alternatives from the incoming edge.
                dest_entry.extend(src.raw_pointer_anchor_storage[&place].iter().cloned());
            } else {
                dest_entry.insert(RawPointerAnchor::Unknown);
                dest_entry.extend(src.raw_pointer_anchor_storage[&place].iter().cloned());
            }
            if dest_entry.len() != old_len { changed = true; }
        }
        let anchor_value_keys: HashSet<_> = dest.raw_pointer_anchors.keys()
            .chain(src.raw_pointer_anchors.keys()).copied().collect();
        for value in anchor_value_keys {
            let dest_entry = dest.raw_pointer_anchors.entry(value).or_default();
            let old_len = dest_entry.len();
            if !src.raw_pointer_anchors.contains_key(&value) {
                dest_entry.insert(RawPointerAnchor::Unknown);
            } else if !dest_entry.is_empty() {
                dest_entry.extend(src.raw_pointer_anchors[&value].iter().cloned());
            } else {
                dest_entry.insert(RawPointerAnchor::Unknown);
                dest_entry.extend(src.raw_pointer_anchors[&value].iter().cloned());
            }
            if dest_entry.len() != old_len { changed = true; }
        }
        for value in &src.raw_pointer_requires_anchor {
            if dest.raw_pointer_requires_anchor.insert(*value) { changed = true; }
        }
        for place in &src.raw_pointer_requires_anchor_storage {
            if dest.raw_pointer_requires_anchor_storage.insert(place.clone()) { changed = true; }
        }
        for place in &src.raw_pointer_anchor_invalidated_storage {
            if dest.raw_pointer_anchor_invalidated_storage.insert(place.clone()) { changed = true; }
        }
        for place in &src.raw_pointer_value_invalidated_storage {
            if dest.raw_pointer_value_invalidated_storage.insert(place.clone()) { changed = true; }
        }
        // A kill marker dominates prior stored-value candidates at CFG joins.
        for (place, origins) in &mut dest.raw_pointer_storage {
            if dest.raw_pointer_value_invalidated_storage.iter()
                .any(|written| Self::places_may_overlap(written, place))
            {
                if origins.len() != 1 || !origins.contains(&RawPointerOrigin::Unknown) {
                    origins.clear();
                    origins.insert(RawPointerOrigin::Unknown);
                    changed = true;
                }
            }
        }
        for (k, prov_set) in &src.field_provenance {
            let dest_entry = dest.field_provenance.entry(k.clone()).or_default();
            for src_place in &prov_set.sources {
                if dest_entry.sources.insert(src_place.clone()) {
                    changed = true;
                }
            }
        }
        for (v, alias) in &src.aliases {
            if dest.aliases.insert(*v, alias.clone()) != Some(alias.clone()) {
                changed = true;
            }
        }
        for loan in &src.escaped_loans {
            if dest.escaped_loans.insert(loan.clone()) {
                changed = true;
            }
        }
        for (closure, modes) in &src.closure_captures {
            let entry = dest.closure_captures.entry(*closure).or_default();
            for mode in modes {
                if !entry.contains(mode) {
                    entry.push(*mode);
                    changed = true;
                }
            }
        }
        for (place, modes) in &src.closure_capture_storage {
            let entry = dest.closure_capture_storage.entry(place.clone()).or_default();
            for mode in modes {
                if !entry.contains(mode) {
                    entry.push(*mode);
                    changed = true;
                }
            }
        }
        changed
    }
}

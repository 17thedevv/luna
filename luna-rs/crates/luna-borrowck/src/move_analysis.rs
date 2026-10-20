use crate::dataflow::DataflowAnalysis;
use crate::place::Place;
use luna_common::{Diagnostic, DiagnosticCode};
use luna_mvir::{Function, Instruction, Operand, Terminator};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveState {
    Uninitialized,
    Live,
    Moved,
    Dropped,
    ConditionallyMoved,
    PartialMoved,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MoveStateData {
    pub is_unvisited: bool,
    pub places: HashMap<Place, MoveState>,
    /// DIAG-3/9: the instruction that moved or dropped each place, used as the
    /// structural origin label of use-after-move / use-after-drop diagnostics.
    pub sites: HashMap<Place, luna_common::Span>,
}

impl Default for MoveStateData {
    fn default() -> Self {
        Self { is_unvisited: true, places: HashMap::new(), sites: HashMap::new() }
    }
}

impl MoveStateData {
    pub fn new() -> Self {
        Self::default()
    }

    fn forget_origin_subtree(&mut self, place: &Place) {
        self.sites.retain(|candidate, _| candidate != place && !candidate.is_descendant_of(place));
    }

    fn origin_for(&self, place: &Place) -> Option<luna_common::Span> {
        self.sites.iter()
            .filter(|(candidate, _)| *candidate == place || place.is_descendant_of(candidate))
            .max_by_key(|(candidate, _)| candidate.projections.len())
            .map(|(_, site)| *site)
    }
}

pub struct MoveAnalyzer<'a> {
    pub diagnostics: Vec<Diagnostic>,
    pub emit_diagnostics: bool,
    pub dead_drops: std::collections::HashSet<luna_mvir::ValueId>,
    pub(crate) guarded_drops: HashSet<luna_mvir::ValueId>,
    pub func: &'a Function,
    pub semantic_ctx: Option<&'a luna_semantic::SemanticContext>,
    pub summaries: Option<&'a HashMap<luna_mvir::GlobalId, crate::effect::CallEffectSummary>>,
    pub values_to_places: HashMap<luna_mvir::ValueId, Place>,
    pub load_origins: HashMap<luna_mvir::ValueId, luna_mvir::ValueId>,
    /// Consumed storage operands at each instruction. Loan analysis checks
    /// invalidation against its own provenance facts; move analysis owns the
    /// decision whether a transfer actually consumes a value.
    pub(crate) consumed_places: HashMap<luna_mvir::ValueId, Vec<Operand>>,
    current_instruction: luna_mvir::ValueId,
}

impl<'a> MoveAnalyzer<'a> {
    pub fn new(func: &'a Function, semantic_ctx: Option<&'a luna_semantic::SemanticContext>, summaries: Option<&'a HashMap<luna_mvir::GlobalId, crate::effect::CallEffectSummary>>) -> Self {
        Self {
            diagnostics: Vec::new(),
            emit_diagnostics: false,
            dead_drops: std::collections::HashSet::new(),
            guarded_drops: HashSet::new(),
            func,
            semantic_ctx,
            summaries,
            values_to_places: HashMap::new(),
            load_origins: HashMap::new(),
            consumed_places: HashMap::new(),
            current_instruction: luna_mvir::ValueId(0),
        }
    }

    pub(crate) fn get_place_state(&self, place: &Place, state: &MoveStateData) -> MoveState {
        let mut current_ancestor = None;
        let mut max_len = -1;
        for (k, v) in &state.places {
            if k.is_ancestor_of(place) {
                if k.projections.len() as isize > max_len {
                    max_len = k.projections.len() as isize;
                    current_ancestor = Some(v.clone());
                }
            }
        }
        
        let effective_state = current_ancestor.unwrap_or(MoveState::Live);
        
        if effective_state == MoveState::Live {
            for (k, v) in &state.places {
                if k.is_descendant_of(place) {
                    if matches!(v, MoveState::Moved | MoveState::Dropped | MoveState::Uninitialized | MoveState::ConditionallyMoved) {
                        return MoveState::PartialMoved;
                    }
                }
            }
        }
        
        effective_state
    }

    fn check_place(&mut self, place: &Place, state: &MoveStateData, val_id: luna_mvir::ValueId) {
        if !self.emit_diagnostics { return; }
        let loc_state = self.get_place_state(place, state);
        let name = format!("%v{}", place.local.0);
        let mut formatted_name = name;
        for proj in &place.projections {
            match proj {
                crate::place::Projection::Field(idx) => {
                    formatted_name = format!("{}.{}", formatted_name, idx);
                }
                crate::place::Projection::Deref => {
                    formatted_name = format!("(*{})", formatted_name);
                }
                crate::place::Projection::Index => {
                    formatted_name = format!("{}[_]", formatted_name);
                }
            }
        }
        
        let span = self.func.values[val_id.0 as usize].span.clone();
        // DIAG-3/9: the diagnostic identity is the typed code plus the primary
        // span, and the origin of the move/drop is attached as a structural
        // related label instead of being restated in the message text.
        let (code, message, origin_label) = match loc_state {
            MoveState::Moved => (
                DiagnosticCode::UseAfterMove,
                format!("Use of moved value '{}'", formatted_name),
                Some("value moved here"),
            ),
            MoveState::ConditionallyMoved => (
                DiagnosticCode::UseAfterMove,
                format!("Use of conditionally moved value '{}'", formatted_name),
                Some("value moved here"),
            ),
            MoveState::Dropped => (
                DiagnosticCode::UseAfterDrop,
                format!("Use of dropped value '{}'", formatted_name),
                Some("value dropped here"),
            ),
            MoveState::Uninitialized => (
                DiagnosticCode::UseOfUninitializedValue,
                format!("Use of uninitialized value '{}'", formatted_name),
                None,
            ),
            MoveState::PartialMoved => (
                DiagnosticCode::UseAfterMove,
                format!("Use of partially moved value '{}'", formatted_name),
                Some("value partially moved here"),
            ),
            MoveState::Live => return,
        };
        if !self.has_diagnostic(code, &span) {
            let mut diag = Diagnostic::error(message).with_code(code);
            diag.span = span.clone();
            // The move may have been recorded for an ancestor place (a whole
            // local moved before a field is read); use the nearest one.
            let origin = state.origin_for(place);
            if let (Some(label), Some(origin)) = (origin_label, origin) {
                diag = diag.with_related(origin, label.to_string());
            }
            self.diagnostics.push(diag);
        }
    }

    /// DIAG-4: diagnostic identity is the typed code plus the primary span,
    /// never the rendered message text.
    fn has_diagnostic(&self, code: DiagnosticCode, span: &Option<luna_common::Span>) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.code == Some(code) && &d.span == span)
    }

    fn check_operand(&mut self, op: &Operand, state: &MoveStateData, val_id: luna_mvir::ValueId) {
        if !self.emit_diagnostics { return; }
        if let Operand::Value(val) = op {
            let place = match self.values_to_places.get(val).cloned() {
                Some(p) => p,
                None => return, // Temporary, assumed live
            };
            self.check_place(&place, state, val_id);
        }
    }

    /// True when invoking this closure must consume the callable value: its
    /// environment owns at least one move-captured, non-Copy (drop-obligated)
    /// value. Consuming-ness is decided by environment ownership, never by what
    /// the closure body happens to do.
    fn is_consuming_closure(&self, closure: &Operand) -> bool {
        let Some(ctx) = self.semantic_ctx else { return false; };
        let Operand::Value(val) = closure else { return false; };
        let ty = self.func.value(*val).ty;
        let luna_semantic::SemanticType::Closure(expr_id, _, _) = ctx.types.get(ty).clone() else { return false; };
        ctx.tables.closure_capture_bindings.get(&expr_id).is_some_and(|bindings| {
            bindings.iter().any(|binding| {
                binding.mode == luna_semantic::CaptureMode::Move && ctx.needs_drop(binding.ty)
            })
        })
    }

    fn mark_moved(&mut self, op: &Operand, state: &mut MoveStateData) {
        if let Operand::Value(val) = op {
            if let Some(ctx) = self.semantic_ctx {
                let inst_ty = self.func.value(*val).ty;
                let sem_ty = ctx.types.get(inst_ty);
                match sem_ty {
                    luna_semantic::SemanticType::Primitive(b) => {
                        use luna_semantic::ty::BuiltinType;
                        if matches!(b, BuiltinType::I8 | BuiltinType::I16 | BuiltinType::I32 | BuiltinType::I64 | BuiltinType::I128 | BuiltinType::Isize | BuiltinType::U8 | BuiltinType::U16 | BuiltinType::U32 | BuiltinType::U64 | BuiltinType::U128 | BuiltinType::Usize | BuiltinType::F32 | BuiltinType::F64 | BuiltinType::Bool | BuiltinType::Char) {
                            return; // Primitive types are trivially copyable
                        }
                    }
                    luna_semantic::SemanticType::Void => {
                        return;
                    }
                    luna_semantic::SemanticType::Pointer(_, _) | luna_semantic::SemanticType::Reference(..) | luna_semantic::SemanticType::Function { .. } => {
                        return; // Pointers, references, and function pointers are copyable handles
                    }
                    _ => {}
                }
            }

            if let Some(place) = self.values_to_places.get(val).cloned() {
                if self.emit_diagnostics {
                    let source = match &self.func.value(*val).inst {
                        Instruction::Load { ptr } => ptr.clone(),
                        _ => op.clone(),
                    };
                    let places = self.consumed_places.entry(self.current_instruction).or_default();
                    if !places.contains(&source) { places.push(source); }
                }
                if self.emit_diagnostics && !place.projections.is_empty() {
                    if let Some(ctx) = self.semantic_ctx {
                        for len in 0..place.projections.len() {
                            let mut ancestor = place.clone();
                            ancestor.projections.truncate(len);
                            let mut ancestor_ty = None;
                            for (v_id, p) in &self.values_to_places {
                                if p == &ancestor {
                                    ancestor_ty = Some(self.func.value(*v_id).ty);
                                    break;
                                }
                            }
                            if let Some(ty_id) = ancestor_ty {
                                let sem_ty = ctx.types.get(ty_id).clone();
                                let has_drop = match sem_ty {
                                    luna_semantic::SemanticType::Struct(sym_id, _, _) | luna_semantic::SemanticType::Enum(sym_id, _, _) => {
                                        ctx.tables.drop_impls.contains_key(&sym_id)
                                    },
                                    _ => false,
                                };
                                if has_drop {
                                    let msg = "Cannot move out of a subplace of a type that implements Drop".to_string();
                                    let span = self.func.values[val.0 as usize].span.clone();
                                    if !self.has_diagnostic(DiagnosticCode::PartialMoveUnderDrop, &span) {
                                        let mut diag = Diagnostic::error(msg)
                                            .with_code(DiagnosticCode::PartialMoveUnderDrop);
                                        diag.span = span;
                                        self.diagnostics.push(diag);
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
                
                state.places.retain(|k, _| !k.is_descendant_of(&place));
                state.forget_origin_subtree(&place);
                state.places.insert(place.clone(), MoveState::Moved);
                self.record_site(&place, state);
                self.collapse_fully_moved_ancestors(place, state);
            }
        }
    }

    fn collapse_fully_moved_ancestors(&self, mut place: Place, state: &mut MoveStateData) {
        while !place.projections.is_empty() {
            place.projections.pop();

            let Some(field_types) = self.droppable_fields_for_place(&place) else {
                break;
            };
            if field_types.is_empty() {
                break;
            }

            let all_droppable_fields_moved = field_types.iter().all(|(field_idx, _)| {
                let field_place = place
                    .clone()
                    .projected(crate::place::Projection::Field(*field_idx));
                matches!(
                    self.get_place_state(&field_place, state),
                    MoveState::Moved | MoveState::Dropped | MoveState::Uninitialized
                )
            });

            if !all_droppable_fields_moved {
                break;
            }

            let inherited = state
                .sites
                .iter()
                .filter(|(candidate, _)| candidate.is_descendant_of(&place))
                .map(|(_, site)| *site)
                .min_by_key(|site| (site.file_id.0, site.start, site.end, site.ctxt.0));
            state.places.retain(|candidate, _| !candidate.is_descendant_of(&place));
            // State compaction is not a new move: retain each field's valid
            // origin so a later field access can still identify its own cause.
            state.places.insert(place.clone(), MoveState::Moved);
            if let Some(site) = inherited {
                state.sites.insert(place.clone(), site);
            }
        }
    }

    /// DIAG-3/9: remember where a place was moved or dropped so the later use
    /// can carry a structural origin label.
    fn record_site(&self, place: &Place, state: &mut MoveStateData) {
        let site = self
            .func
            .values
            .get(self.current_instruction.0 as usize)
            .and_then(|value| value.span.clone());
        if let Some(site) = site {
            state.sites.insert(place.clone(), site);
        }
    }

    fn droppable_fields_for_place(
        &self,
        place: &Place,
    ) -> Option<Vec<(usize, luna_semantic::SemanticTypeId)>> {
        let ctx = self.semantic_ctx?;

        for (value_id, candidate) in &self.values_to_places {
            if candidate != place {
                continue;
            }

            let mut ty_id = self.func.value(*value_id).ty;
            loop {
                match ctx.types.get(ty_id) {
                    luna_semantic::SemanticType::Pointer(_, inner)
                    | luna_semantic::SemanticType::Reference(_, _, inner) => ty_id = *inner,
                    _ => break,
                }
            }

            let field_types = match ctx.types.get(ty_id) {
                luna_semantic::SemanticType::Struct(symbol, _, fields) => {
                    if ctx.tables.drop_impls.contains_key(symbol) {
                        return None;
                    }
                    fields.clone()
                }
                luna_semantic::SemanticType::Tuple(fields) => fields.clone(),
                _ => continue,
            };

            return Some(
                field_types
                    .into_iter()
                    .enumerate()
                    .filter(|(_, field_ty)| ctx.needs_drop(*field_ty))
                    .collect(),
            );
        }

        None
    }

    fn mark_dropped(&mut self, op: &Operand, state: &mut MoveStateData) {
        if let Operand::Value(val) = op {
            if let Some(place) = self.values_to_places.get(val).cloned() {
                let current_state = self.get_place_state(&place, state);
                if current_state != MoveState::Moved && current_state != MoveState::Uninitialized {
                    state.places.retain(|k, _| !k.is_descendant_of(&place));
                    state.forget_origin_subtree(&place);
                    state.places.insert(place.clone(), MoveState::Dropped);
                    self.record_site(&place, state);
                }
            }
        }
    }
    
    fn mark_live(&mut self, op: &Operand, state: &mut MoveStateData) {
        if let Operand::Value(val) = op {
            if let Some(place) = self.values_to_places.get(val).cloned() {
                state.places.retain(|k, _| !k.is_descendant_of(&place));
                state.forget_origin_subtree(&place);
                state.places.insert(place.clone(), MoveState::Live);

                if !place.projections.is_empty() {
                    let mut parent = place.clone();
                    parent.projections.pop();
                    if let Some(parent_state) = state.places.get(&parent) {
                        if *parent_state == MoveState::Uninitialized {
                            state.places.insert(parent, MoveState::Live);
                        }
                    }
                }
            }
        }
    }
}

impl<'a> DataflowAnalysis<MoveStateData> for MoveAnalyzer<'a> {
    fn transfer_instruction(
        &mut self,
        val_id: luna_mvir::ValueId,
        inst: &Instruction,
        state: &mut MoveStateData,
    ) {
        self.current_instruction = val_id;
        match inst {
            Instruction::Alloca | Instruction::HeapAlloc => {
                let p = Place::new(val_id);
                self.values_to_places.insert(val_id, p.clone());
                if (val_id.0 as usize) >= self.func.arg_count {
                    state.places.insert(p, MoveState::Uninitialized);
                } else {
                    state.places.insert(p, MoveState::Live);
                }
            }
            Instruction::Assign(op) => {
                self.check_operand(op, state, val_id);
                self.mark_moved(op, state);
            }
            Instruction::Store { ptr, value } | Instruction::StoreAnchored { ptr, value } => {
                self.check_operand(value, state, val_id);
                self.mark_moved(value, state);
                self.mark_live(ptr, state);
            }
            Instruction::Load { ptr, .. } => {
                self.check_operand(ptr, state, val_id);
                if let Operand::Value(ptr_val) = ptr {
                    let is_lvalue_addr = if let Some(val_data) = self.func.values.get(ptr_val.0 as usize) {
                        matches!(val_data.inst, Instruction::Alloca | Instruction::HeapAlloc | Instruction::FieldPtr { .. } | Instruction::PtrOffset { .. })
                    } else {
                        false
                    };
                    if is_lvalue_addr {
                        if let Some(ptr_place) = self.values_to_places.get(ptr_val).cloned() {
                            self.values_to_places.insert(val_id, ptr_place);
                        }
                    }
                }
            }
            Instruction::CallDirect { args, callee, .. } => {
                self.check_operand(&Operand::Global(callee.clone()), state, val_id);
                let mut effects = None;
                if let Some(sums) = self.summaries {
                    effects = sums.get(callee);
                }

                for arg in args {
                    self.check_operand(arg, state, val_id);
                    self.mark_moved(arg, state);
                }
            }
            Instruction::CallIndirect { args, callee, .. } => {
                self.check_operand(callee, state, val_id);
                for arg in args {
                    self.check_operand(arg, state, val_id);
                    self.mark_moved(arg, state);
                }
            }
            Instruction::Await { future } => {
                self.check_operand(future, state, val_id);
                self.mark_moved(future, state);
            }
            Instruction::CallClosure { closure, args, .. } => {
                self.check_operand(closure, state, val_id);
                for arg in args {
                    self.check_operand(arg, state, val_id);
                    self.mark_moved(arg, state);
                }
                // D1: a closure whose environment owns at least one move-captured,
                // non-Copy value is a one-shot callable. Invoking it consumes the
                // callable value itself, transferring environment ownership into
                // the invocation, so the closure place becomes moved.
                if self.is_consuming_closure(closure) {
                    self.mark_moved(closure, state);
                }
            }
            Instruction::MakeClosure { captures, .. } => {
                for cap in captures {
                    self.check_operand(&Operand::Value(cap.source), state, val_id);
                    if cap.mode == luna_semantic::CaptureMode::Move {
                        self.mark_moved(&Operand::Value(cap.source), state);
                    }
                }
            }
            Instruction::MakeTraitObject { data_ptr, .. } => {
                self.check_operand(data_ptr, state, val_id);
            }
            Instruction::MakeSlice { data_ptr, len } => {
                self.check_operand(data_ptr, state, val_id);
                self.check_operand(len, state, val_id);
            }
            Instruction::DropVirt { obj } => {
                self.check_operand(obj, state, val_id);
            }
            Instruction::CallIntrinsic { args, .. } => {
                for arg in args {
                    self.check_operand(arg, state, val_id);
                }
            }
            Instruction::CallVirt { obj, args, .. } => {
                self.check_operand(obj, state, val_id);
                for arg in args {
                    self.check_operand(arg, state, val_id);
                    self.mark_moved(arg, state);
                }
            }

            Instruction::Borrow { base, .. } => {
                self.check_operand(base, state, val_id);
            }
            Instruction::Add { left, right, .. }
            | Instruction::Sub { left, right, .. }
            | Instruction::Mul { left, right, .. }
            | Instruction::Div { left, right, .. }
            | Instruction::Rem { left, right, .. }
            | Instruction::Eq { left, right, .. }
            | Instruction::NotEq { left, right, .. }
            | Instruction::LessThan { left, right, .. }
            | Instruction::LessOrEq { left, right, .. }
            | Instruction::GreaterThan { left, right, .. }
            | Instruction::GreaterOrEq { left, right, .. }
            | Instruction::BitAnd { left, right, .. }
            | Instruction::BitOr { left, right, .. }
            | Instruction::BitXor { left, right, .. }
            | Instruction::Shl { left, right, .. }
            | Instruction::Shr { left, right, .. } => {
                self.check_operand(left, state, val_id);
                self.check_operand(right, state, val_id);
            }
            Instruction::HeapFree { value } => {
                self.mark_dropped(value, state);
            }
            Instruction::MarkInit { value } => {
                self.mark_live(value, state);
            }
            Instruction::SizeOf { .. } |
            Instruction::AlignOf { .. } |
            Instruction::Null { .. } |
            Instruction::Cast { .. } => {}
            Instruction::PtrOffset { ptr, offset } => {
                // A constant offset into an array base addresses a statically
                // known element subplace. Tracking it as a field projection
                // lets a moved element be marked moved so caller-side array
                // cleanup drops only the remaining elements. A dynamic offset
                // (or a non-array base such as a raw-slice pointer) has no
                // statically known subplace and produces no place.
                if let (Operand::Value(ptr_val), Operand::Number(text)) = (ptr, offset) {
                    if let Some(idx) = text.parse::<usize>().ok() {
                        if let Some(base_place) = self.values_to_places.get(ptr_val).cloned() {
                            let array_base = self.semantic_ctx.is_some_and(|ctx| {
                                let ty = ctx.types.resolve(self.func.value(*ptr_val).ty);
                                let target = match ctx.types.get(ty) {
                                    luna_semantic::SemanticType::Pointer(_, inner)
                                    | luna_semantic::SemanticType::Reference(_, _, inner) => *inner,
                                    _ => ty,
                                };
                                matches!(
                                    ctx.types.get(ctx.types.resolve(target)),
                                    luna_semantic::SemanticType::Array(..)
                                )
                            });
                            if array_base {
                                self.values_to_places.insert(
                                    val_id,
                                    base_place.projected(crate::place::Projection::Field(idx)),
                                );
                            }
                        }
                    }
                }
            }
            Instruction::FieldPtr { base, field_idx, .. } => {
                if let Operand::Value(base_val) = base {
                    if let Some(base_place) = self.values_to_places.get(base_val).cloned() {
                        self.values_to_places.insert(val_id, base_place.projected(crate::place::Projection::Field(*field_idx as usize)));
                    }
                }
            }
            Instruction::BoundsCheck { index, len } => {
                self.check_operand(index, state, val_id);
                self.check_operand(len, state, val_id);
            }
            Instruction::Variant { args, .. } => {
                for arg in args {
                    self.check_operand(arg, state, val_id);
                    self.mark_moved(arg, state);
                }
            }
            Instruction::Neg { value } | Instruction::Tag { value } => {
                self.check_operand(value, state, val_id);
            }
            Instruction::Extract { value, field_idx, .. } => {
                if let Operand::Value(base_val) = value {
                    if let Some(base_place) = self.values_to_places.get(base_val).cloned() {
                        let field_place = base_place.projected(crate::place::Projection::Field(*field_idx as usize));
                        self.values_to_places.insert(val_id, field_place.clone());
                        
                        // Check the field place instead of the base place to allow partial moves
                        if self.emit_diagnostics {
                            let loc_state = self.get_place_state(&field_place, state);
                            if matches!(loc_state, MoveState::Moved | MoveState::Dropped | MoveState::ConditionallyMoved | MoveState::Uninitialized) {
                                self.check_place(&field_place, state, val_id);
                            }
                        }
                        
                        // Check E_PARTIAL_MOVE_UNDER_DROP: if any ancestor of field_place
                        // has a user Drop impl, reject the extraction as a partial move.
                        // This is checked directly here rather than via mark_moved because
                        // Extract must NOT mark the place as moved — the downstream consumer
                        // (Store, Variant, etc.) does that.
                        if self.emit_diagnostics && !field_place.projections.is_empty() {
                            if let Some(ctx) = self.semantic_ctx {
                                for len in 0..field_place.projections.len() {
                                    let mut ancestor = field_place.clone();
                                    ancestor.projections.truncate(len);
                                    let mut ancestor_ty = None;
                                    for (v_id, p) in &self.values_to_places {
                                        if p == &ancestor {
                                            ancestor_ty = Some(self.func.value(*v_id).ty);
                                            break;
                                        }
                                    }
                                    if let Some(ty_id) = ancestor_ty {
                                        let sem_ty = ctx.types.get(ty_id).clone();
                                        let has_drop = match sem_ty {
                                            luna_semantic::SemanticType::Struct(sym_id, _, _) | luna_semantic::SemanticType::Enum(sym_id, _, _) => {
                                                ctx.tables.drop_impls.contains_key(&sym_id)
                                            },
                                            _ => false,
                                        };
                                        if has_drop {
                                            let msg = "Cannot move out of a subplace of a type that implements Drop".to_string();
                                            let span = self.func.values[val_id.0 as usize].span.clone();
                                            if !self.has_diagnostic(DiagnosticCode::PartialMoveUnderDrop, &span) {
                                                let mut diag = Diagnostic::error(msg)
                                                    .with_code(DiagnosticCode::PartialMoveUnderDrop);
                                                diag.span = span;
                                                self.diagnostics.push(diag);
                                            }
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Instruction::Drop { value, .. } => {
                let mut is_heap_ptr_drop = false;
                if let Operand::Value(val) = value {
                    if let Some(val_data) = self.func.values.get(val.0 as usize) {
                        is_heap_ptr_drop = matches!(
                            val_data.inst,
                            Instruction::Load { .. }
                                | Instruction::CallDirect { .. }
                                | Instruction::CallIndirect { .. }
                                | Instruction::Cast { .. }
                                | Instruction::Assign(..)
                        );
                    }
                }

                if !is_heap_ptr_drop {
                    if self.emit_diagnostics {
                        if let Operand::Value(val) = value {
                            if let Some(place) = self.values_to_places.get(val).cloned() {
                                let loc_state = self.get_place_state(&place, state);
                                if loc_state == MoveState::ConditionallyMoved && !self.guarded_drops.contains(&val_id) {
                                    let msg = format!("Cannot drop conditionally moved value");
                                    let span = self.func.values[val.0 as usize].span.clone();
                                    if !self.has_diagnostic(DiagnosticCode::UseAfterMove, &span) {
                                        let mut diag = Diagnostic::error(msg).with_code(DiagnosticCode::UseAfterMove);
                                        diag.span = span;
                                        if let Some(origin) = state.origin_for(&place) {
                                            diag = diag.with_related(origin, "value moved here".to_string());
                                        }
                                        self.diagnostics.push(diag);
                                    }
                                }
                                if loc_state == MoveState::Moved || loc_state == MoveState::Dropped || loc_state == MoveState::Uninitialized || loc_state == MoveState::PartialMoved {
                                    self.dead_drops.insert(val_id);
                                }
                            }
                        }
                    }
                    self.mark_dropped(value, state);
                }
            }
            Instruction::StaticAddress(_) | Instruction::Nop => {}
        }
    }

    fn transfer_terminator(&mut self, term: &Terminator, state: &mut MoveStateData) {
        match term {
            Terminator::Ret { value } => {
                if let Some(val) = value {
                    self.check_operand(val, state, luna_mvir::ValueId(0));
                    self.mark_moved(val, state);
                }
            }
            Terminator::CondBr { condition, .. } => {
                self.check_operand(condition, state, luna_mvir::ValueId(0));
            }
            _ => {}
        }
    }


    fn merge(&mut self, dest: &mut MoveStateData, src: &MoveStateData) -> bool {
        if dest.is_unvisited {
            *dest = src.clone();
            dest.is_unvisited = false;
            return true;
        }
        
        let mut changed = false;
        
        let mut site_keys = HashSet::new();
        for k in dest.sites.keys() { site_keys.insert(k.clone()); }
        for k in src.sites.keys() { site_keys.insert(k.clone()); }
        for k in site_keys {
            // Origin information is part of the dataflow state too. Different
            // incoming moves are all valid antecedents; retain the earliest
            // source site instead of erasing it at a conditional join. Merge
            // even when the ownership states agree so predecessor order cannot
            // choose the reported origin or stop its propagation prematurely.
            let old_site = dest.sites.get(&k).copied();
            let merged_site = [old_site, src.sites.get(&k).copied()]
                .into_iter()
                .flatten()
                .min_by_key(|site| (site.file_id.0, site.start, site.end, site.ctxt.0));
            if old_site != merged_site {
                if let Some(site) = merged_site {
                    dest.sites.insert(k.clone(), site);
                }
                changed = true;
            }
        }

        let mut all_keys = HashSet::new();
        for k in dest.places.keys() { all_keys.insert(k.clone()); }
        for k in src.places.keys() { all_keys.insert(k.clone()); }
        for k in all_keys {
            let dest_state = dest.places.get(&k).cloned();
            let src_state = src.places.get(&k).cloned();
            
            // If neither has it directly, they might inherit it from ancestors, but we only merge explicit entries
            if dest_state == src_state { continue; }
            
            // If one has an explicit state and the other doesn't, the other inherits from its ancestors.
            // But we need to be careful: if we merge, we should merge the *effective* states of the place in both CFG paths!
            let effective_dest = self.get_place_state(&k, dest);
            let effective_src = self.get_place_state(&k, src);
            
            let new_state = match (effective_dest.clone(), effective_src.clone()) {
                (MoveState::Uninitialized, MoveState::Uninitialized) => MoveState::Uninitialized,
                (MoveState::Dropped, MoveState::Dropped) => MoveState::Dropped,
                (MoveState::Moved, MoveState::Moved) => MoveState::Moved,
                (MoveState::ConditionallyMoved, _) | (_, MoveState::ConditionallyMoved) => MoveState::ConditionallyMoved,
                (MoveState::Moved, _) | (_, MoveState::Moved) => MoveState::ConditionallyMoved,
                (MoveState::Dropped, _) | (_, MoveState::Dropped) => MoveState::ConditionallyMoved,
                (MoveState::Uninitialized, _) | (_, MoveState::Uninitialized) => MoveState::ConditionallyMoved,
                (MoveState::PartialMoved, MoveState::PartialMoved) => MoveState::PartialMoved,
                (MoveState::PartialMoved, _) | (_, MoveState::PartialMoved) => MoveState::PartialMoved, // actually a partial move mixed with Live is basically a partial move, but maybe ConditionallyMoved? Let's use PartialMoved
                _ => MoveState::Live,
            };
            
            // Only update dest if it differs from the current explicit state?
            // Actually, if we just insert the merged state, we might be inserting redundant entries.
            // But that's safe.
            if dest_state.as_ref() != Some(&new_state) {
                dest.places.insert(k.clone(), new_state);
                changed = true;
            }
        }
        
        changed
    }

    fn init_entry_state(&mut self, func: &Function, state: &mut MoveStateData) {
        state.is_unvisited = false;
        let mut alloca_count = 0;
        for (idx, val_data) in func.values.iter().enumerate() {
            if matches!(val_data.inst, Instruction::Alloca) {
                let init_state = if alloca_count < func.arg_count {
                    MoveState::Live
                } else {
                    MoveState::Uninitialized
                };
                
                let p = Place::new(luna_mvir::ValueId(idx as u32));
                state.places.insert(p, init_state);
                    
                alloca_count += 1;
            }
        }
    }
}

#[cfg(test)]
mod origin_join_tests {
    use super::*;
    use luna_common::{FileId, Span, SyntaxContext};
    use luna_mvir::{GlobalId, ValueId};
    use luna_semantic::SemanticTypeId;

    #[test]
    fn origin_join_is_order_independent_and_reaches_a_fixed_point() {
        let function = Function {
            name: GlobalId { name: "origin_join".into(), symbol_id: None },
            is_extern: false, is_async: false, arg_count: 0, link_name: None,
            param_types: Vec::new(), ret_ty: SemanticTypeId(0),
            lifetime_info: Default::default(), blocks: Vec::new(), values: Vec::new(),
        };
        let mut analysis = MoveAnalyzer::new(&function, None, None);
        let place = Place::new(ValueId(0));
        let early = Span { file_id: FileId(1), start: 10, end: 12, ctxt: SyntaxContext(1) };
        let same_location = Span { ctxt: SyntaxContext(2), ..early };
        let later = Span { start: 20, end: 22, ..early };
        let incoming: Vec<_> = [(MoveState::Moved, later), (MoveState::Moved, same_location),
                                (MoveState::ConditionallyMoved, early)].into_iter().map(|(state, site)| {
            let mut data = MoveStateData::new();
            data.is_unvisited = false;
            data.places.insert(place.clone(), state);
            data.sites.insert(place.clone(), site);
            data
        }).collect();
        for order in [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
            let mut joined = MoveStateData::new();
            for index in order { analysis.merge(&mut joined, &incoming[index]); }
            assert_eq!(joined.places.get(&place), Some(&MoveState::ConditionallyMoved));
            assert_eq!(joined.sites.get(&place), Some(&early));
            for state in &incoming { assert!(!analysis.merge(&mut joined, state)); }
        }
        let mut equal_state = incoming[0].clone();
        assert!(analysis.merge(&mut equal_state, &incoming[1]), "an origin-only change must propagate");
        assert_eq!(equal_state.places.get(&place), Some(&MoveState::Moved));
        assert_eq!(equal_state.sites.get(&place), Some(&same_location));
        assert!(!analysis.merge(&mut equal_state, &incoming[1]));

        // Compacted ownership states may still carry field-specific origins.
        let child = place.clone().projected(crate::place::Projection::Field(0));
        let sibling = place.clone().projected(crate::place::Projection::Field(1));
        let mut compacted = incoming[0].clone();
        let mut fields = incoming[0].clone();
        fields.sites.insert(child.clone(), early);
        fields.sites.insert(sibling.clone(), later);
        assert!(analysis.merge(&mut compacted, &fields));
        assert_eq!(compacted.sites.get(&child), Some(&early));
        assert!(!compacted.places.contains_key(&child));
        assert!(!analysis.merge(&mut compacted, &fields));
        compacted.forget_origin_subtree(&child);
        assert!(!compacted.sites.contains_key(&child));
        assert_eq!(compacted.sites.get(&sibling), Some(&later));
        compacted.forget_origin_subtree(&place);
        assert!(compacted.sites.is_empty());
    }
}

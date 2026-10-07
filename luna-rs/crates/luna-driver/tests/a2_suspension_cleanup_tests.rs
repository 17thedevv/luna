//! A2-PROOF: suspension-state cleanup must use destruction semantics
//! (`owned && initialized && !fully_moved`), not mere liveness.
//!
//! These tests assert the *contract*. They are expected to FAIL on the current
//! implementation, pinning the root cause at the suspension-state -> drop_fn
//! boundary (live_places includes ConditionallyMoved; PartialMoved excluded).
use luna_borrowck::suspension::compute_suspension_states;
use luna_common::ids::SymbolId;
use luna_driver::async_lowering::lower_async;
use luna_mvir::{
    BasicBlock, Function, GlobalId, Instruction, LabelId, Module, Operand, Terminator, ValueData,
    ValueId, ValueOrigin,
};
use luna_semantic::symbol::{ScopeId, Symbol, SymbolKind};
use luna_semantic::{SemanticContext, SemanticType, SemanticTypeId};

fn push_symbol(ctx: &mut SemanticContext, name: &str, kind: SymbolKind) -> SymbolId {
    let sym = SymbolId(ctx.symbol_table.symbols.len() as u32);
    ctx.symbol_table.symbols.push(Symbol {
        id: sym,
        name: name.to_string(),
        ctxt: luna_common::ids::SyntaxContext::ROOT,
        kind,
        scope: ScopeId(0),
        span: luna_common::Span::default(),
        visibility: luna_ast::Visibility::Public,
        decl_id: None,
        inner_scope: None,
        provider_id: None,
    });
    sym
}

#[test]
fn a2_conditional_move_place_is_not_unconditionally_destroyed_at_suspension() {
    let mut ctx = SemanticContext::new();
    let struct_sym = push_symbol(&mut ctx, "Resource", SymbolKind::Struct);
    let drop_sym = push_symbol(&mut ctx, "drop", SymbolKind::Function);
    ctx.tables.drop_impls.insert(struct_sym, drop_sym);
    let struct_ty = ctx.types.intern(SemanticType::Struct(struct_sym, Vec::new(), Vec::new()));
    let i32_ty = SemanticTypeId(3); // i32
    let bool_ty = SemanticTypeId(1);

    // CFG: entry conditionally moves the resource on one path, then awaits.
    // At the await, the resource place is ConditionallyMoved.
    let func = Function {
        name: GlobalId { name: "worker_cond".to_string(), symbol_id: None },
        is_extern: false,
        is_async: true,
        arg_count: 0,
        link_name: None,
        param_types: vec![],
        ret_ty: i32_ty,
        lifetime_info: Default::default(),
        blocks: vec![
            BasicBlock {
                label: LabelId { name: "entry".to_string() },
                insts: vec![ValueId(0), ValueId(1), ValueId(2), ValueId(3)],
                terminator: Some(Terminator::CondBr {
                    condition: Operand::Value(ValueId(3)),
                    true_target: LabelId { name: "move_path".to_string() },
                    false_target: LabelId { name: "merge".to_string() },
                }),
            },
            BasicBlock {
                label: LabelId { name: "move_path".to_string() },
                insts: vec![ValueId(4)],
                terminator: Some(Terminator::Br { target: LabelId { name: "merge".to_string() } }),
            },
            BasicBlock {
                label: LabelId { name: "merge".to_string() },
                insts: vec![ValueId(5)],
                terminator: Some(Terminator::Ret { value: Some(Operand::Number("0".to_string())) }),
            },
        ],
        values: vec![
            // 0: resource place
            ValueData { inst: Instruction::Alloca, ty: struct_ty, span: None, origin: ValueOrigin::Local },
            ValueData { inst: Instruction::MarkInit { value: Operand::Value(ValueId(0)) }, ty: SemanticTypeId(0), span: None, origin: ValueOrigin::Temporary },
            // 2: move destination (so the value moves out of the resource place)
            ValueData { inst: Instruction::Alloca, ty: struct_ty, span: None, origin: ValueOrigin::Local },
            ValueData { inst: Instruction::Assign(Operand::Number("0".to_string())), ty: bool_ty, span: None, origin: ValueOrigin::Temporary },
            // 4: Store into destination, moving the resource on this path only
            ValueData { inst: Instruction::Store { ptr: Operand::Value(ValueId(2)), value: Operand::Value(ValueId(0)) }, ty: struct_ty, span: None, origin: ValueOrigin::Temporary },
            // 5: suspension point
            ValueData { inst: Instruction::Await { future: Operand::Number("1".to_string()) }, ty: i32_ty, span: None, origin: ValueOrigin::Temporary },
        ],
    };

    let states = compute_suspension_states(&func, Some(&ctx));
    let await_state = states
        .await_states
        .get(&ValueId(5))
        .expect("await state present");

    // Contract: a place that is only conditionally moved at the suspension must
    // not be destroyed unconditionally. It is either not in the destruction set,
    // or destroyed through a runtime init-state guard — never a bare drop.
    let unconditional: Vec<_> = await_state
        .live_places
        .iter()
        .filter(|place| place.local == ValueId(0))
        .collect();

    assert!(
        unconditional.is_empty(),
        "conditionally-moved place {:?} must not be in the unconditional cleanup set; \
         suspension cleanup currently uses liveness, not destruction semantics",
        unconditional
    );
}

#[test]
fn a2_partially_moved_aggregate_remaining_subplace_is_destroyed_at_suspension() {
    let mut ctx = SemanticContext::new();
    let res_sym = push_symbol(&mut ctx, "Resource", SymbolKind::Struct);
    let drop_sym = push_symbol(&mut ctx, "drop", SymbolKind::Function);
    ctx.tables.drop_impls.insert(res_sym, drop_sym);
    let res_ty = ctx.types.intern(SemanticType::Struct(res_sym, Vec::new(), Vec::new()));

    // Pair { first: Resource, second: Resource }
    let pair_sym = push_symbol(&mut ctx, "Pair", SymbolKind::Struct);
    let pair_ty = ctx.types.intern(SemanticType::Struct(pair_sym, Vec::new(), vec![res_ty, res_ty]));
    let i32_ty = SemanticTypeId(3);

    // Move `first` out (unconditional) before the await; `second` stays live.
    let func = Function {
        name: GlobalId { name: "worker_partial".to_string(), symbol_id: None },
        is_extern: false,
        is_async: true,
        arg_count: 0,
        link_name: None,
        param_types: vec![],
        ret_ty: i32_ty,
        lifetime_info: Default::default(),
        blocks: vec![BasicBlock {
            label: LabelId { name: "entry".to_string() },
            insts: vec![ValueId(0), ValueId(1), ValueId(2), ValueId(3), ValueId(4), ValueId(5), ValueId(6)],
            terminator: Some(Terminator::Ret { value: Some(Operand::Number("0".to_string())) }),
        }],
        values: vec![
            ValueData { inst: Instruction::Alloca, ty: pair_ty, span: None, origin: ValueOrigin::Local },
            ValueData { inst: Instruction::MarkInit { value: Operand::Value(ValueId(0)) }, ty: SemanticTypeId(0), span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Alloca, ty: res_ty, span: None, origin: ValueOrigin::Local },
            ValueData { inst: Instruction::FieldPtr { base: Operand::Value(ValueId(0)), field_idx: 0, field_name: None }, ty: pair_ty, span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Load { ptr: Operand::Value(ValueId(3)) }, ty: res_ty, span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Store { ptr: Operand::Value(ValueId(2)), value: Operand::Value(ValueId(4)) }, ty: res_ty, span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Await { future: Operand::Number("1".to_string()) }, ty: i32_ty, span: None, origin: ValueOrigin::Temporary },
        ],
    };

    let states = compute_suspension_states(&func, Some(&ctx));
    let await_state = states.await_states.get(&ValueId(6)).expect("await state present");

    // Contract: the remaining initialized subplace (`second`) must still be
    // destroyed on cancellation. It may appear as the aggregate with a remaining
    // projection, or as an explicit remaining field — never omitted entirely.
    let covered = await_state
        .live_places
        .iter()
        .any(|place| place.local == ValueId(0));
    assert!(
        covered,
        "remaining initialized subplace of a partially-moved aggregate must be destroyed \
         at suspension; got live set {:?}",
        await_state.live_places
    );
}

#[test]
fn a2_conditional_move_drop_is_guarded_in_generated_drop_fn() {
    let mut ctx = SemanticContext::new();
    let struct_sym = push_symbol(&mut ctx, "Resource", SymbolKind::Struct);
    let drop_sym = push_symbol(&mut ctx, "drop", SymbolKind::Function);
    ctx.tables.drop_impls.insert(struct_sym, drop_sym);
    let struct_ty = ctx.types.intern(SemanticType::Struct(struct_sym, Vec::new(), Vec::new()));
    let i32_ty = SemanticTypeId(3);
    let bool_ty = SemanticTypeId(1);

    let func = Function {
        name: GlobalId { name: "worker_cond".to_string(), symbol_id: None },
        is_extern: false,
        is_async: true,
        arg_count: 0,
        link_name: None,
        param_types: vec![],
        ret_ty: i32_ty,
        lifetime_info: Default::default(),
        blocks: vec![
            BasicBlock {
                label: LabelId { name: "entry".to_string() },
                insts: vec![ValueId(0), ValueId(1), ValueId(2), ValueId(3)],
                terminator: Some(Terminator::CondBr {
                    condition: Operand::Value(ValueId(3)),
                    true_target: LabelId { name: "move_path".to_string() },
                    false_target: LabelId { name: "merge".to_string() },
                }),
            },
            BasicBlock {
                label: LabelId { name: "move_path".to_string() },
                insts: vec![ValueId(4)],
                terminator: Some(Terminator::Br { target: LabelId { name: "merge".to_string() } }),
            },
            BasicBlock {
                label: LabelId { name: "merge".to_string() },
                insts: vec![ValueId(5)],
                terminator: Some(Terminator::Ret { value: Some(Operand::Number("0".to_string())) }),
            },
        ],
        values: vec![
            ValueData { inst: Instruction::Alloca, ty: struct_ty, span: None, origin: ValueOrigin::Local },
            ValueData { inst: Instruction::MarkInit { value: Operand::Value(ValueId(0)) }, ty: SemanticTypeId(0), span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Alloca, ty: struct_ty, span: None, origin: ValueOrigin::Local },
            ValueData { inst: Instruction::Assign(Operand::Number("0".to_string())), ty: bool_ty, span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Store { ptr: Operand::Value(ValueId(2)), value: Operand::Value(ValueId(0)) }, ty: struct_ty, span: None, origin: ValueOrigin::Temporary },
            ValueData { inst: Instruction::Await { future: Operand::Number("1".to_string()) }, ty: i32_ty, span: None, origin: ValueOrigin::Temporary },
        ],
    };

    let mut module = Module::new();
    module.functions.push(func);
    lower_async(&mut module, &mut ctx);

    let drop_fn = module
        .functions
        .iter()
        .find(|f| f.name.name == "worker_cond_drop")
        .expect("generated drop function");

    // The cancellation drop for a conditionally-moved capture must be guarded by
    // a runtime ownership flag (a conditional branch), not emitted unconditionally.
    let has_guard = drop_fn
        .blocks
        .iter()
        .any(|b| matches!(b.terminator, Some(Terminator::CondBr { .. })));
    assert!(has_guard, "conditional cancellation drop must be guarded by a runtime flag");

    let has_field_drop = drop_fn
        .values
        .iter()
        .any(|v| matches!(v.inst, Instruction::Drop { .. }));
    assert!(has_field_drop, "guarded drop must still destroy the capture when owned");
}

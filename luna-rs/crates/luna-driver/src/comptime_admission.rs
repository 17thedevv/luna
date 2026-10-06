use luna_ast::{AstArena, ExprId, Item, StmtId};
use luna_common::SourceManager;
use luna_mvir::interp::PreparedComptime;
use luna_semantic::{ComptimeEngine, ComptimeError, ComptimeValue, SemanticContext};

/// The driver owns phase orchestration; MVIR/VM do not implement loan policy.
pub(crate) struct CheckedComptimeEngine {
    pub max_steps: usize,
    pub max_depth: usize,
}

impl CheckedComptimeEngine {
    fn vm(&self) -> luna_mvir::MvirComptimeEngine {
        luna_mvir::MvirComptimeEngine { max_steps: self.max_steps, max_depth: self.max_depth }
    }

    fn admit(&self, mut program: PreparedComptime, arena: &AstArena, ctx: &SemanticContext, sources: &SourceManager) -> Result<PreparedComptime, ComptimeError> {
        let mut declarations = ctx.instantiated_functions.iter()
            .filter(|function| function.instance.closure_id.is_none())
            .map(|function| function.instance.decl_id).collect::<Vec<_>>();
        declarations.sort_by_key(|declaration| declaration.0);
        declarations.dedup();
        let items = declarations.into_iter().map(Item::Decl).collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        crate::verify_items_lifetime(&items, arena, ctx, sources, &mut diagnostics);
        if !diagnostics.is_empty() { return Err(ComptimeError::Diagnostics(diagnostics)); }
        crate::validate_mvir_module(&program.module, "comptime pre-borrow").map_err(ComptimeError::Diagnostics)?;
        let mut interprocedural = luna_borrowck::interprocedural::InterproceduralContext::with_context(ctx);
        interprocedural.compute_summaries(&program.module);
        for function in &mut program.module.functions {
            if function.is_extern || function.blocks.is_empty() { continue; }
            let (errors, redundant_drops) = luna_borrowck::borrow_check_function_with_drop_flags(function, ctx, &interprocedural.summaries);
            diagnostics.extend(errors);
            luna_borrowck::cleanup::eliminate_redundant_drops(function, &redundant_drops);
        }
        if !diagnostics.is_empty() { return Err(ComptimeError::Diagnostics(diagnostics)); }
        crate::validate_mvir_module(&program.module, "comptime post-borrow").map_err(ComptimeError::Diagnostics)?;
        Ok(program)
    }
}

impl ComptimeEngine for CheckedComptimeEngine {
    fn eval_expr(&self, arena: &AstArena, ctx: &SemanticContext, sources: &SourceManager, expression: ExprId) -> Result<ComptimeValue, ComptimeError> {
        let vm = self.vm();
        let program = self.admit(vm.prepare_expr(arena, ctx, sources, expression)?, arena, ctx, sources)?;
        vm.execute(&program, ctx)
    }

    fn eval_stmt(&self, arena: &AstArena, ctx: &SemanticContext, sources: &SourceManager, statement: StmtId) -> Result<ComptimeValue, ComptimeError> {
        let vm = self.vm();
        let program = self.admit(vm.prepare_stmt(arena, ctx, sources, statement)?, arena, ctx, sources)?;
        vm.execute(&program, ctx)
    }
}

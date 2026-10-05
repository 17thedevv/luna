use crate::{SemanticContext, ty::{SemanticTypeId, SemanticType, BuiltinType}};
use luna_ast::{AstArena, Item, Stmt, Expr, Decl};
use luna_lexer::{BuiltinKind, TokenKind};
use luna_common::diagnostic::{Diagnostic, DiagnosticCode};
#[path = "method_resolution.rs"]
mod method_resolution;
#[path = "call_binding.rs"]
mod call_binding;
#[path = "trait_bound_proof.rs"]
mod trait_bound_proof;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssociatedTypeEqObligation {
    pub projection: SemanticTypeId,
    pub target_ty: SemanticTypeId,
    pub span: luna_common::Span,
}

#[derive(Clone, Copy)]
enum ComptimeTarget {
    Expression(luna_ast::ExprId),
    Statement(luna_ast::StmtId),
}

pub struct TypeChecker<'a> {
    ctx: &'a mut SemanticContext,
    arena: &'a AstArena,
    source_manager: &'a luna_common::source::SourceManager,
    is_unsafe_context: bool,
    loop_depth: u32,
    active_lambdas: Vec<luna_ast::ExprId>,
    current_self_type: Option<SemanticTypeId>,
    current_async_fn: Option<luna_ast::DeclId>,
    comptime_engine: Option<&'a dyn crate::ComptimeEngine>,
    current_return_type: Vec<SemanticTypeId>,
    expected_expr_type: Vec<SemanticTypeId>,
    comptime_prepared_functions: std::collections::HashSet<luna_ast::DeclId>,
    populating_nominal_types: std::collections::HashSet<luna_ast::DeclId>,
    populated_nominal_types: std::collections::HashSet<luna_ast::DeclId>,
    current_scope: crate::ScopeId,
    pub current_trait: Option<luna_common::ids::SymbolId>,
    pub current_trait_impl: Option<luna_ast::DeclId>,
    pub normalization_stack: Vec<(SemanticTypeId, luna_common::ids::SymbolId, luna_common::ids::SymbolId)>,
    pub associated_type_obligations: Vec<AssociatedTypeEqObligation>,
}

impl<'a> TypeChecker<'a> {

    pub fn get_span_text(&self, span: luna_common::ids::Span) -> &'a str {
        &self.source_manager.get_file(span.file_id).unwrap().source[span.start as usize..span.end as usize]
    }

    pub fn new(ctx: &'a mut SemanticContext, arena: &'a AstArena, source_manager: &'a luna_common::source::SourceManager) -> Self {
        Self {
            ctx,
            arena,
            source_manager,
            is_unsafe_context: false,
            loop_depth: 0,
            active_lambdas: Vec::new(),
            current_self_type: None,
            current_async_fn: None,
            comptime_engine: None,
            current_return_type: Vec::new(),
            expected_expr_type: Vec::new(),
            comptime_prepared_functions: std::collections::HashSet::new(),
            populating_nominal_types: std::collections::HashSet::new(),
            populated_nominal_types: std::collections::HashSet::new(),
            current_scope: crate::ScopeId(0),
            current_trait: None,
            current_trait_impl: None,
            normalization_stack: Vec::new(),
            associated_type_obligations: Vec::new(),
        }
    }

    pub fn new_with_engine(ctx: &'a mut SemanticContext, arena: &'a AstArena, source_manager: &'a luna_common::source::SourceManager, comptime_engine: &'a dyn crate::ComptimeEngine) -> Self {
        Self {
            ctx,
            arena,
            source_manager,
            is_unsafe_context: false,
            loop_depth: 0,
            active_lambdas: Vec::new(),
            current_self_type: None,
            current_async_fn: None,
            comptime_engine: Some(comptime_engine),
            current_return_type: Vec::new(),
            expected_expr_type: Vec::new(),
            comptime_prepared_functions: std::collections::HashSet::new(),
            populating_nominal_types: std::collections::HashSet::new(),
            populated_nominal_types: std::collections::HashSet::new(),
            current_scope: crate::ScopeId(0),
            current_trait: None,
            current_trait_impl: None,
            normalization_stack: Vec::new(),
            associated_type_obligations: Vec::new(),
        }
    }

    pub fn eval_comptime_expr(&mut self, expr_id: luna_ast::ExprId) -> Result<crate::comptime::ComptimeValue, crate::comptime::ComptimeError> {
        self.eval_prepared_comptime(ComptimeTarget::Expression(expr_id))
    }

    pub fn eval_comptime_stmt(&mut self, stmt_id: luna_ast::StmtId) -> Result<crate::comptime::ComptimeValue, crate::comptime::ComptimeError> {
        self.eval_prepared_comptime(ComptimeTarget::Statement(stmt_id))
    }

    fn eval_prepared_comptime(&mut self, target: ComptimeTarget) -> Result<crate::comptime::ComptimeValue, crate::comptime::ComptimeError> {
        self.ctx.tables.evaluated_comptime = true;
        let Some(engine) = self.comptime_engine else {
            return Err(crate::comptime::ComptimeError::UnsupportedOperation("comptime engine not configured".to_string()));
        };
        self.prepare_comptime_bodies();
        let return_type = match target {
            ComptimeTarget::Expression(expr) => self.ctx.tables.expr_types.get(&expr).copied(),
            ComptimeTarget::Statement(stmt) => self.infer_stmt_value_type(&stmt),
        }.unwrap_or_else(|| self.ctx.types.intern(SemanticType::Void));
        let diagnostics_before = self.ctx.diagnostics.len();
        let mut collector = crate::mono::MonoCollector::new_with_source(self.ctx, self.arena, Some(self.source_manager));
        let mut root = match target {
            ComptimeTarget::Expression(expr) => collector.run_on_expr(expr),
            ComptimeTarget::Statement(stmt) => collector.run_on_stmt(stmt),
        };
        root.return_type = Some(return_type);
        let functions = collector.instantiated.into_values().collect();
        let glues = collector.drop_glues;
        if self.ctx.diagnostics.len() != diagnostics_before {
            return Err(crate::comptime::ComptimeError::TypeMismatch(self.ctx.diagnostics[diagnostics_before..]
                .iter().map(|diagnostic| diagnostic.message.clone()).collect::<Vec<_>>().join("; ")));
        }
        // Do not publish partial early instances as whole-program compilation
        // results. Interned types remain in this session so returned type IDs
        // stay valid; the evaluation root and reachable units are scoped.
        let previous_root = self.ctx.comptime_root.replace(root);
        let previous_functions = std::mem::replace(&mut self.ctx.instantiated_functions, functions);
        let previous_glues = std::mem::replace(&mut self.ctx.drop_glue_instances, glues);
        let result = match target {
            ComptimeTarget::Expression(expr) => engine.eval_expr(self.arena, self.ctx, self.source_manager, expr),
            ComptimeTarget::Statement(stmt) => engine.eval_stmt(self.arena, self.ctx, self.source_manager, stmt),
        };
        self.ctx.comptime_root = previous_root;
        self.ctx.instantiated_functions = previous_functions;
        self.ctx.drop_glue_instances = previous_glues;
        result
    }

    fn report_comptime_error(&mut self, error: crate::ComptimeError, span: luna_common::Span, context: &str) {
        match error {
            crate::ComptimeError::Diagnostics(diagnostics) => self.ctx.diagnostics.extend(diagnostics),
            other => self.ctx.diagnostics.push(Diagnostic::error(format!("{context}: {other}"))
                .with_code(DiagnosticCode::ComptimeEvaluationFailed).with_span(span)),
        }
    }
    
    /// Body elaboration belongs to semantic analysis, including bodies required
    /// by an early comptime evaluation. Follow already-typed call sites to a
    /// fixed point; MVIR must never guess types for an unelaborated callee.
    /// Imported providers have already elaborated their bodies and injected the
    /// resulting semantic tables. Their reconstructed export scopes are not
    /// lexical body scopes, so they must not be rechecked in this consumer.
    fn prepare_comptime_bodies(&mut self) {
        if self.comptime_engine.is_none() { return; }
        loop {
            let mut callees = Vec::new();
            for expr in self.ctx.tables.expr_types.keys() {
                let symbol = match &self.arena.exprs[expr.0 as usize] {
                    Expr::Call { callee, .. } => self.ctx.tables.expr_symbols.get(callee),
                    Expr::MethodCall { .. } => self.ctx.tables.expr_symbols.get(expr),
                    _ => None,
                };
                if let Some(decl) = symbol.and_then(|symbol| self.ctx.tables.symbol_decls.get(symbol)).copied() {
                    if !self.comptime_prepared_functions.contains(&decl)
                        && symbol.is_some_and(|symbol| {
                            let symbol = self.ctx.symbol_table.get_symbol(*symbol);
                            symbol.provider_id == self.ctx.current_provider && symbol.inner_scope.is_some()
                        })
                        && matches!(&self.arena.decls[decl.0 as usize], Decl::Function { body: Some(_), .. }) {
                        callees.push(decl);
                    }
                }
            }
            // A concrete-to-dyn coercion makes its implementation methods
            // reachable even though the call site names only a trait method.
            for coercion in self.ctx.tables.coercions.values() {
                if let crate::CoercionKind::ConcreteToDyn { trait_sym, concrete_sym } = coercion {
                    let key = crate::semantic_tables::ImplKey {
                        trait_id: Some(*trait_sym),
                        self_type_def: (*concrete_sym).into(),
                    };
                    let methods = self.ctx.tables.impl_methods.get(&key).into_iter().flatten()
                        .copied().chain(self.ctx.tables.drop_impls.get(concrete_sym).copied());
                    for symbol in methods {
                        if let Some(&decl) = self.ctx.tables.symbol_decls.get(&symbol) {
                            if !self.comptime_prepared_functions.contains(&decl)
                                && self.ctx.symbol_table.get_symbol(symbol).provider_id == self.ctx.current_provider
                                && self.ctx.symbol_table.get_symbol(symbol).inner_scope.is_some()
                                && matches!(&self.arena.decls[decl.0 as usize], Decl::Function { body: Some(_), .. }) {
                                callees.push(decl);
                            }
                        }
                    }
                }
            }
            if callees.is_empty() { break; }
            callees.sort_by_key(|decl| decl.0);
            callees.dedup();
            for decl in callees {
                if !self.comptime_prepared_functions.insert(decl) { continue; }
                let saved_expected = std::mem::take(&mut self.expected_expr_type);
                let saved_lambdas = std::mem::take(&mut self.active_lambdas);
                let saved_unsafe = self.is_unsafe_context;
                let saved_loop_depth = self.loop_depth;
                let saved_self = self.current_self_type;
                let saved_trait = self.current_trait;
                let saved_impl = self.current_trait_impl;
                let saved_provider = self.ctx.current_provider;
                self.is_unsafe_context = false;
                self.loop_depth = 0;
                self.current_trait = None;
                self.ctx.current_provider = self.ctx.tables.decl_symbols.get(&decl)
                    .and_then(|symbol| self.ctx.symbol_table.get_symbol(*symbol).provider_id);
                self.current_trait_impl = self.ctx.tables.method_to_impl_decl.get(&decl).copied();
                self.current_self_type = self.current_trait_impl.and_then(|impl_decl| {
                    self.ctx.tables.checked_impl_headers.get(&impl_decl).map(|header| header.self_type)
                });
                self.typecheck_item(&Item::Decl(decl));
                self.current_self_type = saved_self;
                self.current_trait = saved_trait;
                self.current_trait_impl = saved_impl;
                self.ctx.current_provider = saved_provider;
                self.loop_depth = saved_loop_depth;
                self.is_unsafe_context = saved_unsafe;
                self.active_lambdas = saved_lambdas;
                self.expected_expr_type = saved_expected;
            }
        }
    }

    pub fn unify(&mut self, expected: SemanticTypeId, actual: SemanticTypeId) -> Result<(), String> {
        let expected = self.normalize_type(expected);
        let actual = self.normalize_type(actual);
        let expected = self.ctx.types.resolve(expected);
        let actual = self.ctx.types.resolve(actual);
        
        let t1 = self.ctx.types.get(expected).clone();
        let t2 = self.ctx.types.get(actual).clone();
        
        if expected == actual {
            return Ok(());
        }

        // Poison containment: Error types unify with everything to prevent cascading diagnostics
        if matches!(t1, SemanticType::Error) || matches!(t2, SemanticType::Error) {
            return Ok(());
        }

        match (t1, t2) {
            (SemanticType::InferenceVar(var), _) => {
                if self.ctx.types.occurs_check(var, actual) {
                    return Err(format!("cycle detected during type inference"));
                }
                self.ctx.types.inference_bindings.insert(var, actual);
                Ok(())
            }
            (_, SemanticType::InferenceVar(var)) => {
                if self.ctx.types.occurs_check(var, expected) {
                    return Err(format!("cycle detected during type inference"));
                }
                self.ctx.types.inference_bindings.insert(var, expected);
                Ok(())
            }
            (SemanticType::Projection { .. }, _) => {
                self.associated_type_obligations.push(AssociatedTypeEqObligation {
                    projection: expected,
                    target_ty: actual,
                    span: luna_common::Span::new(luna_common::ids::FileId(0), 0, 0),
                });
                Ok(())
            }
            (_, SemanticType::Projection { .. }) => {
                self.associated_type_obligations.push(AssociatedTypeEqObligation {
                    projection: actual,
                    target_ty: expected,
                    span: luna_common::Span::new(luna_common::ids::FileId(0), 0, 0),
                });
                Ok(())
            }
            (_, SemanticType::Never) => Ok(()),
            (SemanticType::GenericParam(_), _) | (_, SemanticType::GenericParam(_)) => {
                Err("Rigid generic parameter does not match the other type; instantiate the declaration's binder explicitly".to_string())
            }
            (SemanticType::Pointer(m1, i1), SemanticType::Pointer(m2, i2)) if m1 == m2 => {
                self.unify(i1, i2)
            }
            (SemanticType::Reference(l1, m1, i1), SemanticType::Reference(l2, m2, i2)) if m1 == m2 && l1 == l2 => {
                self.unify(i1, i2)
            }
            (SemanticType::Reference(_, m1, i1), SemanticType::Pointer(m2, i2)) if m1 == m2 => {
                self.unify(i1, i2)
            }
            (SemanticType::Pointer(m1, i1), SemanticType::Reference(_, m2, i2)) if m1 == m2 => {
                self.unify(i1, i2)
            }
            (SemanticType::Void, SemanticType::Tuple(els)) if els.is_empty() => Ok(()),
            (SemanticType::Tuple(els), SemanticType::Void) if els.is_empty() => Ok(()),
            (SemanticType::Tuple(els1), SemanticType::Tuple(els2)) if els1.len() == els2.len() => {
                for (e1, e2) in els1.into_iter().zip(els2.into_iter()) {
                    self.unify(e1, e2)?;
                }
                Ok(())
            }
            (SemanticType::Array(i1, s1), SemanticType::Array(i2, s2)) if s1 == s2 => {
                self.unify(i1, i2)
            }
            (SemanticType::Slice(i1), SemanticType::Slice(i2)) => {
                self.unify(i1, i2)
            }
            (SemanticType::Struct(s1, args1, _), SemanticType::Struct(s2, args2, _)) if s1 == s2 && args1.len() == args2.len() => {
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    self.unify(*a1, *a2)?;
                }
                Ok(())
            }
            (SemanticType::Enum(e1, args1, _), SemanticType::Enum(e2, args2, _)) if e1 == e2 && args1.len() == args2.len() => {
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    self.unify(*a1, *a2)?;
                }
                Ok(())
            }
            (SemanticType::Function { params: p1, return_type: r1, is_unsafe: u1 }, SemanticType::Function { params: p2, return_type: r2, is_unsafe: u2 }) if p1.len() == p2.len() && u1 == u2 => {
                for (a1, a2) in p1.into_iter().zip(p2.into_iter()) {
                    self.unify(a1, a2)?;
                }
                self.unify(r1, r2)
            }
            (SemanticType::Closure(_, p1, r1), SemanticType::Closure(_, p2, r2)) if p1.len() == p2.len() => {
                for (a, b) in p1.into_iter().zip(p2.into_iter()) {
                    self.unify(a, b)?;
                }
                self.unify(r1, r2)
            }
            (SemanticType::Future(o1), SemanticType::Future(o2)) => {
                self.unify(o1, o2)
            }
            _ => {
                let exp_sem = self.ctx.types.get(expected);
                let act_sem = self.ctx.types.get(actual);
                Err(format!("type mismatch: expected {:?}, got {:?}", exp_sem, act_sem))
            }
        }
    }

    pub fn contains_ffi_provenance_capability(&self, ty_id: SemanticTypeId) -> bool {
        let mut visited = std::collections::HashSet::new();
        self.contains_ffi_provenance_capability_inner(ty_id, &mut visited)
    }

    fn contains_ffi_provenance_capability_inner(
        &self,
        ty_id: SemanticTypeId,
        visited: &mut std::collections::HashSet<SemanticTypeId>,
    ) -> bool {
        let resolved_id = self.ctx.types.resolve(ty_id);
        if !visited.insert(resolved_id) {
            return false;
        }

        let ty = self.ctx.types.get(resolved_id).clone();
        match ty {
            SemanticType::Reference(..) | SemanticType::Pointer(..) | SemanticType::Slice(..) => true,
            SemanticType::Struct(sym_id, _, ref field_tys) => {
                if field_tys.iter().any(|&f| self.contains_ffi_provenance_capability_inner(f, visited)) {
                    return true;
                }
                if let Some(full_ty_id) = self.ctx.tables.symbol_types.get(&sym_id).copied() {
                    if full_ty_id != ty_id && full_ty_id != resolved_id {
                        if let SemanticType::Struct(_, _, ref full_fields) = self.ctx.types.get(full_ty_id).clone() {
                            if full_fields.iter().any(|&f| self.contains_ffi_provenance_capability_inner(f, visited)) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            SemanticType::Enum(_, _, ref variant_tys) => {
                variant_tys.iter().any(|&v| self.contains_ffi_provenance_capability_inner(v, visited))
            }
            SemanticType::Tuple(ref elems) => {
                elems.iter().any(|&e| self.contains_ffi_provenance_capability_inner(e, visited))
            }
            SemanticType::Array(elem, _) => {
                self.contains_ffi_provenance_capability_inner(elem, visited)
            }
            SemanticType::Function { .. } => false,
            SemanticType::Primitive(_) | SemanticType::Void | SemanticType::Never | SemanticType::Error => false,
            _ => false,
        }
    }

    pub fn is_ffi_safe_param(&self, ty_id: SemanticTypeId) -> Result<(), String> {
        let resolved_id = self.ctx.types.resolve(ty_id);
        let ty = self.ctx.types.get(resolved_id).clone();
        match ty {
            SemanticType::Primitive(_) | SemanticType::Void => Ok(()),
            SemanticType::Pointer(_, _) => Ok(()),
            SemanticType::Reference(_, _, _inner) => Ok(()),
            SemanticType::Function { ref params, return_type, .. } => {
                for &p in params {
                    self.is_ffi_safe_param(p)?;
                }
                self.is_ffi_safe_return(return_type)?;
                Ok(())
            }
            SemanticType::Struct(sym_id, _, ref field_tys) => {
                if self.contains_ffi_provenance_capability(resolved_id) {
                    return Err("By-value aggregate containing reference or pointer capability is not supported across FFI boundary".to_string());
                }
                if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym_id) {
                    if let Decl::Struct { annotations, .. } = &self.arena.decls[decl_id.0 as usize] {
                        let has_repr_c = annotations.iter().any(|a| {
                            let name = self.get_span_text(a.name);
                            name == "repr"
                        });
                        if !has_repr_c {
                            return Err("Struct is not marked with #[repr(C)]".to_string());
                        }
                    }
                }
                for &f in field_tys {
                    self.is_ffi_safe_field(f)?;
                }
                Ok(())
            }
            SemanticType::Tuple(_) | SemanticType::Enum(..) | SemanticType::Array(..) => {
                if self.contains_ffi_provenance_capability(resolved_id) {
                    return Err("By-value aggregate containing reference or pointer capability is not supported across FFI boundary".to_string());
                }
                Err("Type is not FFI-safe".to_string())
            }
            _ => Err("Parameter type is not FFI-safe".to_string()),
        }
    }

    pub fn is_ffi_safe_return(&self, ty_id: SemanticTypeId) -> Result<(), String> {
        let resolved_id = self.ctx.types.resolve(ty_id);
        let ty = self.ctx.types.get(resolved_id).clone();
        match ty {
            SemanticType::Primitive(_) | SemanticType::Void | SemanticType::Never => Ok(()),
            SemanticType::Pointer(_, _) => Ok(()),
            SemanticType::Reference(..) => {
                Err("Extern function returning safe reference is not supported without an explicit lifetime contract".to_string())
            }
            SemanticType::Function { ref params, return_type, .. } => {
                for &p in params {
                    self.is_ffi_safe_param(p)?;
                }
                self.is_ffi_safe_return(return_type)?;
                Ok(())
            }
            SemanticType::Struct(sym_id, _, ref field_tys) => {
                if self.contains_ffi_provenance_capability(resolved_id) {
                    return Err("By-value aggregate containing reference or pointer capability is not supported across FFI boundary".to_string());
                }
                if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym_id) {
                    if let Decl::Struct { annotations, .. } = &self.arena.decls[decl_id.0 as usize] {
                        let has_repr_c = annotations.iter().any(|a| {
                            let name = self.get_span_text(a.name);
                            name == "repr"
                        });
                        if !has_repr_c {
                            return Err("Struct is not marked with #[repr(C)]".to_string());
                        }
                    }
                }
                for &f in field_tys {
                    self.is_ffi_safe_field(f)?;
                }
                Ok(())
            }
            SemanticType::Tuple(_) | SemanticType::Enum(..) | SemanticType::Array(..) => {
                if self.contains_ffi_provenance_capability(resolved_id) {
                    return Err("By-value aggregate containing reference or pointer capability is not supported across FFI boundary".to_string());
                }
                Err("Type is not FFI-safe".to_string())
            }
            _ => Err("Return type is not FFI-safe".to_string()),
        }
    }

    pub fn is_ffi_safe_field(&self, ty_id: SemanticTypeId) -> Result<(), String> {
        let resolved_id = self.ctx.types.resolve(ty_id);
        let ty = self.ctx.types.get(resolved_id).clone();
        match ty {
            SemanticType::Primitive(_) | SemanticType::Void => Ok(()),
            SemanticType::Pointer(_, _) => {
                Err("By-value aggregate containing pointer capability is not supported across FFI boundary".to_string())
            }
            SemanticType::Reference(..) => {
                Err("By-value aggregate containing safe reference is not supported across FFI boundary".to_string())
            }
            SemanticType::Function { ref params, return_type, .. } => {
                for &p in params {
                    self.is_ffi_safe_param(p)?;
                }
                self.is_ffi_safe_return(return_type)?;
                Ok(())
            }
            SemanticType::Struct(sym_id, _, ref field_tys) => {
                if self.contains_ffi_provenance_capability(resolved_id) {
                    return Err("By-value aggregate containing reference or pointer capability is not supported across FFI boundary".to_string());
                }
                if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym_id) {
                    if let Decl::Struct { annotations, .. } = &self.arena.decls[decl_id.0 as usize] {
                        let has_repr_c = annotations.iter().any(|a| {
                            let name = self.get_span_text(a.name);
                            name == "repr"
                        });
                        if !has_repr_c {
                            return Err("Struct is not marked with #[repr(C)]".to_string());
                        }
                    }
                }
                for &f in field_tys {
                    self.is_ffi_safe_field(f)?;
                }
                Ok(())
            }
            _ => Err("Field type is not FFI-safe".to_string()),
        }
    }

    pub fn is_ffi_safe(&self, ty_id: SemanticTypeId) -> Result<(), String> {
        let resolved_id = self.ctx.types.resolve(ty_id);
        let ty = self.ctx.types.get(resolved_id).clone();
        match ty {
            SemanticType::Function { params, return_type, .. } => {
                for p in params {
                    self.is_ffi_safe_param(p)?;
                }
                self.is_ffi_safe_return(return_type)?;
                Ok(())
            }
            _ => self.is_ffi_safe_param(ty_id),
        }
    }

    pub fn normalize_projection(
        &mut self,
        self_ty: SemanticTypeId,
        trait_id: luna_common::ids::SymbolId,
        assoc_type: luna_common::ids::SymbolId,
        span: luna_common::Span,
    ) -> SemanticTypeId {
        let self_ty = self.ctx.types.resolve(self_ty);

        // 1. Check if self_ty is symbolic (InferenceVar or GenericParam)
        match self.ctx.types.get(self_ty) {
            SemanticType::InferenceVar(_) | SemanticType::GenericParam(_) => {
                let proj = self.ctx.types.intern(SemanticType::Projection {
                    self_type: self_ty,
                    trait_id,
                    assoc_type,
                });
                for obl in &self.associated_type_obligations {
                    if obl.projection == proj {
                        return obl.target_ty;
                    }
                }
                return proj;
            }
            SemanticType::Error => {
                return self.ctx.types.intern(SemanticType::Error);
            }
            _ => {}
        }

        // 2. Cycle Detection
        let key_tuple = (self_ty, trait_id, assoc_type);
        if self.normalization_stack.contains(&key_tuple) {
            let assoc_name = self.ctx.symbol_table.get_symbol(assoc_type).name.clone();
            let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "E_ASSOC_TYPE_CYCLE: Cycle detected while normalizing associated type projection `<_ as {}>::{}`",
                    trait_name, assoc_name
                ))
                .with_code(DiagnosticCode::AssociatedTypeCycle)
                .with_span(span)
            );
            return self.ctx.types.intern(SemanticType::Error);
        }

        self.normalization_stack.push(key_tuple);
        let result = self.normalize_projection_concrete(self_ty, trait_id, assoc_type, span);
        self.normalization_stack.pop();
        result
    }

    fn normalize_projection_concrete(
        &mut self,
        self_ty: SemanticTypeId,
        trait_id: luna_common::ids::SymbolId,
        assoc_type: luna_common::ids::SymbolId,
        span: luna_common::Span,
    ) -> SemanticTypeId {
        let sem_ty = self.ctx.types.get(self_ty).clone();
        let (nominal_sym, generic_args) = match &sem_ty {
            SemanticType::Struct(s, args, _) => (*s, args.clone()),
            SemanticType::Enum(e, args, _) => (*e, args.clone()),
            SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                return self.normalize_projection(*inner, trait_id, assoc_type, span);
            }
            _ => {
                let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
                let assoc_name = self.ctx.symbol_table.get_symbol(assoc_type).name.clone();
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!(
                        "E_UNRESOLVED_TRAIT_IMPL: Type `{:?}` does not implement trait `{}` for projection `{}`",
                        sem_ty, trait_name, assoc_name
                    ))
                    .with_code(DiagnosticCode::UnresolvedTraitImpl)
                    .with_span(span)
                );
                return self.ctx.types.intern(SemanticType::Error);
            }
        };

        let impl_key = crate::semantic_tables::ImplKey {
            trait_id: Some(trait_id),
            self_type_def: nominal_sym.into(),
        };

        // 1. Try matching against trait_impl_entries
        let mut matched_entries = Vec::new();
        for entry in &self.ctx.tables.trait_impl_entries {
            if entry.trait_id == trait_id {
                let mut test_subst = crate::ty::Substitution::new();
                if self.ctx.matches_impl_pattern(entry.self_type, self_ty, &entry.generic_params, &mut test_subst) {
                    matched_entries.push((entry.clone(), test_subst));
                }
            }
        }

        if matched_entries.len() > 1 {
            let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
            let assoc_name = self.ctx.symbol_table.get_symbol(assoc_type).name.clone();
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "E_AMBIGUOUS_ASSOCIATED_TYPE: Multiple conflicting implementations found for associated type projection `<_ as {}>::{}`",
                    trait_name, assoc_name
                ))
                .with_code(DiagnosticCode::AmbiguousAssociatedType)
                .with_span(span)
            );
            return self.ctx.types.intern(SemanticType::Error);
        }

        let (target_ty, subst) = if matched_entries.len() == 1 {
            let (matched_entry, subst) = matched_entries.into_iter().next().unwrap();
            let target_ty = if let Some(did) = matched_entry.decl_id {
                self.ctx.tables.decl_associated_types.get(&(did, assoc_type)).copied()
                    .or_else(|| self.ctx.tables.impl_associated_types.get(&(impl_key.clone(), assoc_type)).copied())
            } else {
                self.ctx.tables.impl_associated_types.get(&(impl_key.clone(), assoc_type)).copied()
            };
            (target_ty, subst)
        } else {
            // Fallback for mock contexts / legacy table entries
            let decl_ids = match self.ctx.tables.trait_impls.get(&impl_key) {
                Some(ids) if !ids.is_empty() => ids.clone(),
                _ => {
                    let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
                    let assoc_name = self.ctx.symbol_table.get_symbol(assoc_type).name.clone();
                    self.ctx.diagnostics.push(
                        Diagnostic::error(format!(
                            "E_UNRESOLVED_TRAIT_IMPL: Trait implementation `{}` not found for associated type projection `{}`",
                            trait_name, assoc_name
                        ))
                        .with_code(DiagnosticCode::UnresolvedTraitImpl)
                        .with_span(span)
                    );
                    return self.ctx.types.intern(SemanticType::Error);
                }
            };

            if decl_ids.len() > 1 {
                let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
                let assoc_name = self.ctx.symbol_table.get_symbol(assoc_type).name.clone();
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!(
                        "E_AMBIGUOUS_ASSOCIATED_TYPE: Multiple conflicting implementations found for associated type projection `<_ as {}>::{}`",
                        trait_name, assoc_name
                    ))
                    .with_code(DiagnosticCode::AmbiguousAssociatedType)
                    .with_span(span)
                );
                return self.ctx.types.intern(SemanticType::Error);
            }

            let target = self.ctx.tables.impl_associated_types.get(&(impl_key.clone(), assoc_type)).copied();
            let mut subst = crate::ty::Substitution::new();
            if let Some(&impl_self_ty) = self.ctx.tables.impl_self_types.get(&impl_key) {
                self.match_types(impl_self_ty, self_ty, &mut subst);
            }
            (target, subst)
        };

        let target_ty = match target_ty {
            Some(t) => t,
            None => {
                let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
                let assoc_name = self.ctx.symbol_table.get_symbol(assoc_type).name.clone();
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!(
                        "E_NO_ASSOCIATED_TYPE: Associated type `{}` is not defined in matching impl of `{}`",
                        assoc_name, trait_name
                    ))
                    .with_code(DiagnosticCode::NoAssociatedType)
                    .with_span(span)
                );
                return self.ctx.types.intern(SemanticType::Error);
            }
        };

        let substituted_ty = self.ctx.types.subst(target_ty, &subst);
        self.normalize_type(substituted_ty)
    }

    fn match_types(&self, pattern_ty: SemanticTypeId, concrete_ty: SemanticTypeId, subst: &mut crate::ty::Substitution) {
        let pattern_ty = self.ctx.types.resolve(pattern_ty);
        let concrete_ty = self.ctx.types.resolve(concrete_ty);
        match (self.ctx.types.get(pattern_ty), self.ctx.types.get(concrete_ty)) {
            (&SemanticType::GenericParam(gp), _) => {
                subst.insert(gp, concrete_ty);
            }
            (&SemanticType::Struct(s1, ref args1, _), &SemanticType::Struct(s2, ref args2, _)) if s1 == s2 => {
                for (&a1, &a2) in args1.iter().zip(args2.iter()) {
                    self.match_types(a1, a2, subst);
                }
            }
            (&SemanticType::Enum(e1, ref args1, _), &SemanticType::Enum(e2, ref args2, _)) if e1 == e2 => {
                for (&a1, &a2) in args1.iter().zip(args2.iter()) {
                    self.match_types(a1, a2, subst);
                }
            }
            (&SemanticType::Tuple(ref args1), &SemanticType::Tuple(ref args2)) => {
                for (&a1, &a2) in args1.iter().zip(args2.iter()) {
                    self.match_types(a1, a2, subst);
                }
            }
            (&SemanticType::Reference(_, _, inner1), &SemanticType::Reference(_, _, inner2)) |
            (&SemanticType::Pointer(_, inner1), &SemanticType::Pointer(_, inner2)) => {
                self.match_types(inner1, inner2, subst);
            }
            _ => {}
        }
    }

    pub fn normalize_type(&mut self, ty_id: SemanticTypeId) -> SemanticTypeId {
        let ty_id = self.ctx.types.resolve(ty_id);
        let ty = self.ctx.types.get(ty_id).clone();
        match ty {
            SemanticType::Projection { self_type, trait_id, assoc_type } => {
                let norm_self = self.normalize_type(self_type);
                self.normalize_projection(norm_self, trait_id, assoc_type, luna_common::Span::new(luna_common::ids::FileId(0), 0, 0))
            }
            SemanticType::Struct(sym, args, fields) => {
                let mut changed = false;
                let mut new_args = Vec::with_capacity(args.len());
                for a in args {
                    let na = self.normalize_type(a);
                    if na != a { changed = true; }
                    new_args.push(na);
                }
                let mut new_fields = Vec::with_capacity(fields.len());
                for f in fields {
                    let nf = self.normalize_type(f);
                    if nf != f { changed = true; }
                    new_fields.push(nf);
                }
                if changed {
                    self.ctx.types.intern(SemanticType::Struct(sym, new_args, new_fields))
                } else {
                    ty_id
                }
            }
            SemanticType::Enum(sym, args, variants) => {
                let mut changed = false;
                let mut new_args = Vec::with_capacity(args.len());
                for a in args {
                    let na = self.normalize_type(a);
                    if na != a { changed = true; }
                    new_args.push(na);
                }
                if changed {
                    self.ctx.types.intern(SemanticType::Enum(sym, new_args, variants))
                } else {
                    ty_id
                }
            }
            SemanticType::Tuple(elems) => {
                let mut changed = false;
                let mut new_elems = Vec::with_capacity(elems.len());
                for e in elems {
                    let ne = self.normalize_type(e);
                    if ne != e { changed = true; }
                    new_elems.push(ne);
                }
                if changed {
                    self.ctx.types.intern(SemanticType::Tuple(new_elems))
                } else {
                    ty_id
                }
            }
            SemanticType::Array(elem, size) => {
                let ne = self.normalize_type(elem);
                if ne != elem {
                    self.ctx.types.intern(SemanticType::Array(ne, size))
                } else {
                    ty_id
                }
            }
            SemanticType::Slice(elem) => {
                let ne = self.normalize_type(elem);
                if ne != elem {
                    self.ctx.types.intern(SemanticType::Slice(ne))
                } else {
                    ty_id
                }
            }
            SemanticType::Pointer(m, inner) => {
                let ni = self.normalize_type(inner);
                if ni != inner {
                    self.ctx.types.intern(SemanticType::Pointer(m, ni))
                } else {
                    ty_id
                }
            }
            SemanticType::Reference(l, m, inner) => {
                let ni = self.normalize_type(inner);
                if ni != inner {
                    self.ctx.types.intern(SemanticType::Reference(l, m, ni))
                } else {
                    ty_id
                }
            }
            SemanticType::Function { params, return_type, is_unsafe } => {
                let mut changed = false;
                let mut new_params = Vec::with_capacity(params.len());
                for p in params {
                    let np = self.normalize_type(p);
                    if np != p { changed = true; }
                    new_params.push(np);
                }
                let nr = self.normalize_type(return_type);
                if nr != return_type { changed = true; }
                if changed {
                    self.ctx.types.intern(SemanticType::Function { params: new_params, return_type: nr, is_unsafe })
                } else {
                    ty_id
                }
            }
            SemanticType::Future(inner) => {
                let ni = self.normalize_type(inner);
                if ni != inner {
                    self.ctx.types.intern(SemanticType::Future(ni))
                } else {
                    ty_id
                }
            }
            _ => ty_id,
        }
    }

    pub fn solve_associated_type_obligations(&mut self) {
        let obligations = std::mem::take(&mut self.associated_type_obligations);
        for obl in obligations {
            let norm_proj = self.normalize_type(obl.projection);
            let norm_target = self.normalize_type(obl.target_ty);
            if let Err(e) = self.unify(norm_proj, norm_target) {
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!("Associated type obligation failed: {}", e))
                        .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                        .with_span(obl.span)
                );
            }
        }
    }

    fn lower_associated_type_projection(
        &mut self,
        self_ty: SemanticTypeId,
        assoc_name: &str,
        span: luna_common::Span,
    ) -> SemanticTypeId {
        let self_ty = self.ctx.types.resolve(self_ty);
        let mut candidate_traits = Vec::new();

        // 1. If currently inside an impl of a trait, check if current_trait defines this assoc_name
        if let Some(cur_trait) = self.current_trait {
            if self.ctx.tables.assoc_type_names.contains_key(&(cur_trait, assoc_name.to_string())) {
                candidate_traits.push(cur_trait);
            }
        }

        // 2. If self_ty is GenericParam(gp_sym)
        let sem_ty = self.ctx.types.get(self_ty).clone();
        if let SemanticType::GenericParam(gp_sym) = sem_ty {
            if let Some(bounds) = self.ctx.tables.trait_bounds.get(&gp_sym) {
                for bound in bounds {
                    if self.ctx.tables.assoc_type_names.contains_key(&(bound.trait_id, assoc_name.to_string())) {
                        if !candidate_traits.contains(&bound.trait_id) {
                            candidate_traits.push(bound.trait_id);
                        }
                    }
                }
            }
        } else {
            // Nominal type (Struct / Enum) or Primitive
            let self_key_opt: Option<crate::semantic_tables::ImplSelfTypeKey> = match &sem_ty {
                SemanticType::Struct(s, _, _) => Some((*s).into()),
                SemanticType::Enum(e, _, _) => Some((*e).into()),
                SemanticType::Primitive(b) => Some((*b).into()),
                _ => None,
            };
            if let Some(self_key) = self_key_opt {
                for (impl_key, _) in &self.ctx.tables.trait_impls {
                    if impl_key.self_type_def == self_key {
                        if let Some(t_id) = impl_key.trait_id {
                            if self.ctx.tables.assoc_type_names.contains_key(&(t_id, assoc_name.to_string())) {
                                if !candidate_traits.contains(&t_id) {
                                    candidate_traits.push(t_id);
                                }
                            }
                        }
                    }
                }
                if candidate_traits.is_empty() {
                    for (&(t_id, ref a_name), _) in &self.ctx.tables.assoc_type_names {
                        if a_name == assoc_name {
                            if !candidate_traits.contains(&t_id) {
                                candidate_traits.push(t_id);
                            }
                        }
                    }
                }
            }
        }

        if candidate_traits.len() > 1 {
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "E_AMBIGUOUS_ASSOCIATED_TYPE: Associated type `{}` is ambiguous between multiple candidate traits",
                    assoc_name
                ))
                .with_code(DiagnosticCode::AmbiguousAssociatedType)
                .with_span(span)
            );
            return self.ctx.types.intern(SemanticType::Error);
        }

        if candidate_traits.is_empty() {
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "E_UNRESOLVED_TRAIT_IMPL: No candidate trait defines associated type `{}` for this type",
                    assoc_name
                ))
                .with_code(DiagnosticCode::UnresolvedTraitImpl)
                .with_span(span)
            );
            return self.ctx.types.intern(SemanticType::Error);
        }

        let trait_id = candidate_traits[0];
        let assoc_sym = *self.ctx.tables.assoc_type_names.get(&(trait_id, assoc_name.to_string())).unwrap();
        self.normalize_projection(self_ty, trait_id, assoc_sym, span)
    }

    fn instantiate_generic_named_type(
        &mut self,
        sym: luna_common::ids::SymbolId,
        base_ty: SemanticTypeId,
        generic_args: &[luna_ast::TypeId],
    ) -> SemanticTypeId {
        if generic_args.is_empty() {
            return base_ty;
        }
        if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym).copied() {
            let decl = &self.arena.decls[decl_id.0 as usize];
            let generic_params = match decl {
                luna_ast::Decl::Struct { generic_params, .. } => Some(generic_params),
                luna_ast::Decl::Enum { generic_params, .. } => Some(generic_params),
                _ => None,
            };
            if let Some(gp_list) = generic_params {
                let mut concrete_args = Vec::new();
                for arg in generic_args {
                    concrete_args.push(self.lower_type(*arg));
                }
                let mut subst = crate::ty::Substitution::new();
                for (idx, _gp) in gp_list.iter().enumerate() {
                    if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(decl_id, idx)) {
                        if idx < concrete_args.len() {
                            subst.insert(*gp_sym, concrete_args[idx]);
                        }
                    }
                }
                let resolved_ty = self.ctx.types.get(base_ty).clone();
                match resolved_ty {
                    SemanticType::Struct(s_sym, _, original_field_tys) => {
                        let mut field_tys = Vec::new();
                        for orig_ty in original_field_tys {
                            field_tys.push(self.ctx.types.subst(orig_ty, &subst));
                        }
                        self.ctx.types.intern(SemanticType::Struct(s_sym, concrete_args, field_tys))
                    }
                    SemanticType::Enum(e_sym, _, original_var_tys) => {
                        let mut variant_tys = Vec::new();
                        for orig_ty in original_var_tys {
                            variant_tys.push(self.ctx.types.subst(orig_ty, &subst));
                        }
                        self.ctx.types.intern(SemanticType::Enum(e_sym, concrete_args, variant_tys))
                    }
                    _ => base_ty,
                }
            } else {
                base_ty
            }
        } else {
            base_ty
        }
    }

    pub fn typecheck_items(&mut self, items: &[Item]) {
        self.populate_signatures(items);
        self.check_recursive_types(items);
        for item in items {
            self.typecheck_item(item);
        }
        self.solve_associated_type_obligations();
        self.check_copy_drop_invariants();

        let expr_keys: Vec<_> = self.ctx.tables.expr_types.keys().copied().collect();
        for k in expr_keys {
            let ty = self.ctx.tables.expr_types[&k];
            let resolved = self.ctx.types.resolve(ty);
            self.ctx.tables.expr_types.insert(k, resolved);
        }
        let sym_keys: Vec<_> = self.ctx.tables.symbol_types.keys().copied().collect();
        for k in sym_keys {
            let ty = self.ctx.tables.symbol_types[&k];
            let resolved = self.ctx.types.resolve(ty);
            self.ctx.tables.symbol_types.insert(k, resolved);
        }
        let pat_keys: Vec<_> = self.ctx.tables.pat_types.keys().copied().collect();
        for k in pat_keys {
            let ty = self.ctx.tables.pat_types[&k];
            let resolved = self.ctx.types.resolve(ty);
            self.ctx.tables.pat_types.insert(k, resolved);
        }
    }

    
    fn check_copy_drop_invariants(&mut self) {
        let copy_trait = match self.ctx.lang_items.get(crate::lang_item::LangItem::Copy) {
            Some(sym) => sym,
            None => return,
        };

        for (impl_key, decl_ids) in &self.ctx.tables.trait_impls {
            if impl_key.trait_id == Some(copy_trait) {
                if let crate::semantic_tables::ImplSelfTypeKey::Nominal(self_ty_sym) = impl_key.self_type_def {
                    if self.ctx.tables.raw_storage_anchor_contracts.contains_key(&self_ty_sym) {
                        for &decl_id in decl_ids {
                            let span = match &self.arena.decls[decl_id.0 as usize] {
                                luna_ast::Decl::Impl { self_type, .. } => match &self.arena.types[self_type.0 as usize] {
                                    luna_ast::Type::Named { segments, .. } => segments.last().copied().unwrap_or_default(),
                                    _ => luna_common::Span::new(luna_common::ids::FileId(0), 0, 0),
                                },
                                _ => luna_common::Span::new(luna_common::ids::FileId(0), 0, 0),
                            };
                            self.ctx.diagnostics.push(
                                luna_common::Diagnostic::error(
                                    "a type with raw storage anchors cannot implement Copy in RAW-STORAGE-ANCHOR-v1"
                                )
                                .with_code(DiagnosticCode::RawStorageAnchorViolation)
                                .with_span(span),
                            );
                        }
                    }
                    if let Some(&self_ty) = self.ctx.tables.symbol_types.get(&self_ty_sym) {
                        if self.ctx.needs_drop(self_ty) {
                            for &decl_id in decl_ids {
                                let mut span = luna_common::Span::new(luna_common::ids::FileId(0), 0, 0);
                                if let luna_ast::Decl::Impl { self_type, .. } = &self.arena.decls[decl_id.0 as usize] {
                                    if let luna_ast::Type::Named { segments, .. } = &self.arena.types[self_type.0 as usize] {
                                        if let Some(last) = segments.last() {
                                            span = *last;
                                        }
                                    }
                                }
                                self.ctx.diagnostics.push(
                                    luna_common::Diagnostic::error(
                                        "E_COPY_DROP_CONFLICT: A composite type cannot satisfy or derive Copy if any transitive field satisfies Drop"
                                    )
                                    .with_code(DiagnosticCode::CopyDropConflict)
                                    .with_span(span)
                                );
                            }
                        }
                    }
                }
            }
        }

        // Check monomorphized instantiated types
        for ty_id in self.ctx.types.get_all_types() {
            let ty = self.ctx.types.get(ty_id).clone();
            if let crate::ty::SemanticType::Struct(sym_id, args, _) | crate::ty::SemanticType::Enum(sym_id, args, _) = ty {
                if args.is_empty() {
                    // Non-generic types are already checked above
                    continue;
                }

                let impl_key = crate::semantic_tables::ImplKey {
                    trait_id: Some(copy_trait),
                    self_type_def: sym_id.into(),
                };
                
                if self.ctx.tables.trait_impls.contains_key(&impl_key) {
                    if self.ctx.needs_drop(ty_id) {
                        let mut span = luna_common::Span::new(luna_common::ids::FileId(0), 0, 0);
                        let sym = self.ctx.symbol_table.get_symbol(sym_id);
                        span = sym.span;
                        
                        // Prevent duplicate errors for the same concrete type
                        let diag = luna_common::Diagnostic::error(
                            "E_COPY_DROP_CONFLICT: A composite type cannot satisfy or derive Copy if any transitive field satisfies Drop (in generic instantiation)"
                        )
                        .with_code(DiagnosticCode::CopyDropConflict)
                        .with_span(span);
                        
                        if !self.ctx.diagnostics.contains(&diag) {
                            self.ctx.diagnostics.push(diag);
                        }
                    }
                }
            }
        }
    }

    fn check_recursive_types(&mut self, items: &[Item]) {
        let mut visiting = std::collections::HashSet::new();
        let mut visited = std::collections::HashSet::new();
        self.check_recursive_types_inner(items, &mut visiting, &mut visited);
    }

    fn check_recursive_types_inner(
        &mut self,
        items: &[Item],
        visiting: &mut std::collections::HashSet<crate::SymbolId>,
        visited: &mut std::collections::HashSet<crate::SymbolId>,
    ) {
        for item in items {
            if let Item::Decl(decl_id) = item {
                let decl = &self.arena.decls[decl_id.0 as usize];
                if let Decl::Module { items: inner_decls, .. } = decl {
                    let inner_items: Vec<Item> = inner_decls.iter().map(|&d| Item::Decl(d)).collect();
                    self.check_recursive_types_inner(&inner_items, visiting, visited);
                } else if let Some(&sym_id) = self.ctx.tables.decl_symbols.get(decl_id) {
                    if let Some(ty_id) = self.ctx.tables.symbol_types.get(&sym_id).copied() {
                        self.check_recursive_type(ty_id, *decl_id, visiting, visited);
                    }
                }
            }
        }
    }

    fn check_recursive_type(
        &mut self,
        ty_id: SemanticTypeId,
        original_decl: luna_ast::DeclId,
        visiting: &mut std::collections::HashSet<crate::SymbolId>,
        visited: &mut std::collections::HashSet<crate::SymbolId>,
    ) {
        let ty = self.ctx.types.get(ty_id).clone();
        
        let sym_id_opt = match ty {
            SemanticType::Struct(sym_id, _, _) => Some(sym_id),
            SemanticType::Enum(sym_id, _, _) => Some(sym_id),
            _ => None,
        };

        if let Some(sym_id) = sym_id_opt {
            if visited.contains(&sym_id) {
                return;
            }
            if !visiting.insert(sym_id) {
                let span = match &self.arena.decls[original_decl.0 as usize] {
                    luna_ast::Decl::Struct { name, .. } => *name,
                    luna_ast::Decl::Enum { name, .. } => *name,
                    luna_ast::Decl::TypeAlias { name, .. } => *name,
                    _ => luna_common::Span::new(luna_common::ids::FileId(0), 0, 0),
                };
                self.ctx.diagnostics.push(
                    Diagnostic::error("Recursive type has infinite size".to_string())
                        .with_code(DiagnosticCode::InfiniteSizeRecursiveType)
                        .with_span(span)
                );
                return;
            }
        }

        match ty {
            SemanticType::Struct(sym_id, _, _) => {
                if let Some(&full_ty_id) = self.ctx.tables.symbol_types.get(&sym_id) {
                    if let SemanticType::Struct(_, _, full_fields) = self.ctx.types.get(full_ty_id).clone() {
                        for f in full_fields {
                            self.check_recursive_type(f, original_decl, visiting, visited);
                        }
                    }
                }
            }
            SemanticType::Enum(sym_id, _, _) => {
                if let Some(&full_ty_id) = self.ctx.tables.symbol_types.get(&sym_id) {
                    if let SemanticType::Enum(_, _, full_variants) = self.ctx.types.get(full_ty_id).clone() {
                        for v in full_variants {
                            self.check_recursive_type(v, original_decl, visiting, visited);
                        }
                    }
                }
            }
            SemanticType::Tuple(fields) => {
                for f in fields {
                    self.check_recursive_type(f, original_decl, visiting, visited);
                }
            }
            SemanticType::Array(inner, _) => {
                self.check_recursive_type(inner, original_decl, visiting, visited);
            }
            // Indirections do not contribute to infinite size
            SemanticType::Pointer(..) | SemanticType::Reference(..) | SemanticType::Function { .. } | SemanticType::Closure(..) => {}
            _ => {}
        }

        if let Some(sym_id) = sym_id_opt {
            visiting.remove(&sym_id);
            visited.insert(sym_id);
        }
    }

    /// Resolve a named trait using its complete logical path. Generic bounds
    /// must not depend on final-segment root aliases such as bare `Iterator`.
    fn resolve_type_path_symbol(
        &self,
        segments: &[luna_common::Span],
        starting_scopes: &[crate::ScopeId],
    ) -> Option<luna_common::ids::SymbolId> {
        if segments.is_empty() {
            return None;
        }
        for &start_scope in starting_scopes {
            let mut scope = start_scope;
            let mut resolved = None;
            let mut complete = true;
            for (index, segment) in segments.iter().enumerate() {
                let name = self.get_span_text(*segment);
                let symbol = if index == 0 {
                    self.ctx.symbol_table.lookup_with_ctxt(name, segment.ctxt, scope)
                } else {
                    self.ctx.symbol_table.lookup_exact_with_ctxt(name, segment.ctxt, scope)
                };
                let Some(symbol) = symbol else {
                    complete = false;
                    break;
                };
                resolved = Some(symbol);
                if index + 1 < segments.len() {
                    let Some(inner_scope) = self.ctx.symbol_table.symbols[symbol.0 as usize].inner_scope else {
                        complete = false;
                        break;
                    };
                    scope = inner_scope;
                }
            }
            if complete {
                return resolved;
            }
        }
        None
    }

    fn populate_decl_bounds(&mut self, declaration: luna_ast::DeclId,
        parameters: &[luna_ast::GenericParam], scope: crate::ScopeId) {
        let previous = self.current_scope;
        self.current_scope = scope;
        for (index, parameter) in parameters.iter().enumerate() {
            let Some(&symbol) = self.ctx.tables.generic_param_symbols.get(&(declaration, index)) else { continue; };
            let generic_type = self.ctx.types.intern(SemanticType::GenericParam(symbol));
            self.ctx.tables.symbol_types.insert(symbol, generic_type);
            for &bound_type in &parameter.bounds {
                let luna_ast::Type::Named { segments, generic_args, associated_bindings } =
                    self.arena.types[bound_type.0 as usize].clone() else {
                    self.ctx.diagnostics.push(Diagnostic::error("A generic trait bound must name a trait")
                        .with_code(DiagnosticCode::TypeMismatch).with_span(parameter.name));
                    continue;
                };
                let span = segments.last().copied().unwrap_or(parameter.name);
                let Some(trait_id) = self.resolve_type_path_symbol(&segments, &[scope, crate::ScopeId(0)]) else {
                    self.ctx.diagnostics.push(Diagnostic::error("Unresolved trait in generic bound")
                        .with_code(DiagnosticCode::UnresolvedSymbol).with_span(span));
                    continue;
                };
                if self.ctx.symbol_table.get_symbol(trait_id).kind != crate::SymbolKind::Trait {
                    self.ctx.diagnostics.push(Diagnostic::error("A generic trait bound must name a trait")
                        .with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    continue;
                }
                if !self.ctx.symbol_table.is_accessible(trait_id, scope, self.ctx.current_provider) {
                    self.ctx.diagnostics.push(Diagnostic::error("A generic bound cannot access this private trait")
                        .with_code(DiagnosticCode::PrivateSymbolAccess).with_span(span));
                    continue;
                }
                if generic_args.len() != self.ctx.tables.trait_generic_params.get(&trait_id).map_or(0, Vec::len) {
                    self.ctx.diagnostics.push(Diagnostic::error("Incorrect number of type arguments in generic trait bound")
                        .with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    continue;
                }
                let trait_args = generic_args.iter().map(|&arg| self.lower_type(arg)).collect();
                let bound = crate::semantic_tables::TraitBound { param: symbol, trait_id, trait_args };
                let bounds = self.ctx.tables.trait_bounds.entry(symbol).or_default();
                if !bounds.contains(&bound) { bounds.push(bound); }
                for binding in associated_bindings {
                    let name = self.get_span_text(binding.name).to_string();
                    let Some(&associated) = self.ctx.tables.assoc_type_names.get(&(trait_id, name)) else {
                        self.ctx.diagnostics.push(Diagnostic::error("Unknown associated type in generic bound")
                            .with_code(DiagnosticCode::NoAssociatedType).with_span(binding.name));
                        continue;
                    };
                    let target_type = self.lower_type(binding.ty);
                    let equality = (trait_id, associated, target_type);
                    let equalities = self.ctx.tables.assoc_type_bounds.entry(symbol).or_default();
                    if !equalities.contains(&equality) {
                        equalities.push(equality);
                        let projection = self.ctx.types.intern(SemanticType::Projection {
                            self_type: generic_type, trait_id, assoc_type: associated,
                        });
                        self.associated_type_obligations.push(AssociatedTypeEqObligation {
                            projection, target_ty: target_type, span: binding.name,
                        });
                    }
                }
            }
        }
        self.current_scope = previous;
    }

    fn populate_trait_bounds(&mut self, items: &[Item]) {
        for item in items {
            let Item::Decl(declaration) = item else { continue; };
            let declaration = *declaration;
            let scope = self.ctx.tables.decl_scopes.get(&declaration).copied().unwrap_or(crate::ScopeId(0));
            let decl = self.arena.decls[declaration.0 as usize].clone();
            let (parameters, children) = match decl {
                Decl::Function { generic_params, .. } | Decl::Struct { generic_params, .. }
                | Decl::Enum { generic_params, .. } | Decl::TypeAlias { generic_params, .. } =>
                    (generic_params, Vec::new()),
                Decl::Impl { generic_params, methods, .. } | Decl::Trait { generic_params, methods, .. } =>
                    (generic_params, methods),
                Decl::Module { items, .. } => (Vec::new(), items),
                Decl::Extern { func, .. } => (Vec::new(), vec![func]),
                _ => continue,
            };
            self.populate_decl_bounds(declaration, &parameters, scope);
            let children: Vec<_> = children.into_iter().map(Item::Decl).collect();
            self.populate_trait_bounds(&children);
        }
    }

    /// Reconcile the resolver's provisional index with the checked self head.
    /// Forward declarations and aliases may be unavailable during declaration
    /// collection. Every accepted impl must have the same final method index.
    fn register_checked_impl_methods(&mut self, key: crate::semantic_tables::ImplKey,
        declaration: luna_ast::DeclId, methods: &[luna_ast::DeclId]) {
        let symbols: Vec<_> = methods.iter().filter_map(|method|
            self.ctx.tables.decl_symbols.get(method).copied()).collect();
        for (old_key, declarations) in &mut self.ctx.tables.trait_impls {
            if *old_key != key { declarations.retain(|owner| *owner != declaration); }
        }
        for (old_key, old_methods) in &mut self.ctx.tables.impl_methods {
            if *old_key != key { old_methods.retain(|method| !symbols.contains(method)); }
        }
        let declarations = self.ctx.tables.trait_impls.entry(key.clone()).or_default();
        if !declarations.contains(&declaration) { declarations.push(declaration); }
        let indexed = self.ctx.tables.impl_methods.entry(key.clone()).or_default();
        for &method in &symbols {
            if !indexed.contains(&method) { indexed.push(method); }
            self.ctx.tables.method_impls.insert(method, key.clone());
            self.ctx.tables.method_sym_to_impl_decl.insert(method, declaration);
        }
        for &method in methods { self.ctx.tables.method_to_impl_decl.insert(method, declaration); }
    }

    fn populate_signatures_pass1(&mut self, items: &[Item]) {
        self.populate_types(items);
        self.populate_impls(items);
    }

    fn populate_types(&mut self, items: &[Item]) {
        // Pass 1: Declare all struct/enum/type types
        for item in items {
            if let Item::Decl(decl_id) = item {
                let decl = &self.arena.decls[decl_id.0 as usize];
                let is_nominal = matches!(decl, Decl::Struct { .. } | Decl::Enum { .. });
                if is_nominal {
                    if self.populated_nominal_types.contains(decl_id)
                        || !self.populating_nominal_types.insert(*decl_id)
                    {
                        continue;
                    }
                }
                match decl {
                    Decl::Struct { fields, .. } => {
                        let prev_scope = self.current_scope;
                        let sym_id_opt = self.ctx.tables.decl_symbols.get(decl_id).copied();
                        if let Some(sym_id) = sym_id_opt {
                            if let Some(inner) = self.ctx.symbol_table.get_symbol(sym_id).inner_scope {
                                self.current_scope = inner;
                            }
                            let dummy_ty = self.ctx.types.intern(SemanticType::Struct(sym_id, vec![], vec![]));
                            self.ctx.tables.symbol_types.insert(sym_id, dummy_ty);
                        }
                        let mut field_tys = Vec::new();
                        let mut seen_fields = std::collections::HashSet::new();
                        for field in fields {
                            let field_name = self.get_span_text(field.name);
                            if !seen_fields.insert(field_name) {
                                self.ctx.diagnostics.push(Diagnostic::error(format!("Duplicate field `{}` in struct", field_name)).with_code(DiagnosticCode::DuplicateDefinition).with_span(field.name));
                            }
                            let f_ty = self.lower_type(field.ty);
                            if self.ctx.types.is_unsized(f_ty) {
                                self.ctx.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "E_UNSIZED_TYPE_IN_VALUE_POSITION: Field `{}` has unsized type `{:?}` which cannot be stored directly in a struct without indirection",
                                        field_name,
                                        self.ctx.types.get(f_ty)
                                    ))
                                    .with_code(DiagnosticCode::TypeMismatch)
                                    .with_span(field.name)
                                );
                            }
                            field_tys.push(f_ty);
                        }
                        if let Some(sym_id) = sym_id_opt {
                            let struct_ty = self.ctx.types.intern(SemanticType::Struct(sym_id, Vec::new(), field_tys.clone()));
                            self.ctx.tables.symbol_types.insert(sym_id, struct_ty);

                            // Field type admissibility gate for struct lifetime contracts:
                            // life(field) only admits reference types (&T or &rw T). Non-reference types reject with E2016.
                            if let Some(resolved_contract) = self.ctx.tables.resolved_type_lifetime_contracts.get(&sym_id).cloned() {
                                let struct_name = self.ctx.symbol_table.get_symbol(sym_id).name.clone();
                                let mut has_admissibility_error = false;
                                for constraint in &resolved_contract.constraints {
                                    if let crate::ResolvedTypeLifetimeSubject::Field(field_sym) = constraint.longer {
                                        if let Some(field_syms) = self.ctx.tables.struct_fields.get(&sym_id) {
                                            if let Some(idx) = field_syms.iter().position(|&s| s == field_sym) {
                                                if let Some(&fty) = field_tys.get(idx) {
                                                    if !matches!(self.ctx.types.get(fty), SemanticType::Reference(..)) {
                                                        let field_name = self.ctx.symbol_table.get_symbol(field_sym).name.clone();
                                                        self.ctx.diagnostics.push(
                                                            Diagnostic::error(format!(
                                                                "field '{}' in struct '{}' has non-reference type, but struct lifetime contracts only admit reference fields (&T or &rw T)",
                                                                field_name, struct_name
                                                            ))
                                                            .with_code(DiagnosticCode::LifetimeConstraintViolation)
                                                            .with_span(constraint.span),
                                                        );
                                                        has_admissibility_error = true;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if has_admissibility_error {
                                    self.ctx.tables.type_lifetime_contracts.remove(&sym_id);
                                }
                            }
                            if let Some(anchor_contract) = self.ctx.tables.resolved_raw_storage_anchor_contracts.get(&sym_id).cloned() {
                                let mut invalid = false;
                                for field_sym in anchor_contract.fields {
                                    let Some(field_index) = self.ctx.tables.struct_fields.get(&sym_id)
                                        .and_then(|fields| fields.iter().position(|field| *field == field_sym)) else { continue; };
                                    let is_direct_raw_pointer = field_tys.get(field_index).is_some_and(|ty| {
                                        matches!(self.ctx.types.get(*ty), SemanticType::Pointer(..))
                                    });
                                    if !is_direct_raw_pointer {
                                        let field_name = self.ctx.symbol_table.get_symbol(field_sym).name.clone();
                                        self.ctx.diagnostics.push(
                                            Diagnostic::error(format!(
                                                "anchored field '{}' in struct '{}' must have a direct raw pointer type (*T or *rw T)",
                                                field_name, self.ctx.symbol_table.get_symbol(sym_id).name
                                            ))
                                            .with_code(DiagnosticCode::RawStorageAnchorViolation)
                                            .with_span(self.ctx.symbol_table.get_symbol(field_sym).span),
                                        );
                                        invalid = true;
                                    }
                                }
                                if invalid {
                                    self.ctx.tables.raw_storage_anchor_contracts.remove(&sym_id);
                                    self.ctx.tables.resolved_raw_storage_anchor_contracts.remove(&sym_id);
                                }
                            }
                        }
                        self.current_scope = prev_scope;
                    }
                    Decl::Enum { generic_params, variants, .. } => {
                        let prev_scope = self.current_scope;
                        let mut enum_args = Vec::new();
                        let sym_id_opt = self.ctx.tables.decl_symbols.get(decl_id).copied();
                        if let Some(sym_id) = sym_id_opt {
                            if let Some(inner) = self.ctx.symbol_table.get_symbol(sym_id).inner_scope {
                                self.current_scope = inner;
                            }
                            for (gp_idx, _) in generic_params.iter().enumerate() {
                                if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(*decl_id, gp_idx)) {
                                    enum_args.push(self.ctx.types.intern(SemanticType::GenericParam(*gp_sym)));
                                }
                            }

                            let dummy_ty = self.ctx.types.intern(SemanticType::Enum(sym_id, enum_args.clone(), vec![]));
                            self.ctx.tables.symbol_types.insert(sym_id, dummy_ty);
                        }
                        let mut variant_tys = Vec::new();
                        let mut seen_variants = std::collections::HashSet::new();
                        for variant in variants {
                            let variant_name = self.get_span_text(variant.name);
                            if !seen_variants.insert(variant_name) {
                                self.ctx.diagnostics.push(Diagnostic::error(format!("Duplicate variant `{}` in enum", variant_name)).with_code(DiagnosticCode::DuplicateDefinition).with_span(variant.name));
                            }
                            // Each variant's type is based on its fields
                            let mut field_tys = Vec::new();
                            for field_id in &variant.fields {
                                if let Decl::Param { ty, .. } = &self.arena.decls[field_id.0 as usize] {
                                    if let Some(t) = ty {
                                        field_tys.push(self.lower_type(*t));
                                    }
                                }
                            }
                            // For enums with no fields on a variant, push a void type
                            if field_tys.is_empty() {
                                variant_tys.push(self.ctx.types.intern(SemanticType::Void));
                            } else if field_tys.len() == 1 {
                                variant_tys.push(field_tys[0]);
                            } else {
                                variant_tys.push(self.ctx.types.intern(SemanticType::Tuple(field_tys)));
                            }
                        }
                        if let Some(sym_id) = sym_id_opt {
                            let enum_ty = self.ctx.types.intern(SemanticType::Enum(sym_id, enum_args, variant_tys));
                            self.ctx.tables.symbol_types.insert(sym_id, enum_ty);
                        }
                        self.current_scope = prev_scope;
                    }
                    Decl::TypeAlias { aliased_type, generic_params, .. } => {
                        let sym_id_opt = self.ctx.tables.decl_symbols.get(decl_id).copied();
                        if let Some(sym_id) = sym_id_opt {
                            if let Some(aliased) = aliased_type {
                                let prev_scope = self.current_scope;
                                if let Some(inner) = self.ctx.symbol_table.get_symbol(sym_id).inner_scope {
                                    self.current_scope = inner;
                                }
                                for (gp_idx, _) in generic_params.iter().enumerate() {
                                    if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(*decl_id, gp_idx)) {
                                        let gp_ty = self.ctx.types.intern(SemanticType::GenericParam(*gp_sym));
                                        self.ctx.tables.symbol_types.insert(*gp_sym, gp_ty);
                                    }
                                }
                                let sem_ty = self.lower_type(*aliased);
                                self.ctx.tables.symbol_types.insert(sym_id, sem_ty);
                                self.current_scope = prev_scope;
                            }
                        }
                    }
                    Decl::Module { items: inner_decls, .. } => {
                        let prev_scope = self.current_scope;
                        if let Some(&sym_id) = self.ctx.tables.decl_symbols.get(decl_id) {
                            if let Some(inner) = self.ctx.symbol_table.symbols[sym_id.0 as usize].inner_scope {
                                self.current_scope = inner;
                            }
                        }
                        let inner_items: Vec<Item> = inner_decls.iter().map(|&d| Item::Decl(d)).collect();
                        self.populate_types(&inner_items);
                        self.current_scope = prev_scope;
                    }
                    _ => {}
                }
                if is_nominal {
                    self.populating_nominal_types.remove(decl_id);
                    self.populated_nominal_types.insert(*decl_id);
                }
            }
        }
    }

    fn impl_trait_path_label(&self, trait_type_id: luna_ast::TypeId) -> (String, luna_common::Span) {
        match &self.arena.types[trait_type_id.0 as usize] {
            luna_ast::Type::Named { segments, .. } => {
                let label = segments
                    .iter()
                    .map(|segment| self.get_span_text(*segment))
                    .collect::<Vec<_>>()
                    .join("::");
                let span = match (segments.first(), segments.last()) {
                    (Some(first), Some(last)) => luna_common::Span {
                        file_id: first.file_id,
                        start: first.start,
                        end: last.end,
                        ctxt: first.ctxt,
                    },
                    _ => luna_common::Span::default(),
                };
                (label, span)
            }
            other => (format!("{other:?}"), luna_common::Span::default()),
        }
    }

    fn report_unresolved_impl_trait(&mut self, trait_type_id: luna_ast::TypeId) {
        let (path, span) = self.impl_trait_path_label(trait_type_id);
        self.ctx.diagnostics.push(
            Diagnostic::error(format!("cannot resolve trait path `{path}` in this scope"))
                .with_code(DiagnosticCode::UnresolvedSymbol)
                .with_span(span),
        );
    }

    fn report_non_trait_impl_path(&mut self, trait_type_id: luna_ast::TypeId) {
        let (path, span) = self.impl_trait_path_label(trait_type_id);
        self.ctx.diagnostics.push(
            Diagnostic::error(format!("impl path `{path}` does not name a trait"))
                .with_code(DiagnosticCode::UnresolvedTraitImpl)
                .with_span(span),
        );
    }

    fn populate_impls(&mut self, items: &[Item]) {
        // Pass 1.8: Register Trait Impls and Impl Associated Types
        for item in items {
            if let Item::Decl(decl_id) = item {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Impl { trait_type, self_type, associated_types, generic_params, methods, .. } => {
                    let impl_scope = *self.ctx.tables.decl_scopes.get(decl_id).unwrap_or(&self.current_scope);
                    let self_sym_opt = self.ctx.tables.type_symbols.get(self_type).copied().or_else(|| {
                        let self_ast_ty = &self.arena.types[self_type.0 as usize];
                        if let luna_ast::Type::Named { segments, .. } = self_ast_ty {
                            if segments.len() == 1 {
                                let s = &segments[0];
                                let name = self.get_span_text(*s);
                                self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, impl_scope)
                                    .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, crate::ScopeId(0)))
                            } else {
                                let mut scope = impl_scope;
                                let mut res = None;
                                for (i, seg) in segments.iter().enumerate() {
                                    let seg_name = self.get_span_text(*seg);
                                    let sym_id = if i == 0 {
                                        self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                            .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                                    } else {
                                        self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                                    };
                                    if let Some(id) = sym_id {
                                        res = Some(id);
                                        if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                            scope = inner;
                                        } else if i < segments.len() - 1 {
                                            res = None;
                                            break;
                                        }
                                    } else {
                                        res = None;
                                        break;
                                    }
                                }
                                res
                            }
                        } else {
                            None
                        }
                    });
                    let trait_sym_opt = trait_type.and_then(|trait_type_id| {
                        self.ctx.tables.type_symbols.get(&trait_type_id).copied().or_else(|| {
                            let trait_ast_ty = &self.arena.types[trait_type_id.0 as usize];
                            if let luna_ast::Type::Named { segments, .. } = trait_ast_ty {
                                if segments.len() == 1 {
                                    let s = &segments[0];
                                    let name = self.get_span_text(*s);
                                    self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, impl_scope)
                                        .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, crate::ScopeId(0)))
                                } else {
                                    let mut scope = impl_scope;
                                    let mut res = None;
                                    for (i, seg) in segments.iter().enumerate() {
                                        let seg_name = self.get_span_text(*seg);
                                        let sym_id = if i == 0 {
                                            self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                                .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                                        } else {
                                            self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                                        };
                                        if let Some(id) = sym_id {
                                            res = Some(id);
                                            if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                                scope = inner;
                                            } else if i < segments.len() - 1 {
                                                res = None;
                                                break;
                                            }
                                        } else {
                                            res = None;
                                            break;
                                        }
                                    }
                                    res
                                }
                            } else {
                                None
                            }
                        })
                    });

                    // An explicitly written `impl Trait for Type` must never
                    // degrade into an inherent impl when the trait path did
                    // not resolve. In particular, language-item identity is
                    // not a lexical root binding: `Drop` is not rewritten to
                    // `std::Drop` here.
                    if let Some(trait_type_id) = trait_type {
                        let Some(trait_sym) = trait_sym_opt else {
                            self.report_unresolved_impl_trait(*trait_type_id);
                            continue;
                        };
                        if self.ctx.symbol_table.get_symbol(trait_sym).kind != crate::SymbolKind::Trait {
                            self.report_non_trait_impl_path(*trait_type_id);
                            continue;
                        }
                    }

                    let span = match &self.arena.types[self_type.0 as usize] {
                        luna_ast::Type::Named { segments, .. } => *segments.last().unwrap(),
                        luna_ast::Type::Lifetime(s) => *s,
                        luna_ast::Type::MacroCall { span, .. } => *span,
                        _ => {
                            if let Some(t_ty) = trait_type {
                                if let luna_ast::Type::Named { segments, .. } = &self.arena.types[t_ty.0 as usize] {
                                    *segments.last().unwrap()
                                } else {
                                    luna_common::Span::default()
                                }
                            } else {
                                luna_common::Span::default()
                            }
                        }
                    };

                    let prev_scope = self.current_scope;
                    let impl_scope = *self.ctx.tables.decl_scopes.get(decl_id).unwrap_or(&crate::ScopeId(0));
                    self.current_scope = impl_scope;
                    let self_sem_ty = self.lower_type(*self_type);
                    self.current_scope = prev_scope;

                    // Check orphan rule (for both trait impls and inherent impls)
                    let locality_res = self.ctx.check_impl_locality(trait_sym_opt, self_sem_ty, span);

                    if locality_res.is_ok() {
                        let header_parameters = (0..generic_params.len()).filter_map(|idx|
                            self.ctx.tables.generic_param_symbols.get(&(*decl_id, idx)).copied()
                        ).collect();
                        self.ctx.tables.checked_impl_headers.insert(*decl_id,
                            crate::semantic_tables::CheckedImplHeader {
                                self_type: self_sem_ty,
                                generic_params: header_parameters,
                            });
                        if let Some(trait_sym) = trait_sym_opt {
                            let mut gp_syms = Vec::new();
                            for idx in 0..generic_params.len() {
                                if let Some(&gp_sym) = self.ctx.tables.generic_param_symbols.get(&(*decl_id, idx)) {
                                    gp_syms.push(gp_sym);
                                }
                            }

                            let trait_args: Vec<SemanticTypeId> = if let Some(t_ty_id) = trait_type {
                                let t_ast_ty = &self.arena.types[t_ty_id.0 as usize];
                                if let luna_ast::Type::Named { generic_args, .. } = t_ast_ty {
                                    let prev_scope = self.current_scope;
                                    self.current_scope = impl_scope;
                                    let args = generic_args.iter().map(|arg| self.lower_type(*arg)).collect();
                                    self.current_scope = prev_scope;
                                    args
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            };

                            // Check coherence
                            let coherence_res = self.ctx.check_impl_coherence(trait_sym, self_sem_ty, &gp_syms, &trait_args, span);

                            if coherence_res.is_ok() {
                                self.ctx.tables.trait_impl_entries.push(crate::semantic_tables::TraitImplEntry {
                                    decl_id: Some(*decl_id),
                                    trait_id: trait_sym,
                                    self_type: self_sem_ty,
                                    generic_params: gp_syms.clone(),
                                    trait_args,
                                });

                                let self_key_opt: Option<crate::semantic_tables::ImplSelfTypeKey> = match self.ctx.types.get(self_sem_ty) {
                                    SemanticType::Primitive(b) => Some(crate::semantic_tables::ImplSelfTypeKey::Primitive(*b)),
                                    SemanticType::Struct(s, ..) | SemanticType::Enum(s, ..) => Some(crate::semantic_tables::ImplSelfTypeKey::Nominal(*s)),
                                    SemanticType::Slice(_) => Some(crate::semantic_tables::ImplSelfTypeKey::Slice),
                                    _ => self_sym_opt.map(crate::semantic_tables::ImplSelfTypeKey::Nominal),
                                };

                                if let Some(self_key) = self_key_opt {
                                    let key = crate::semantic_tables::ImplKey {
                                        trait_id: Some(trait_sym),
                                        self_type_def: self_key,
                                    };
                                    self.register_checked_impl_methods(key.clone(), *decl_id, methods);
                                    let entry = self.ctx.tables.trait_impls
                                        .entry(key.clone())
                                        .or_insert_with(Vec::new);
                                    if !entry.contains(decl_id) {
                                        entry.push(*decl_id);
                                    }

                                    self.current_scope = impl_scope;
                                    self.ctx.tables.impl_self_types.insert(key.clone(), self_sem_ty);
                                    self.ctx.tables.impl_generic_params.insert(key.clone(), gp_syms);

                                    for assoc_id in associated_types {
                                        let a_decl = &self.arena.decls[assoc_id.0 as usize];
                                        if let Decl::TypeAlias { name, aliased_type: Some(ty), .. } = a_decl {
                                            let a_name = self.get_span_text(*name);
                                            if let Some(&trait_assoc_sym) = self.ctx.tables.assoc_type_names.get(&(trait_sym, a_name.to_string())) {
                                                let initial_proj = self.ctx.types.intern(SemanticType::Projection {
                                                    self_type: self_sem_ty,
                                                    trait_id: trait_sym,
                                                    assoc_type: trait_assoc_sym,
                                                });
                                                self.ctx.tables.impl_associated_types.insert((key.clone(), trait_assoc_sym), initial_proj);
                                                self.ctx.tables.decl_associated_types.insert((*decl_id, trait_assoc_sym), initial_proj);
                                            }
                                            let concrete_ty = self.lower_type(*ty);
                                            if let Some(a_sym) = self.ctx.tables.decl_symbols.get(assoc_id).copied() {
                                                self.ctx.tables.symbol_types.insert(a_sym, concrete_ty);
                                            }
                                            if let Some(&trait_assoc_sym) = self.ctx.tables.assoc_type_names.get(&(trait_sym, a_name.to_string())) {
                                                self.ctx.tables.impl_associated_types.insert((key.clone(), trait_assoc_sym), concrete_ty);
                                                self.ctx.tables.decl_associated_types.insert((*decl_id, trait_assoc_sym), concrete_ty);
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            let self_key_opt: Option<crate::semantic_tables::ImplSelfTypeKey> = match self.ctx.types.get(self_sem_ty) {
                                SemanticType::Primitive(b) => Some(crate::semantic_tables::ImplSelfTypeKey::Primitive(*b)),
                                SemanticType::Struct(s, ..) | SemanticType::Enum(s, ..) => Some(crate::semantic_tables::ImplSelfTypeKey::Nominal(*s)),
                                SemanticType::Slice(_) => Some(crate::semantic_tables::ImplSelfTypeKey::Slice),
                                _ => self_sym_opt.map(crate::semantic_tables::ImplSelfTypeKey::Nominal),
                            };
                            if let Some(self_key) = self_key_opt {
                                let mut gp_syms = Vec::new();
                                for idx in 0..generic_params.len() {
                                    if let Some(&gp_sym) = self.ctx.tables.generic_param_symbols.get(&(*decl_id, idx)) {
                                        gp_syms.push(gp_sym);
                                    }
                                }
                                let key = crate::semantic_tables::ImplKey {
                                    trait_id: None,
                                    self_type_def: self_key,
                                };
                                self.register_checked_impl_methods(key.clone(), *decl_id, methods);
                                self.ctx.tables.impl_self_types.insert(key.clone(), self_sem_ty);
                                if !gp_syms.is_empty() {
                                    self.ctx.tables.impl_generic_params.insert(key.clone(), gp_syms);
                                }
                            }
                        }
                    }
                }
                    Decl::Module { items: inner_decls, .. } => {
                        let prev_scope = self.current_scope;
                        if let Some(&sym_id) = self.ctx.tables.decl_symbols.get(decl_id) {
                            if let Some(inner) = self.ctx.symbol_table.symbols[sym_id.0 as usize].inner_scope {
                                self.current_scope = inner;
                            }
                        }
                        let inner_items: Vec<Item> = inner_decls.iter().map(|&d| Item::Decl(d)).collect();
                        self.populate_impls(&inner_items);
                        self.current_scope = prev_scope;
                    }
                    _ => {}
                }
            }
        }
    }

    fn populate_signatures_pass2(&mut self, items: &[Item]) {
        // Pass 2: Functions and global variables
        for item in items {
            if let Item::Decl(decl_id) = item {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Function { params, return_type, is_async, is_unsafe, .. } => {
                        let prev_scope = self.current_scope;
                        let sym_id_opt = self.ctx.tables.decl_symbols.get(decl_id).copied();
                        if let Some(sym_id) = sym_id_opt {
                            if let Some(inner) = self.ctx.symbol_table.get_symbol(sym_id).inner_scope {
                                self.current_scope = inner;
                            }
                        }
                        let mut param_tys = Vec::new();
                        for param_id in params {
                            if let Decl::Param { ty, .. } = &self.arena.decls[param_id.0 as usize] {
                                let param_ty = if let Some(t) = ty { self.lower_type(*t) } else { self.ctx.types.new_inference_var() };
                                param_tys.push(param_ty);
                                if let Some(param_sym) = self.ctx.tables.decl_symbols.get(param_id) {
                                    self.ctx.tables.symbol_types.insert(*param_sym, param_ty);
                                }
                            }
                        }
                        let ret_ty = if let Some(r) = return_type { self.lower_type(*r) } else { self.ctx.types.intern(SemanticType::Void) };
                        let actual_ret_ty = if *is_async {
                            self.ctx.types.intern(SemanticType::Future(ret_ty))
                        } else {
                            ret_ty
                        };
                        let func_ty = self.ctx.types.intern(SemanticType::Function { params: param_tys, return_type: actual_ret_ty, is_unsafe: *is_unsafe || sym_id_opt.is_some_and(|symbol| matches!(self.ctx.symbol_table.get_symbol(symbol).kind, crate::SymbolKind::ExternFunction)) });
                        if let Some(sym_id) = sym_id_opt {
                            self.ctx.tables.symbol_types.insert(sym_id, func_ty);
                            self.register_callable_signature(sym_id);
                            self.ctx.tables.function_effects.entry(sym_id).or_insert_with(crate::effect::EffectSet::pure);
                            if *is_unsafe {
                                self.ctx.tables.unsafe_functions.insert(sym_id);
                            }
                        }
                        self.current_scope = prev_scope;
                    }
                    Decl::Var { type_annot, .. } => {
                        let sym_id_opt = self.ctx.tables.decl_symbols.get(decl_id).copied();
                        if let Some(sym_id) = sym_id_opt {
                            if !self.ctx.tables.symbol_types.contains_key(&sym_id) {
                                let mut ty = self.ctx.types.new_inference_var();
                                if let Some(annot) = type_annot {
                                    ty = self.lower_type(*annot);
                                }
                                self.ctx.tables.symbol_types.insert(sym_id, ty);
                            }
                        }
                    }
                    Decl::Trait { methods, .. } => {
                        let prev_scope = self.current_scope;
                        if let Some(&scope) = self.ctx.tables.decl_scopes.get(decl_id) {
                            self.current_scope = scope;
                        }
                        let prev_self = self.current_self_type;
                        if let Some(&trait_sym) = self.ctx.tables.decl_symbols.get(decl_id) {
                            self.current_self_type = Some(self.ctx.types.intern(SemanticType::GenericParam(trait_sym)));
                        }
                        for method_id in methods {
                            let method_decl = &self.arena.decls[method_id.0 as usize];
                            if let Decl::Function { params, return_type, is_unsafe, is_async, .. } = method_decl {
                                let sym_id_opt = self.ctx.tables.decl_symbols.get(method_id).copied();
                                let mut param_tys = Vec::new();
                                for param_id in params {
                                    if let Decl::Param { ty, .. } = &self.arena.decls[param_id.0 as usize] {
                                        let param_ty = if let Some(t) = ty { self.lower_type(*t) } else { self.ctx.types.new_inference_var() };
                                        param_tys.push(param_ty);
                                        if let Some(param_sym) = self.ctx.tables.decl_symbols.get(param_id) {
                                            self.ctx.tables.symbol_types.insert(*param_sym, param_ty);
                                        }
                                    }
                                }
                                let ret_ty = if let Some(r) = return_type { self.lower_type(*r) } else { self.ctx.types.intern(SemanticType::Void) };
                                let actual_ret_ty = if *is_async {
                                    self.ctx.types.intern(SemanticType::Future(ret_ty))
                                } else {
                                    ret_ty
                                };
                                let func_ty = self.ctx.types.intern(SemanticType::Function { params: param_tys, return_type: actual_ret_ty, is_unsafe: *is_unsafe || sym_id_opt.is_some_and(|symbol| matches!(self.ctx.symbol_table.get_symbol(symbol).kind, crate::SymbolKind::ExternFunction)) });
                                if let Some(sym_id) = sym_id_opt {
                                    self.ctx.tables.symbol_types.insert(sym_id, func_ty);
                                    self.register_callable_signature(sym_id);
                                    if *is_unsafe {
                                        self.ctx.tables.unsafe_functions.insert(sym_id);
                                    }
                                }
                            }
                        }
                        self.current_self_type = prev_self;
                        self.current_scope = prev_scope;
                    }
                    Decl::Impl { generic_params, self_type, methods, .. } => {
                        if true {
                            let prev_scope = self.current_scope;
                            let impl_scope = *self.ctx.tables.decl_scopes.get(decl_id).unwrap_or(&crate::ScopeId(0));
                            self.current_scope = impl_scope;
                            let self_sem_ty = self.lower_type(*self_type);
                            let prev_self = self.current_self_type;
                            self.current_self_type = Some(self_sem_ty);
                            for method_id in methods {
                                let method_decl = &self.arena.decls[method_id.0 as usize];
                                if let Decl::Function { params, return_type, is_async, is_unsafe, .. } = method_decl {
                                    let sym_id_opt = self.ctx.tables.decl_symbols.get(method_id).copied();
                                    // Enter method scope to see method-level generic params
                                    if let Some(sym_id) = sym_id_opt {
                                        if let Some(inner) = self.ctx.symbol_table.get_symbol(sym_id).inner_scope {
                                            self.current_scope = inner;
                                        }
                                    }
                                    let mut param_tys = Vec::new();
                                    for param_id in params {
                                        if let Decl::Param { ty, is_self, .. } = &self.arena.decls[param_id.0 as usize] {
                                            let param_ty = if let Some(t) = ty {
                                                self.lower_type(*t)
                                            } else if *is_self {
                                                self_sem_ty
                                            } else {
                                                self.ctx.types.new_inference_var()
                                            };
                                            param_tys.push(param_ty);
                                            if let Some(param_sym) = self.ctx.tables.decl_symbols.get(param_id) {
                                                self.ctx.tables.symbol_types.insert(*param_sym, param_ty);
                                            }
                                        }
                                    }
                                    let ret_ty = if let Some(r) = return_type { self.lower_type(*r) } else { self.ctx.types.intern(SemanticType::Void) };
                                    let actual_ret_ty = if *is_async {
                                        self.ctx.types.intern(SemanticType::Future(ret_ty))
                                    } else {
                                        ret_ty
                                    };
                                    let func_ty = self.ctx.types.intern(SemanticType::Function { params: param_tys, return_type: actual_ret_ty, is_unsafe: *is_unsafe || sym_id_opt.is_some_and(|symbol| matches!(self.ctx.symbol_table.get_symbol(symbol).kind, crate::SymbolKind::ExternFunction)) });
                                    if let Some(sym_id) = sym_id_opt {
                                        self.ctx.tables.symbol_types.insert(sym_id, func_ty);
                                        self.register_callable_signature(sym_id);
                                        if *is_unsafe {
                                            self.ctx.tables.unsafe_functions.insert(sym_id);
                                        }
                                    }
                                }
                            }
                            self.current_self_type = prev_self;
                            self.current_scope = prev_scope;
                        }
                    }
                    Decl::Module { items: inner_decls, .. } => {
                        let prev_scope = self.current_scope;
                        if let Some(&sym_id) = self.ctx.tables.decl_symbols.get(decl_id) {
                            if let Some(inner) = self.ctx.symbol_table.symbols[sym_id.0 as usize].inner_scope {
                                self.current_scope = inner;
                            }
                        }
                        let inner_items: Vec<Item> = inner_decls.iter().map(|&d| Item::Decl(d)).collect();
                        self.populate_signatures_pass2(&inner_items);
                        self.current_scope = prev_scope;
                    }
                    Decl::Extern { func, .. } => {
                        let inner_items = [Item::Decl(*func)];
                        self.populate_signatures_pass2(&inner_items);
                    }
                    _ => {}
                }
            }
        }
    }

    fn collect_const_inits(
        &self,
        items: &[Item],
        out: &mut Vec<(luna_ast::DeclId, luna_ast::ExprId, luna_common::Span)>,
    ) {
        for item in items {
            if let Item::Decl(decl_id) = item {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Var { name, is_const: true, initializer: Some(init), .. } => {
                        out.push((*decl_id, *init, *name));
                    }
                    Decl::Module { items: inner_decls, .. } => {
                        let inner_items: Vec<Item> = inner_decls.iter().map(|&d| Item::Decl(d)).collect();
                        self.collect_const_inits(&inner_items, out);
                    }
                    _ => {}
                }
            }
        }
    }

    fn validate_memory_intrinsics(&mut self) {
        use crate::lang_item::LangItem;
        use crate::ty::Mutability;
        let hooks: Vec<_> = self.ctx.lang_items.iter()
            .filter(|(item, _)| item.is_memory_intrinsic()).collect();
        for (item, symbol) in hooks {
            let declaration = self.ctx.tables.symbol_decls.get(&symbol)
                .and_then(|id| self.arena.decls.get(id.0 as usize));
            let mut valid = false;
            if let Some(Decl::Function { generic_params, params: declarations, body,
                is_unsafe, is_intrinsic, is_async, is_comptime, is_variadic, .. }) = declaration {
                if *is_unsafe && *is_intrinsic && !*is_async && !*is_comptime
                    && !*is_variadic && body.is_none() && generic_params.len() == 1
                    && generic_params[0].kind == luna_ast::GenericParamKind::Type
                    && generic_params[0].bounds.is_empty() {
                    if let Some(&function_ty) = self.ctx.tables.symbol_types.get(&symbol) {
                        if let SemanticType::Function { params, return_type, is_unsafe } = self.ctx.types.get(function_ty) {
                            let binder = self.ctx.tables.symbol_decls.get(&symbol)
                                .and_then(|id| self.ctx.tables.generic_param_symbols.get(&(*id, 0)));
                            let element = params.first().and_then(|ty| match self.ctx.types.get(*ty) {
                                SemanticType::Pointer(mutable, element) => {
                                    let expected = if item == LangItem::SliceFromRawParts {
                                        Mutability::Immutable
                                    } else { Mutability::Mutable };
                                    if *mutable == expected && matches!(self.ctx.types.get(*element),
                                        SemanticType::GenericParam(id) if Some(id) == binder) { Some(*element) }
                                    else { None }
                                }, _ => None,
                            });
                            if let Some(element) = element {
                                valid = if item == LangItem::DropInPlace {
                                    params.len() == 1 && matches!(self.ctx.types.get(*return_type), SemanticType::Void)
                                } else {
                                    let expected = if item == LangItem::SliceFromRawParts {
                                        Mutability::Immutable
                                    } else { Mutability::Mutable };
                                    params.len() == 2
                                        && matches!(self.ctx.types.get(params[1]), SemanticType::Primitive(BuiltinType::U64))
                                        && matches!(self.ctx.types.get(*return_type),
                                            SemanticType::Reference(_, mutable, slice) if *mutable == expected
                                                && matches!(self.ctx.types.get(*slice), SemanticType::Slice(inner) if *inner == element))
                                };
                            valid &= *is_unsafe && declarations.iter().all(|id| matches!(self.arena.decls.get(id.0 as usize),
                                    Some(Decl::Param { is_variadic: false, is_self: false, .. })));
                            }
                        }
                    }
                }
            }
            if !valid {
                self.ctx.diagnostics.push(Diagnostic::error(format!(
                    "invalid declaration for memory intrinsic `{}`: expected an unsafe, bodyless, non-variadic intrinsic with the canonical generic signature", item.name()))
                    .with_code(DiagnosticCode::InvalidAnnotation)
                    .with_span(self.ctx.symbol_table.get_symbol(symbol).span));
            }
        }
    }

    fn populate_signatures(&mut self, items: &[Item]) {
        self.populate_signatures_pass1(items);
        self.populate_trait_bounds(items);
        self.populate_signatures_pass2(items);
        self.validate_memory_intrinsics();

        // Pass 1.5: Early evaluation of top-level constants
        let mut const_inits = Vec::new();
        self.collect_const_inits(items, &mut const_inits);

        // Validate const admission rules
        let mut valid_const_inits = Vec::new();
        for (decl_id, init, name_span) in const_inits {
            if let Err(msg) = crate::const_eval::is_const_evaluable(self.arena, self.ctx, self.source_manager, init) {
                self.ctx.diagnostics.push(Diagnostic::error(format!("cannot evaluate constant in comptime: {}", msg)).with_code(DiagnosticCode::ComptimeEvaluationFailed).with_span(name_span));
            } else {
                valid_const_inits.push((decl_id, init, name_span));
            }
        }

        // Sort constants in topological dependency order and detect cycles
        match crate::const_eval::sort_constants_topological(&valid_const_inits, self.arena, self.ctx, self.source_manager) {
            Ok(ordered) => {
                for (decl_id, init) in ordered {
                    let decl = &self.arena.decls[decl_id.0 as usize];
                    let (type_annot, sym_id_opt) = match decl {
                        Decl::Var { type_annot, .. } => (*type_annot, self.ctx.tables.decl_symbols.get(&decl_id).copied()),
                        _ => (None, None),
                    };
                    // Early evaluation walks a flat dependency order; recover
                    // the declaration's lexical scope for names, privacy and
                    // type annotations rather than using the outer/root scope.
                    let previous_scope = self.current_scope;
                    if let Some(symbol) = sym_id_opt {
                        self.current_scope = self.ctx.symbol_table.get_symbol(symbol).scope;
                    }
                    let init_ty = if let Some(annot) = type_annot {
                        let expected = self.lower_type(annot);
                        self.typecheck_expr_expected(&init, expected)
                    } else { self.typecheck_expr(&init) };
                    if matches!(self.ctx.types.get(init_ty), SemanticType::Error) {
                        if let Some(symbol) = sym_id_opt { self.ctx.tables.symbol_types.insert(symbol, init_ty); }
                        self.current_scope = previous_scope;
                        continue;
                    }
                    let final_ty = if let Some(annot) = type_annot {
                        let annot_ty = self.lower_type(annot);
                        if let Err(e) = self.unify(annot_ty, init_ty) {
                            let span = self.get_expr_span_for_diag(&init).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                        }
                        annot_ty
                    } else {
                        init_ty
                    };
                    if let Some(sym_id) = sym_id_opt {
                        self.ctx.tables.symbol_types.insert(sym_id, final_ty);
                    }
                    match self.eval_comptime_expr(init) {
                        Ok(val) => {
                            self.ctx.comptime_values.insert(init, val.clone());
                            if let Some(sym_id) = sym_id_opt {
                                self.ctx.const_values.insert(sym_id, val);
                            }
                        }
                        Err(e) => {
                            let span = self.get_expr_span_for_diag(&init).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.report_comptime_error(e, span, "cannot evaluate constant in comptime");
                        }
                    }
                    self.current_scope = previous_scope;
                }
            }
            Err(cycle_err) => {
                self.ctx.diagnostics.push(Diagnostic::error(format!(
                    "cycle detected in constant evaluation: {}",
                    cycle_err.cycle.join(" -> ")
                )).with_code(DiagnosticCode::ComptimeEvaluationFailed).with_span(cycle_err.span));
            }
        }

        // Pass 3: Check Drop Impls & Trait Impls
        for item in items {
            if let Item::Decl(decl_id) = item {
                let decl = &self.arena.decls[decl_id.0 as usize];
                if let Decl::Impl { trait_type, self_type, methods, associated_types, .. } = decl {
                    let impl_scope = *self.ctx.tables.decl_scopes.get(decl_id).unwrap_or(&self.current_scope);
                    let self_sym_opt = self.ctx.tables.type_symbols.get(self_type).copied().or_else(|| {
                        let self_ast_ty = &self.arena.types[self_type.0 as usize];
                        if let luna_ast::Type::Named { segments, .. } = self_ast_ty {
                            if segments.len() == 1 {
                                let s = &segments[0];
                                let name = self.get_span_text(*s);
                                self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, impl_scope)
                                    .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, crate::ScopeId(0)))
                            } else {
                                let mut scope = impl_scope;
                                let mut res = None;
                                for (i, seg) in segments.iter().enumerate() {
                                    let seg_name = self.get_span_text(*seg);
                                    let sym_id = if i == 0 {
                                        self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                            .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                                    } else {
                                        self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                                    };
                                    if let Some(id) = sym_id {
                                        res = Some(id);
                                        if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                            scope = inner;
                                        } else if i < segments.len() - 1 {
                                            res = None;
                                            break;
                                        }
                                    } else {
                                        res = None;
                                        break;
                                    }
                                }
                                res
                            }
                        } else {
                            None
                        }
                    });

                    if let Some(trait_ty_id) = trait_type {
                        let trait_sym_opt = self.ctx.tables.type_symbols.get(trait_ty_id).copied().or_else(|| {
                            let trait_ast_ty = &self.arena.types[trait_ty_id.0 as usize];
                            if let luna_ast::Type::Named { segments, .. } = trait_ast_ty {
                                if segments.len() == 1 {
                                    let s = &segments[0];
                                    let name = self.get_span_text(*s);
                                    self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, impl_scope)
                                        .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(name, s.ctxt, crate::ScopeId(0)))
                                } else {
                                    let mut scope = impl_scope;
                                    let mut res = None;
                                    for (i, seg) in segments.iter().enumerate() {
                                        let seg_name = self.get_span_text(*seg);
                                        let sym_id = if i == 0 {
                                            self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                                .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                                        } else {
                                            self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                                        };
                                        if let Some(id) = sym_id {
                                            res = Some(id);
                                            if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                                scope = inner;
                                            } else if i < segments.len() - 1 {
                                                res = None;
                                                break;
                                            }
                                        } else {
                                            res = None;
                                            break;
                                        }
                                    }
                                    res
                                }
                            } else {
                                None
                            }
                        });

                        if let Some(trait_sym) = trait_sym_opt {
                            if Some(trait_sym) == self.ctx.lang_items.get(crate::lang_item::LangItem::Drop) {
                                if let Some(sym_id) = self_sym_opt {
                                    if let Some(&drop_meth_decl) = methods.first() {
                                        if let Some(&drop_meth_sym) = self.ctx.tables.decl_symbols.get(&drop_meth_decl) {
                                            self.ctx.tables.drop_impls.insert(sym_id, drop_meth_sym);
                                        }
                                    }
                                }
                            } else {
                                self.check_trait_impl_lifetime_conformance(trait_sym, methods);
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_trait_impl_lifetime_conformance(
        &mut self,
        trait_sym: luna_common::ids::SymbolId,
        impl_methods: &[luna_ast::DeclId],
    ) {
        let trait_methods = match self.ctx.tables.trait_methods.get(&trait_sym) {
            Some(m) => m.clone(),
            None => return,
        };

        for &meth_decl in impl_methods {
            let impl_meth_sym = match self.ctx.tables.decl_symbols.get(&meth_decl) {
                Some(&s) => s,
                None => continue,
            };
            let meth_name = self.ctx.symbol_table.get_symbol(impl_meth_sym).name.clone();

            // Find corresponding trait method symbol by name
            let trait_meth_sym = trait_methods.iter().find(|&&m_sym| {
                self.ctx.symbol_table.get_symbol(m_sym).name == meth_name
            }).copied();

            let trait_meth_sym = match trait_meth_sym {
                Some(s) => s,
                None => continue,
            };

            let decl_span = if let luna_ast::Decl::Function { name, .. } = &self.arena.decls[meth_decl.0 as usize] {
                *name
            } else {
                luna_common::Span::new(luna_common::ids::FileId(0), 0, 0)
            };

            let trait_contract = self.ctx.tables.fn_lifetime_contracts.get(&trait_meth_sym).cloned();
            let impl_contract = self.ctx.tables.fn_lifetime_contracts.get(&impl_meth_sym).cloned();
            let impl_return_can_carry_safe_reference = self.ctx.tables.symbol_types
                .get(&impl_meth_sym)
                .and_then(|&method_ty| match self.ctx.types.get(self.ctx.types.resolve(method_ty)) {
                    SemanticType::Function { return_type, .. } => Some(self.ctx.types.contains_safe_reference(*return_type)),
                    _ => None,
                })
                .unwrap_or(true);

            // 1. Check Return Provenance Covariance:
            // impl_contract.return_provenance <= trait_contract.return_provenance
            // A trait-level return contract only needs to be repeated by an
            // implementation when its instantiated return type can carry a
            // safe reference. Owned/scalar iterator items do not acquire a
            // borrow merely because other Iterator implementations return refs.
            if impl_return_can_carry_safe_reference {
            if let Some(tc) = &trait_contract {
                if let Some(t_prov) = &tc.return_provenance {
                    let t_indices = t_prov.indices();
                    match impl_contract.as_ref().and_then(|ic| ic.return_provenance.as_ref()) {
                        None => {
                            // Impl omits return provenance required by trait!
                            self.ctx.diagnostics.push(
                                luna_common::Diagnostic::error(format!(
                                    "error[E2016]: LifetimeConstraintViolation: method '{}' in trait impl omits declared lifetime return contract 'life_from' from trait",
                                    meth_name
                                ))
                                .with_code(DiagnosticCode::LifetimeConstraintViolation)
                                .with_span(decl_span),
                            );
                        }
                        Some(i_prov) => {
                            let i_indices = i_prov.indices();
                            // Every index in i_indices must be present in t_indices (subset check)
                            for &idx in i_indices {
                                if !t_indices.contains(&idx) {
                                    self.ctx.diagnostics.push(
                                        luna_common::Diagnostic::error(format!(
                                            "error[E2016]: LifetimeConstraintViolation: method '{}' in trait impl has weaker return provenance (parameter index {}) than trait contract",
                                            meth_name, idx
                                        ))
                                        .with_code(DiagnosticCode::LifetimeConstraintViolation)
                                        .with_span(decl_span),
                                    );
                                    break;
                                }
                            }
                        }
                    }
                } else if let Some(ic) = &impl_contract {
                    if ic.return_provenance.is_some() {
                        self.ctx.diagnostics.push(
                            luna_common::Diagnostic::error(format!(
                                "error[E2016]: LifetimeConstraintViolation: method '{}' in trait impl declares return provenance not present in trait method",
                                meth_name
                            ))
                            .with_code(DiagnosticCode::LifetimeConstraintViolation)
                            .with_span(decl_span),
                        );
                    }
                }
            } else if let Some(ic) = &impl_contract {
                if ic.return_provenance.is_some() {
                    self.ctx.diagnostics.push(
                        luna_common::Diagnostic::error(format!(
                            "error[E2016]: LifetimeConstraintViolation: method '{}' in trait impl declares return provenance not present in trait method",
                            meth_name
                        ))
                        .with_code(DiagnosticCode::LifetimeConstraintViolation)
                        .with_span(decl_span),
                    );
                }
            }
            }

            // 2. Check Outlives Precondition Contravariance:
            // Every constraint in impl_contract must be guaranteed by trait_contract
            if let Some(ic) = &impl_contract {
                for constraint in &ic.outlives_constraints {
                    let satisfied = trait_contract.as_ref().map_or(false, |tc| {
                        tc.outlives_holds(constraint.longer, constraint.shorter)
                    });
                    if !satisfied {
                        self.ctx.diagnostics.push(
                            luna_common::Diagnostic::error(format!(
                                "method '{}' in trait impl requires outlives constraint between parameter {} and {} not guaranteed by trait",
                                meth_name, constraint.longer, constraint.shorter
                            ))
                            .with_code(DiagnosticCode::LifetimeConstraintViolation)
                            .with_span(decl_span),
                        );
                    }
                }
            }
        }
    }


    fn check_bounds_for_call(&mut self, func_sym: luna_common::ids::SymbolId, subst: &crate::ty::Substitution, span: luna_common::Span) {
        if let Some(&decl_id) = self.ctx.tables.symbol_decls.get(&func_sym) {
            self.check_bounds_for_decl(decl_id, subst, span);
        }
    }

    fn check_bounds_for_decl(&mut self, decl_id: luna_ast::DeclId, subst: &crate::ty::Substitution, span: luna_common::Span) {
        {
            let mut i = 0;
            loop {
                let gp_sym = match self.ctx.tables.generic_param_symbols.get(&(decl_id, i)) {
                    Some(&s) => s,
                    None => break,
                };
                // Get the concrete type that was inferred for this generic param
                let inferred_ty_opt = subst.get(gp_sym).copied();
                if let Some(inferred_ty) = inferred_ty_opt {
                    let resolved_ty = self.ctx.types.resolve(inferred_ty);
                    let resolved_sem = self.ctx.types.get(resolved_ty).clone();
                    // An unresolved variable can be completed by later argument
                    // inference. A caller generic parameter instead needs proof
                    // from its own declared bounds or an applicable blanket impl.
                    if matches!(resolved_sem, SemanticType::InferenceVar(_)) {
                        i += 1;
                        continue;
                    }
                    // Find the self key for the resolved type (peel through references/pointers)
                    let concrete_self_key = match &resolved_sem {
                        SemanticType::Struct(s, _, _) | SemanticType::Enum(s, _, _) => Some(crate::semantic_tables::ImplSelfTypeKey::Nominal(*s)),
                        SemanticType::Primitive(b) => Some(crate::semantic_tables::ImplSelfTypeKey::Primitive(*b)),
                        SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                            let inner_ty = self.ctx.types.get(*inner);
                            match inner_ty {
                                SemanticType::Struct(s, _, _) | SemanticType::Enum(s, _, _) => Some(crate::semantic_tables::ImplSelfTypeKey::Nominal(*s)),
                                SemanticType::Primitive(b) => Some(crate::semantic_tables::ImplSelfTypeKey::Primitive(*b)),
                                _ => None,
                            }
                        }
                        _ => None,
                    };
                    if let Some(bounds) = self.ctx.tables.trait_bounds.get(&gp_sym).cloned() {
                        for bound in &bounds {
                            let required_arguments: Vec<_> = bound.trait_args.iter()
                                .map(|&ty| self.ctx.types.subst(ty, subst)).collect();
                            let satisfied = self.prove_trait_bound(resolved_ty, bound.trait_id, &required_arguments);
                            if !satisfied {
                                let trait_name = self.ctx.symbol_table.get_symbol(bound.trait_id).name.clone();
                                let type_name = concrete_self_key.map(|concrete_key| match concrete_key {
                                    crate::semantic_tables::ImplSelfTypeKey::Nominal(s) => self.ctx.symbol_table.get_symbol(s).name.clone(),
                                    crate::semantic_tables::ImplSelfTypeKey::Primitive(b) => format!("{:?}", b).to_lowercase(),
                                    crate::semantic_tables::ImplSelfTypeKey::Slice => "slice".to_string(),
                                }).unwrap_or_else(|| {
                                    crate::comptime::reflect::ComptimeReflection::type_name(resolved_ty, &self.ctx)
                                });
                                let gp_name = self.ctx.symbol_table.get_symbol(gp_sym).name.clone();
                                self.ctx.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "The type `{}` does not implement trait `{}` (required by inferred generic parameter `{}`)",
                                        type_name, trait_name, gp_name
                                    ))
                                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                                    .with_span(span)
                                );
                            }
                        }
                    }
                    if let Some(assoc_bounds) = self.ctx.tables.assoc_type_bounds.get(&gp_sym).cloned() {
                        for (owner, associated, expected) in assoc_bounds {
                            let expected = self.ctx.types.subst(expected, subst);
                            if !self.prove_associated_bound(resolved_ty, owner, associated, expected) {
                                self.ctx.diagnostics.push(Diagnostic::error(format!(
                                    "E_ASSOCIATED_TYPE_MISMATCH: The associated type equality `{}::{}` is not established for `{}`",
                                    self.ctx.symbol_table.get_symbol(owner).name,
                                    self.ctx.symbol_table.get_symbol(associated).name,
                                    crate::comptime::reflect::ComptimeReflection::type_name(resolved_ty, &self.ctx)))
                                    .with_code(DiagnosticCode::TypeMismatch).with_span(span));
                            }
                        }
                    }
                }
                i += 1;
            }
        }
    }

    /// Walk an AST type to find GenericParam references and map them to concrete types from arguments.

    pub fn check_object_safety(&mut self, trait_sym: luna_common::ids::SymbolId, span: Option<luna_common::Span>) -> bool {
        // Rule: core::Drop cannot be made into a dyn object (Rule A & Rule F)
        if Some(trait_sym) == self.ctx.lang_items.get(crate::lang_item::LangItem::Drop) {
            let mut diag = Diagnostic::error(format!(
                "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because destructor methods cannot be called directly",
                self.ctx.symbol_table.get_symbol(trait_sym).name
            )).with_code(DiagnosticCode::NonObjectSafeTrait);
            if let Some(sp) = span { diag = diag.with_span(sp); }
            self.ctx.diagnostics.push(diag);
            return false;
        }

        let Some(&decl_id) = self.ctx.tables.symbol_decls.get(&trait_sym) else {
            return true;
        };
        let Decl::Trait { methods, associated_types, supertraits, .. } = &self.arena.decls[decl_id.0 as usize] else {
            return true;
        };

        // Rule: Trait with associated types cannot be made into a dyn object in v1 (Q5)
        if !associated_types.is_empty() {
            let mut diag = Diagnostic::error(format!(
                "E_DYN_ASSOCIATED_TYPE_UNSUPPORTED: Trait `{}` cannot be made into an object because it has associated types (v1 restriction)",
                self.ctx.symbol_table.get_symbol(trait_sym).name
            )).with_code(DiagnosticCode::NonObjectSafeTrait);
            if let Some(sp) = span { diag = diag.with_span(sp); }
            self.ctx.diagnostics.push(diag);
            return false;
        }

        // Rule: Trait with supertraits cannot be made into a dyn object in v1 (Q7)
        if !supertraits.is_empty() {
            let mut diag = Diagnostic::error(format!(
                "E_DYN_SUPERTRAIT_UNSUPPORTED: Trait `{}` cannot be made into an object because supertrait dyn composition is unsupported in v1",
                self.ctx.symbol_table.get_symbol(trait_sym).name
            )).with_code(DiagnosticCode::NonObjectSafeTrait);
            if let Some(sp) = span { diag = diag.with_span(sp); }
            self.ctx.diagnostics.push(diag);
            return false;
        }

        for &method_decl_id in methods {
            let method_decl = &self.arena.decls[method_decl_id.0 as usize];
            if let Decl::Function { name, generic_params, params, return_type, is_async, .. } = method_decl {
                let method_name = self.get_span_text(*name);
                
                // Rule 0: Async trait methods cannot be made into an object in v1 (Rule I.5)
                if *is_async {
                    let mut diag = Diagnostic::error(format!(
                        "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because method `{}` is async (v1 restriction)",
                        self.ctx.symbol_table.get_symbol(trait_sym).name,
                        method_name
                    )).with_code(DiagnosticCode::NonObjectSafeTrait);
                    if let Some(sp) = span { diag = diag.with_span(sp); }
                    self.ctx.diagnostics.push(diag);
                    return false;
                }
                
                // Rule 1: No generic parameters
                if !generic_params.is_empty() {
                    let mut diag = Diagnostic::error(format!(
                        "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because method `{}` has generic type parameters",
                        self.ctx.symbol_table.get_symbol(trait_sym).name,
                        method_name
                    )).with_code(DiagnosticCode::NonObjectSafeTrait);
                    if let Some(sp) = span { diag = diag.with_span(sp); }
                    self.ctx.diagnostics.push(diag);
                    return false;
                }
                
                // Rule 2: Must have receiver (`self`, `&self`, or `&rw self`)
                if params.is_empty() {
                    let mut diag = Diagnostic::error(format!(
                        "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because method `{}` has no `self` receiver",
                        self.ctx.symbol_table.get_symbol(trait_sym).name,
                        method_name
                    )).with_code(DiagnosticCode::NonObjectSafeTrait);
                    if let Some(sp) = span { diag = diag.with_span(sp); }
                    self.ctx.diagnostics.push(diag);
                    return false;
                }
                let first_param = &self.arena.decls[params[0].0 as usize];
                if let Decl::Param { is_self, .. } = first_param {
                    if !*is_self {
                        let mut diag = Diagnostic::error(format!(
                            "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because method `{}` has no `self` receiver",
                            self.ctx.symbol_table.get_symbol(trait_sym).name,
                            method_name
                        )).with_code(DiagnosticCode::NonObjectSafeTrait);
                        if let Some(sp) = span { diag = diag.with_span(sp); }
                        self.ctx.diagnostics.push(diag);
                        return false;
                    }
                }
                
                // Rule 3: Return type cannot be unboxed `Self`
                if let Some(ret_ty_id) = return_type {
                    let ret_ast = &self.arena.types[ret_ty_id.0 as usize];
                    if let luna_ast::Type::Named { segments, .. } = ret_ast {
                        if let Some(last_seg) = segments.last() {
                            let ret_name = self.get_span_text(*last_seg);
                            if ret_name == "Self" {
                                let mut diag = Diagnostic::error(format!(
                                    "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because method `{}` returns `Self`",
                                    self.ctx.symbol_table.get_symbol(trait_sym).name,
                                    method_name
                                )).with_code(DiagnosticCode::NonObjectSafeTrait);
                                if let Some(sp) = span { diag = diag.with_span(sp); }
                                self.ctx.diagnostics.push(diag);
                                return false;
                            }
                        }
                    }
                }

                // Rule 4: Self cannot appear in non-receiver parameter positions
                for &param_decl_id in params.iter().skip(1) {
                    let param_decl = &self.arena.decls[param_decl_id.0 as usize];
                    if let Decl::Param { ty: Some(param_ty_id), .. } = param_decl {
                        let param_ast = &self.arena.types[param_ty_id.0 as usize];
                        if let luna_ast::Type::Named { segments, .. } = param_ast {
                            if let Some(last_seg) = segments.last() {
                                let param_name = self.get_span_text(*last_seg);
                                if param_name == "Self" {
                                    let mut diag = Diagnostic::error(format!(
                                        "E_TRAIT_NOT_OBJECT_SAFE: Trait `{}` cannot be made into an object because method `{}` has `Self` in parameter position",
                                        self.ctx.symbol_table.get_symbol(trait_sym).name,
                                        method_name
                                    )).with_code(DiagnosticCode::NonObjectSafeTrait);
                                    if let Some(sp) = span { diag = diag.with_span(sp); }
                                    self.ctx.diagnostics.push(diag);
                                    return false;
                                }
                            }
                        }
                    }
                }
            }
        }
        true
    }

    pub fn try_coerce(&mut self, expr_id: luna_ast::ExprId, from_ty: SemanticTypeId, to_ty: SemanticTypeId) -> bool {
        if let Some(kind) = crate::coercion::try_coerce(self.ctx, from_ty, to_ty) {
            self.ctx.tables.coercions.insert(expr_id, kind);
            true
        } else {
            let from_sem = self.ctx.types.get(self.ctx.types.resolve_inference(from_ty));
            let to_sem = self.ctx.types.get(self.ctx.types.resolve_inference(to_ty));
            if matches!(from_sem, SemanticType::Primitive(BuiltinType::F64))
                && matches!(to_sem, SemanticType::Primitive(BuiltinType::F32))
            {
                let expr = &self.arena.exprs[expr_id.0 as usize];
                if let Expr::Literal(tok, text) = expr {
                    if tok.kind == TokenKind::FloatLiteral && !text.ends_with("f64") {
                        self.ctx.tables.expr_types.insert(expr_id, to_ty);
                        return true;
                    }
                }
            }
            false
        }
    }

    pub fn lower_type(&mut self, ast_ty_id: luna_ast::TypeId) -> SemanticTypeId {
        if let Some(&cached) = self.ctx.tables.ast_type_to_semantic.get(&ast_ty_id) {
            return cached;
        }
        let sem_ty_id = self.lower_type_inner(ast_ty_id);
        self.ctx.tables.ast_type_to_semantic.insert(ast_ty_id, sem_ty_id);
        sem_ty_id
    }

    fn lower_type_inner(&mut self, ast_ty_id: luna_ast::TypeId) -> SemanticTypeId {
        use luna_ast::Type;
        let ast_ty = &self.arena.types[ast_ty_id.0 as usize];
        let sem_ty_id = match ast_ty {
            Type::Builtin(kind) => {
                let builtin = match kind {
                    BuiltinKind::I8 => BuiltinType::I8,
                    BuiltinKind::I16 => BuiltinType::I16,
                    BuiltinKind::I32 => BuiltinType::I32,
                    BuiltinKind::I64 => BuiltinType::I64,
                    BuiltinKind::I128 => BuiltinType::I128,
                    BuiltinKind::Isize => BuiltinType::Isize,
                    BuiltinKind::U8 => BuiltinType::U8,
                    BuiltinKind::U16 => BuiltinType::U16,
                    BuiltinKind::U32 => BuiltinType::U32,
                    BuiltinKind::U64 => BuiltinType::U64,
                    BuiltinKind::U128 => BuiltinType::U128,
                    BuiltinKind::Usize => BuiltinType::Usize,
                    BuiltinKind::F32 => BuiltinType::F32,
                    BuiltinKind::F64 => BuiltinType::F64,
                    BuiltinKind::Bool => BuiltinType::Bool,
                    BuiltinKind::Str => BuiltinType::String,
                    BuiltinKind::Char => BuiltinType::Char,
                    BuiltinKind::Void => return self.ctx.types.intern(SemanticType::Void),
                };
                self.ctx.types.intern(SemanticType::Primitive(builtin))
            }
            Type::Named { segments, generic_args, .. } => {
                let name = segments.last().map(|span| self.get_span_text(*span));
                if let Some("Self" | "self") = name {
                    if let Some(self_ty) = self.current_self_type {
                        return self_ty;
                    } else {
                        return self.ctx.types.new_inference_var();
                    }
                }
                if segments.len() >= 2 {
                    let first_seg = self.get_span_text(segments[0]);
                    if first_seg == "Self" || first_seg == "self" {
                        // Associated type projection like Self::Output or Self::Residual
                        return self.ctx.types.new_inference_var();
                    }
                }
                if let Some("Future") = name {
                    if let Some(&arg) = generic_args.first() {
                        let out_ty = self.lower_type(arg);
                        return self.ctx.types.intern(SemanticType::Future(out_ty));
                    }
                }
                let last_span = segments.last().copied();
                let ctxt = last_span.map(|s| s.ctxt).unwrap_or(luna_common::ids::SyntaxContext::ROOT);
                // Check if already resolved by Resolver, or resolve path
                let symbol = self.ctx.tables.type_symbols.get(&ast_ty_id).copied().or_else(|| {
                    if segments.len() == 1 {
                        name.and_then(|n| {
                            self.ctx.symbol_table.lookup_with_ctxt(n, ctxt, self.current_scope)
                                .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(n, ctxt, crate::ScopeId(0)))
                        })
                    } else {
                        let mut scope = self.current_scope;
                        let mut res = None;
                        for (i, seg) in segments.iter().enumerate() {
                            let seg_name = self.get_span_text(*seg);
                            let sym_id = if i == 0 {
                                self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                    .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                            } else {
                                self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                            };
                            if let Some(id) = sym_id {
                                if i > 0 && !self.ctx.symbol_table.is_accessible(id, self.current_scope, self.ctx.current_provider) {
                                    self.ctx.diagnostics.push(Diagnostic::error(format!("Type `{}` is private and cannot be accessed from this scope", seg_name)).with_code(DiagnosticCode::PrivateSymbolAccess).with_span(*seg));
                                    return None;
                                }
                                res = Some(id);
                                if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                    scope = inner;
                                } else if i < segments.len() - 1 {
                                    res = None;
                                    break;
                                }
                            } else {
                                res = None;
                                break;
                            }
                        }
                        res
                    }
                });
                if let Some(sym) = symbol {
                    if !self.ctx.symbol_table.is_accessible(sym, self.current_scope, self.ctx.current_provider) {
                        let name_str = segments.iter().map(|s| self.get_span_text(*s)).collect::<Vec<_>>().join("::");
                        let span = segments.last().copied().unwrap_or(luna_common::Span::default());
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Type `{}` is private and cannot be accessed from this scope", name_str)).with_code(DiagnosticCode::PrivateSymbolAccess).with_span(span));
                        return self.ctx.types.intern(SemanticType::Error);
                    }
                    let sym_kind = self.ctx.symbol_table.get_symbol(sym).kind.clone();
                    if let crate::SymbolKind::TypeParam = sym_kind {
                        // This is a generic type parameter - return GenericParam type
                        self.ctx.types.intern(SemanticType::GenericParam(sym))
                    } else {
                        // Resolution knows declarations independently of source
                        // order. Populate a referenced local nominal definition
                        // before lowering its use, rather than inventing an
                        // unconstrained inference variable for a known symbol.
                        // Imported types are already available; recursive local
                        // definitions use the dummy identity installed while
                        // their own declaration is being populated.
                        if !self.ctx.tables.symbol_types.contains_key(&sym) {
                            if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym).copied() {
                                if matches!(self.arena.decls.get(decl_id.0 as usize),
                                    Some(Decl::Struct { .. } | Decl::Enum { .. }))
                                {
                                    self.populate_types(&[Item::Decl(decl_id)]);
                                }
                            }
                        }
                        let base_ty = self.ctx.tables.symbol_types.get(&sym).copied().unwrap_or_else(|| self.ctx.types.new_inference_var());
                        if generic_args.is_empty() {
                            base_ty
                        } else {
                            let mut concrete_args = Vec::new();
                            for arg in generic_args {
                                concrete_args.push(self.lower_type(*arg));
                            }
                            let mut subst = crate::ty::Substitution::new();
                            if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym).copied() {
                                for (idx, &arg_ty) in concrete_args.iter().enumerate() {
                                    if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(decl_id, idx)) {
                                        subst.insert(*gp_sym, arg_ty);
                                    }
                                }
                            }
                            // A type alias owns its own generic parameter list.
                            // Substitute through the complete alias body; using
                            // the alias arguments as the outer nominal arguments
                            // corrupts nested aliases such as
                            // `Wrapper<Pair<A, B>>`.
                            if matches!(sym_kind, crate::SymbolKind::Alias) {
                                return self.ctx.types.subst(base_ty, &subst);
                            }

                            let resolved_ty = self.ctx.types.get(base_ty).clone();
                            match resolved_ty {
                                SemanticType::Struct(s_sym, _, original_field_tys) => {
                                    let mut field_tys = Vec::new();
                                    for orig_ty in original_field_tys {
                                        field_tys.push(self.ctx.types.subst(orig_ty, &subst));
                                    }
                                    self.ctx.types.intern(SemanticType::Struct(s_sym, concrete_args, field_tys))
                                }
                                SemanticType::Enum(e_sym, _, original_var_tys) => {
                                    let mut variant_tys = Vec::new();
                                    for orig_ty in original_var_tys {
                                        variant_tys.push(self.ctx.types.subst(orig_ty, &subst));
                                    }
                                    self.ctx.types.intern(SemanticType::Enum(e_sym, concrete_args, variant_tys))
                                }
                                _ => base_ty,
                            }
                        }
                    }
                } else if segments.len() >= 2 {
                    let last_seg = segments.last().unwrap();
                    let assoc_name = self.get_span_text(*last_seg).to_string();
                    let prefix_segments = &segments[..segments.len() - 1];
                    let span = luna_common::Span {
                        file_id: segments[0].file_id,
                        start: segments[0].start,
                        end: last_seg.end,
                        ctxt: segments[0].ctxt,
                    };

                    let prefix_ty: Option<SemanticTypeId> = if prefix_segments.len() == 1 && (self.get_span_text(prefix_segments[0]) == "Self" || self.get_span_text(prefix_segments[0]) == "self") {
                        self.current_self_type
                    } else {
                        let mut scope = self.current_scope;
                        let mut prefix_sym = None;
                        for (i, seg) in prefix_segments.iter().enumerate() {
                            let seg_name = self.get_span_text(*seg);
                            let sym_id = if i == 0 {
                                self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                    .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                            } else {
                                self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                            };
                            if let Some(id) = sym_id {
                                prefix_sym = Some(id);
                                if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                    scope = inner;
                                } else if i < prefix_segments.len() - 1 {
                                    prefix_sym = None;
                                    break;
                                }
                            } else {
                                prefix_sym = None;
                                break;
                            }
                        }

                        if let Some(sym) = prefix_sym {
                            let sym_kind = self.ctx.symbol_table.get_symbol(sym).kind.clone();
                            if let crate::SymbolKind::TypeParam = sym_kind {
                                Some(self.ctx.types.intern(SemanticType::GenericParam(sym)))
                            } else {
                                let base_ty = self.ctx.tables.symbol_types.get(&sym).copied();
                                if let Some(base_ty) = base_ty {
                                    if generic_args.is_empty() {
                                        Some(base_ty)
                                    } else {
                                        Some(self.instantiate_generic_named_type(sym, base_ty, generic_args))
                                    }
                                } else {
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(self_ty) = prefix_ty {
                        self.lower_associated_type_projection(self_ty, &assoc_name, span)
                    } else {
                        let name_str = segments.iter().map(|s| self.get_span_text(*s)).collect::<Vec<_>>().join("::");
                        self.ctx.diagnostics.push(luna_common::Diagnostic::error(format!("cannot find type `{}` in this scope", name_str)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(span));
                        self.ctx.types.intern(SemanticType::Error)
                    }
                } else {
                    let name_str = segments.iter().map(|s| self.get_span_text(*s)).collect::<Vec<_>>().join("::");
                    let span = luna_common::Span {
                        file_id: segments[0].file_id,
                        start: segments[0].start,
                        end: segments.last().unwrap().end,
                        ctxt: segments[0].ctxt,
                    };
                    self.ctx.diagnostics.push(luna_common::Diagnostic::error(format!("cannot find type `{}` in this scope", name_str)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(span));
                    self.ctx.types.intern(SemanticType::Error)
                }
            }
            Type::Tuple { elements } => {
                let mut elem_tys = Vec::new();
                for &e in elements {
                    elem_tys.push(self.lower_type(e));
                }
                self.ctx.types.intern(SemanticType::Tuple(elem_tys))
            }
            Type::Function { params, return_type, is_unsafe } => {
                let mut param_tys = Vec::new();
                for &p in params {
                    param_tys.push(self.lower_type(p));
                }
                let ret_ty = if let Some(r) = return_type { self.lower_type(*r) } else { self.ctx.types.intern(SemanticType::Void) };
                let func_ty = self.ctx.types.intern(SemanticType::Function { params: param_tys, return_type: ret_ty, is_unsafe: *is_unsafe });
                func_ty
            }
            Type::Slice { inner } => {
                let inner_ty = self.lower_type(*inner);
                self.ctx.types.intern(SemanticType::Pointer(crate::ty::Mutability::Mutable, inner_ty));
                self.ctx.types.intern(SemanticType::Slice(inner_ty))
            }
            Type::Array { element_type, size } => {
                let inner_ty = self.lower_type(*element_type);
                let resolved_size = match self.eval_comptime_expr(*size) {
                    Ok(val) => val.as_usize().unwrap_or(0) as u64,
                    Err(e) => {
                        let span = self.get_expr_span_for_diag(size).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.report_comptime_error(e, span, "cannot evaluate array size in comptime");
                        0
                    }
                };
                self.ctx.types.intern(SemanticType::Array(inner_ty, resolved_size))
            }
            Type::Reference { is_mutable, inner, .. } => {
                let inner_ty = self.lower_type(*inner);
                let mutability = if *is_mutable { crate::ty::Mutability::Mutable } else { crate::ty::Mutability::Immutable };
                self.ctx.types.intern(SemanticType::Reference(crate::ty::LifetimeId(0), mutability, inner_ty))
            }
            Type::Pointer { is_mutable, inner, .. } => {
                let inner_ty = self.lower_type(*inner);
                let mutability = if *is_mutable { crate::ty::Mutability::Mutable } else { crate::ty::Mutability::Immutable };
                self.ctx.types.intern(SemanticType::Pointer(mutability, inner_ty))
            }
            Type::TraitObject { trait_type } => {
                let trait_ast = &self.arena.types[trait_type.0 as usize];
                if let Type::Named { segments, generic_args, associated_bindings } = trait_ast {
                    let last_span = segments.last().copied();
                    if !generic_args.is_empty() || !associated_bindings.is_empty() {
                        let mut diag = Diagnostic::error("E_DYN_ASSOCIATED_TYPE_UNSUPPORTED: dyn Trait with associated type bindings or type arguments is unsupported in v1").with_code(DiagnosticCode::NonObjectSafeTrait);
                        if let Some(sp) = last_span { diag = diag.with_span(sp); }
                        self.ctx.diagnostics.push(diag);
                        return self.ctx.types.intern(SemanticType::Error);
                    }
                    let trait_sym = self.ctx.tables.type_symbols.get(trait_type).copied().or_else(|| {
                        if segments.len() == 1 {
                            let seg = segments[0];
                            let name = self.get_span_text(seg);
                            self.ctx.symbol_table.lookup_with_ctxt(name, seg.ctxt, self.current_scope)
                                .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(name, seg.ctxt, crate::ScopeId(0)))
                        } else {
                            let mut scope = self.current_scope;
                            let mut res = None;
                            for (i, seg) in segments.iter().enumerate() {
                                let seg_name = self.get_span_text(*seg);
                                let sym_id = if i == 0 {
                                    self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                        .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                                } else {
                                    self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                                };
                                if let Some(id) = sym_id {
                                    res = Some(id);
                                    if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                        scope = inner;
                                    } else if i < segments.len() - 1 {
                                        res = None;
                                        break;
                                    }
                                } else {
                                    res = None;
                                    break;
                                }
                            }
                            res
                        }
                    });

                    if let Some(trait_sym) = trait_sym {
                        if !self.check_object_safety(trait_sym, last_span) {
                            return self.ctx.types.intern(SemanticType::Error);
                        }
                        self.ctx.types.intern(SemanticType::DynTrait(trait_sym))
                    } else {
                        let mut diag = Diagnostic::error("Trait not found for dyn Trait").with_code(DiagnosticCode::UnresolvedSymbol);
                        if let Some(sp) = last_span { diag = diag.with_span(sp); }
                        self.ctx.diagnostics.push(diag);
                        self.ctx.types.intern(SemanticType::Error)
                    }
                } else {
                    self.ctx.types.intern(SemanticType::Error)
                }
            }
            Type::MacroCall { span, .. } => {
                self.ctx.diagnostics.push(Diagnostic::error("internal compiler error: unexpanded macro call in type position").with_code(DiagnosticCode::InvalidSyntax).with_span(*span));
                self.ctx.types.intern(SemanticType::Error)
            }
            Type::Typeof { expr } => {
                self.typecheck_expr(expr)
            }
            Type::Never => {
                self.ctx.types.intern(SemanticType::Never)
            }
            other => {
                self.ctx.diagnostics.push(Diagnostic::error(format!("Unsupported or unrecognized type construct in semantic phase: {:?}", other)).with_code(DiagnosticCode::InvalidSyntax));
                self.ctx.types.intern(SemanticType::Error)
            }
        };
        self.ctx.tables.ast_type_to_semantic.insert(ast_ty_id, sem_ty_id);
        sem_ty_id
    }

    fn typecheck_item(&mut self, item: &Item) {
        match item {
            Item::Decl(decl_id) => {
                let decl = &self.arena.decls[decl_id.0 as usize];
                match decl {
                    Decl::Function { name, params, body, is_async, return_type, is_unsafe, .. } => {
                        self.comptime_prepared_functions.insert(*decl_id);
                        let prev_scope = self.current_scope;
                        let mut is_main = false;
                        if let Some(sym_id) = self.ctx.tables.decl_symbols.get(decl_id).copied() {
                            let sym = self.ctx.symbol_table.get_symbol(sym_id);
                            if sym.name == "main" {
                                is_main = true;
                            }
                            if let Some(inner) = sym.inner_scope {
                                self.current_scope = inner;
                            }
                        }
                        let prev_async = self.current_async_fn;
                        if *is_async {
                            self.current_async_fn = Some(*decl_id);
                        } else {
                            self.current_async_fn = None;
                        }
                        let prev_unsafe = self.is_unsafe_context;
                        if *is_unsafe {
                            self.is_unsafe_context = true;
                        }
                        let ret_ty = if let Some(r) = return_type {
                            self.lower_type(*r)
                        } else {
                            self.ctx.types.intern(SemanticType::Void)
                        };

                        let mut is_valid_main = false;
                        if is_main {
                            let mut valid = true;
                            if *is_async {
                                valid = false;
                            }
                            if params.len() > 1 {
                                valid = false;
                            } else if params.len() == 1 {
                                let param_decl = &self.arena.decls[params[0].0 as usize];
                                if let Decl::Param { ty: Some(param_ty_id), is_self, name: param_name, .. } = param_decl {
                                    if *is_self || self.get_span_text(*param_name) != "args" {
                                        valid = false;
                                    } else {
                                        let sem_param_ty = self.lower_type(*param_ty_id);
                                        if let SemanticType::Slice(inner_ty) = self.ctx.types.get(sem_param_ty) {
                                            if let SemanticType::Primitive(BuiltinType::String) = self.ctx.types.get(*inner_ty) {
                                                // valid
                                            } else {
                                                valid = false;
                                            }
                                        } else {
                                            valid = false;
                                        }
                                    }
                                } else {
                                    valid = false;
                                }
                            }
                            if valid {
                                if let SemanticType::Primitive(BuiltinType::I32) = self.ctx.types.get(ret_ty) {
                                    is_valid_main = true;
                                } else if let SemanticType::Void = self.ctx.types.get(ret_ty) {
                                    if params.len() == 0 {
                                        is_valid_main = true;
                                    }
                                }
                            }
                            if !is_valid_main {
                                self.ctx.diagnostics.push(
                                    Diagnostic::error("E_INVALID_MAIN_SIGNATURE: main must match one of: fn main() -> void, fn main() -> i32, fn main(args: [str]) -> i32")
                                        .with_code(DiagnosticCode::InvalidMainSignature)
                                        .with_span(*name)
                                );
                            }
                        }

                        if !is_main && self.ctx.types.is_unsized(ret_ty) {
                            self.ctx.diagnostics.push(
                                Diagnostic::error(format!(
                                    "E_UNSIZED_TYPE_IN_VALUE_POSITION: Return type `{:?}` is unsized and cannot be returned by value",
                                    self.ctx.types.get(ret_ty)
                                ))
                                .with_code(DiagnosticCode::TypeMismatch)
                                .with_span(*name)
                            );
                        }

                        for &param_decl_id in params {
                            let param_decl = &self.arena.decls[param_decl_id.0 as usize];
                            if let Decl::Param { ty: Some(param_ty_id), is_self, name: param_name, .. } = param_decl {
                                if !*is_self {
                                    let sem_param_ty = self.lower_type(*param_ty_id);
                                    let mut suppress_unsized = false;
                                    if is_main && is_valid_main && self.get_span_text(*param_name) == "args" {
                                        suppress_unsized = true;
                                    }
                                    if self.ctx.types.is_unsized(sem_param_ty) && !suppress_unsized {
                                        self.ctx.diagnostics.push(
                                            Diagnostic::error(format!(
                                                "E_UNSIZED_TYPE_IN_VALUE_POSITION: Parameter `{}` has unsized type `{:?}` which cannot be passed by value",
                                                self.get_span_text(*param_name),
                                                self.ctx.types.get(sem_param_ty)
                                            ))
                                            .with_code(DiagnosticCode::TypeMismatch)
                                            .with_span(*param_name)
                                        );
                                    }
                                }
                            }
                        }
                        self.current_return_type.push(ret_ty);
                        if let Some(body_stmt) = body {
                            self.typecheck_function_body(body_stmt, ret_ty);
                        }
                        self.current_return_type.pop();
                        self.is_unsafe_context = prev_unsafe;
                        self.current_async_fn = prev_async;
                        self.current_scope = prev_scope;
                    }
                    Decl::Var { name, initializer, pattern, type_annot, is_const, .. } => {
                        let mut init_ty = if let Some(init) = initializer {
                            if let Some(annot) = type_annot {
                                let expected_ty = self.lower_type(*annot);
                                self.expected_expr_type.push(expected_ty);
                                let init_ty = self.typecheck_expr(init);
                                self.expected_expr_type.pop();
                                init_ty
                            } else {
                                self.typecheck_expr(init)
                            }
                        } else {
                            self.ctx.types.new_inference_var()
                        };
                        if matches!(self.ctx.types.get(init_ty), SemanticType::Error) {
                            if let Some(symbol) = self.ctx.tables.decl_symbols.get(decl_id).copied() {
                                self.ctx.tables.symbol_types.insert(symbol, init_ty);
                            }
                            return;
                        }
                        
                        if let Some(annot) = type_annot {
                            let expected_ty = self.lower_type(*annot);
                            if self.ctx.types.is_unsized(expected_ty) {
                                self.ctx.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "E_UNSIZED_TYPE_IN_VALUE_POSITION: Type `{:?}` is unsized and cannot appear in value position",
                                        self.ctx.types.get(expected_ty)
                                    ))
                                    .with_code(DiagnosticCode::TypeMismatch)
                                    .with_span(*name)
                                );
                            }
                            if let Some(init) = initializer {
                                if !self.try_coerce(*init, init_ty, expected_ty) {
                                    if let Err(e) = self.unify(expected_ty, init_ty) {
                                        let span = self.get_expr_span_for_diag(init).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                        self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                                    } else {
                                        init_ty = expected_ty;
                                    }
                                } else {
                                    init_ty = expected_ty;
                                }
                            } else {
                                init_ty = expected_ty;
                            }
                        } else {
                            if self.ctx.types.is_unsized(init_ty) {
                                let span = initializer.as_ref().and_then(|i| self.get_expr_span_for_diag(i)).unwrap_or(*name);
                                self.ctx.diagnostics.push(
                                    Diagnostic::error(format!(
                                        "E_UNSIZED_TYPE_IN_VALUE_POSITION: Type `{:?}` is unsized and cannot appear in value position",
                                        self.ctx.types.get(init_ty)
                                    ))
                                    .with_code(DiagnosticCode::TypeMismatch)
                                    .with_span(span)
                                );
                            }
                        }
                        
                        if *is_const {
                            if let Some(init) = initializer {
                                if let Err(msg) = crate::const_eval::is_const_evaluable(self.arena, self.ctx, self.source_manager, *init) {
                                    self.ctx.diagnostics.push(Diagnostic::error(format!("cannot evaluate constant in comptime: {}", msg)).with_code(DiagnosticCode::ComptimeEvaluationFailed).with_span(*name));
                                } else {
                                    match self.eval_comptime_expr(*init) {
                                        Ok(val) => {
                                             self.ctx.comptime_values.insert(*init, val.clone());
                                            if let Some(sym_id) = self.ctx.tables.decl_symbols.get(decl_id).copied() {
                                                self.ctx.const_values.insert(sym_id, val);
                                            }
                                        }
                                        Err(e) => {
                                            self.report_comptime_error(e, *name, "cannot evaluate constant in comptime");
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(sym_id) = self.ctx.tables.decl_symbols.get(decl_id).copied() {
                            let mut needs_update = false;
                            if let Some(current_ty) = self.ctx.tables.symbol_types.get(&sym_id).copied() {
                                let resolved = self.ctx.types.get(current_ty).clone();
                                if matches!(resolved, SemanticType::InferenceVar(_)) {
                                    needs_update = true;
                                }
                            } else {
                                needs_update = true;
                            }
                            if needs_update {
                                self.ctx.tables.symbol_types.insert(sym_id, init_ty);
                            }
                        }
                        
                        if let Some(pat_id) = pattern {
                            self.typecheck_pattern(pat_id, init_ty);
                        }
                    }
                    Decl::Impl { generic_params: _, self_type, methods, .. } => {
                        let prev_scope = self.current_scope;
                        let impl_scope = *self.ctx.tables.decl_scopes.get(decl_id).unwrap_or(&crate::ScopeId(0));
                        self.current_scope = impl_scope;
                        let self_sem_ty = self.lower_type(*self_type);
                        let prev_self = self.current_self_type;
                        self.current_self_type = Some(self_sem_ty);
                        for method_id in methods {
                            let method_decl = &self.arena.decls[method_id.0 as usize];
                            if let Decl::Function { body: Some(body_stmt), is_async, return_type, is_unsafe, .. } = method_decl {
                                let prev_method_scope = self.current_scope;
                                // Enter method scope
                                if let Some(sym_id) = self.ctx.tables.decl_symbols.get(method_id).copied() {
                                    if let Some(inner) = self.ctx.symbol_table.get_symbol(sym_id).inner_scope {
                                        self.current_scope = inner;
                                    }
                                }
                                let prev_async = self.current_async_fn;
                                if *is_async {
                                    self.current_async_fn = Some(*method_id);
                                } else {
                                    self.current_async_fn = None;
                                }
                                let prev_unsafe = self.is_unsafe_context;
                                if *is_unsafe {
                                    self.is_unsafe_context = true;
                                }
                                let ret_ty = if let Some(r) = return_type {
                                    self.lower_type(*r)
                                } else {
                                    self.ctx.types.intern(SemanticType::Void)
                                };
                                self.current_return_type.push(ret_ty);
                                self.typecheck_function_body(body_stmt, ret_ty);
                                self.current_return_type.pop();
                                self.is_unsafe_context = prev_unsafe;
                                self.current_async_fn = prev_async;
                                self.current_scope = prev_method_scope;
                            }
                        }
                        self.current_self_type = prev_self;
                        self.current_scope = prev_scope;
                    }
                    Decl::Extern { func, .. } => {
                        // The inner func is registered, we must check its FFI safety
                        if let Some(sym_id) = self.ctx.tables.decl_symbols.get(func).copied() {
                            let mut eff = crate::effect::EffectSet::pure();
                            eff.add(crate::effect::Effect::Extern);
                            eff.add(crate::effect::Effect::IO);
                            self.ctx.tables.function_effects.insert(sym_id, eff);

                            if let Some(func_ty) = self.ctx.tables.symbol_types.get(&sym_id).copied() {
                                if let Err(e) = self.is_ffi_safe(func_ty) {
                                    let span = self.ctx.symbol_table.get_symbol(sym_id).span;
                                    self.ctx.diagnostics.push(
                                        Diagnostic::error(format!("error[E2030]: NonFfiSafeType: Extern function signature is not FFI-safe: {}", e))
                                            .with_code(DiagnosticCode::NonFfiSafeType)
                                            .with_span(span)
                                    );
                                }
                            }
                        }
                    }
                    Decl::Module { items: inner_decls, .. } => {
                        let prev_scope = self.current_scope;
                        if let Some(&sym_id) = self.ctx.tables.decl_symbols.get(decl_id) {
                            if let Some(inner) = self.ctx.symbol_table.symbols[sym_id.0 as usize].inner_scope {
                                self.current_scope = inner;
                            }
                        }
                        for &child_decl in inner_decls {
                            self.typecheck_item(&Item::Decl(child_decl));
                        }
                        self.current_scope = prev_scope;
                    }
                    _ => {}
                }
            }
            Item::Stmt(stmt_id) => {
                self.typecheck_stmt(stmt_id);
            }
        }
    }

    fn typecheck_pattern(&mut self, pat_id: &luna_ast::PatId, ty: SemanticTypeId) {
        if let Some(sym_id) = self.ctx.tables.pat_symbols.get(pat_id).copied() {
            let kind = self.ctx.symbol_table.get_symbol(sym_id).kind;
            if matches!(kind, crate::SymbolKind::Variable | crate::SymbolKind::Constant) {
                self.ctx.tables.symbol_types.insert(sym_id, ty);
            }
        }
        self.ctx.tables.pat_types.insert(*pat_id, ty);
        
        let pattern = &self.arena.pats[pat_id.0 as usize];
        match pattern {
            luna_ast::Pattern::Literal(token) if token.kind == TokenKind::IntegerLiteral => {
                let text = self.get_span_text(token.span).to_string();
                self.expected_expr_type.push(ty);
                let literal_ty = self.typecheck_integer_literal_at(token.span, &text, false);
                self.expected_expr_type.pop();
                if let Err(message) = self.unify(ty, literal_ty) {
                    self.ctx.diagnostics.push(Diagnostic::error(message).with_code(DiagnosticCode::TypeMismatch).with_span(token.span));
                }
            }
            luna_ast::Pattern::Tuple { elements, .. } => {
                let resolved_ty = self.ctx.types.get(ty).clone();
                if let SemanticType::Tuple(elem_tys) = resolved_ty {
                    if elements.len() != elem_tys.len() {
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Tuple pattern has {} elements, but tuple type has {}", elements.len(), elem_tys.len())).with_code(DiagnosticCode::TypeMismatch));
                    }
                    for (i, elem) in elements.iter().enumerate() {
                        if i < elem_tys.len() {
                            self.typecheck_pattern(elem, elem_tys[i]);
                        }
                    }
                }
            }
            luna_ast::Pattern::Struct { fields, path, has_rest } => {
                let resolved_ty = self.ctx.types.get(ty).clone();
                if let SemanticType::Struct(sym_id, _, _) = resolved_ty {
                    let decl_id_opt = self.ctx.symbol_table.get_symbol(sym_id).decl_id;
                    if let Some(decl_id) = decl_id_opt {
                        let decl = self.arena.decls[decl_id.0 as usize].clone();
                        if let luna_ast::Decl::Struct { fields: struct_fields, .. } = &decl {
                            let mut pat_span = luna_common::ids::Span::new(luna_common::ids::FileId(0), 0, 0);
                            if let Some(first) = path.first() { pat_span = *first; }
                            
                            // Check for missing fields if has_rest is false
                            let mut missing_fields = false;
                            if !*has_rest {
                                for struct_field in struct_fields {
                                    let struct_field_name = self.get_span_text(struct_field.name);
                                    let mut provided = false;
                                    for field in fields {
                                        let field_name_str = self.get_span_text(field.name);
                                        if field_name_str == struct_field_name { provided = true; break; }
                                    }
                                    if !provided {
                                        self.ctx.diagnostics.push(luna_common::diagnostic::Diagnostic::error(format!("Pattern requires field `{}` but it was not provided", struct_field_name)).with_code(DiagnosticCode::MissingField).with_span(pat_span));
                                        missing_fields = true;
                                    }
                                }
                            }
                            if missing_fields {
                                return;
                            }

                            for field in fields {
                                let field_name_str = self.get_span_text(field.name);
                                
                                let mut field_ty = self.ctx.types.new_inference_var();
                                let mut found = false;
                                for struct_field in struct_fields {
                                    let struct_field_name = self.get_span_text(struct_field.name);
                                    if field_name_str == struct_field_name {
                                        self.check_field_visibility(sym_id, struct_field.visibility, &field_name_str, field.name);
                                        field_ty = self.ctx.tables.ast_type_to_semantic.get(&struct_field.ty).copied().unwrap_or(field_ty);
                                        found = true;
                                        break;
                                    }
                                }
                                
                                if !found {
                                    self.ctx.diagnostics.push(luna_common::diagnostic::Diagnostic::error(format!("Struct has no field named `{}`", field_name_str)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(field.name));
                                }
                                
                                if let Some(field_pat) = field.pattern {
                                    self.typecheck_pattern(&field_pat, field_ty);
                                }
                            }
                        }
                    }
                }
            }
            luna_ast::Pattern::Enum { fields, path } => {
                let mut pat_span = luna_common::ids::Span::new(luna_common::ids::FileId(0), 0, 0);
                if let Some(first) = path.first() { pat_span = *first; }

                let mut variant_payload_tys = Vec::new();
                if let Some(variant_sym_id) = self.ctx.tables.pat_symbols.get(pat_id).copied() {
                    let variant_sym = self.ctx.symbol_table.get_symbol(variant_sym_id);
                    if let crate::symbol::SymbolKind::EnumVariant(variant_idx) = variant_sym.kind {
                        if let Some(decl_id) = variant_sym.decl_id {
                            let enum_sym_id = self.ctx.tables.decl_symbols.get(&decl_id).copied();
                            let base_enum_ty = enum_sym_id.and_then(|id| self.ctx.tables.symbol_types.get(&id).copied());

                            if let Some(base_ty) = base_enum_ty {
                                if let SemanticType::Enum(e_sym, base_args, original_var_tys) = self.ctx.types.get(base_ty).clone() {
                                    let mut subst = crate::ty::Substitution::new();
                                    let mut fresh_args = Vec::new();
                                    for arg in &base_args {
                                        if let SemanticType::GenericParam(gp) = self.ctx.types.get(*arg).clone() {
                                            let ivar = self.ctx.types.new_inference_var();
                                            subst.insert(gp, ivar);
                                            fresh_args.push(ivar);
                                        } else {
                                            fresh_args.push(*arg);
                                        }
                                    }
                                    let mut instantiated_var_tys = Vec::new();
                                    for orig_ty in original_var_tys {
                                        instantiated_var_tys.push(self.ctx.types.subst(orig_ty, &subst));
                                    }
                                    let fresh_enum_ty = self.ctx.types.intern(SemanticType::Enum(e_sym, fresh_args, instantiated_var_tys));
                                    if let Err(err) = self.unify(fresh_enum_ty, ty) {
                                        self.ctx.diagnostics.push(luna_common::diagnostic::Diagnostic::error(err).with_code(DiagnosticCode::TypeMismatch).with_span(pat_span));
                                    }
                                }
                            }

                            let resolved_subj_ty = self.ctx.types.resolve(ty);
                            if let SemanticType::Enum(_, _, ref var_tys) = self.ctx.types.get(resolved_subj_ty).clone() {
                                if let Some(&payload_ty) = var_tys.get(variant_idx as usize) {
                                    let resolved_payload = self.ctx.types.resolve(payload_ty);
                                    if fields.is_empty() {
                                        // Unit variant
                                    } else if fields.len() == 1 {
                                        variant_payload_tys.push(resolved_payload);
                                    } else if let SemanticType::Tuple(ref elems) = self.ctx.types.get(resolved_payload).clone() {
                                        variant_payload_tys.extend(elems.iter().map(|&e| self.ctx.types.resolve(e)));
                                    } else {
                                        variant_payload_tys.push(resolved_payload);
                                    }
                                }
                            }
                        }
                    }
                }
                
                if fields.len() != variant_payload_tys.len() {
                    self.ctx.diagnostics.push(luna_common::diagnostic::Diagnostic::error(format!("Enum variant expects {} fields, but {} were provided", variant_payload_tys.len(), fields.len())).with_code(DiagnosticCode::TypeMismatch).with_span(pat_span));
                    return;
                }

                for (i, field) in fields.iter().enumerate() {
                    let field_ty = variant_payload_tys.get(i).copied().unwrap_or_else(|| self.ctx.types.new_inference_var());
                    self.typecheck_pattern(field, field_ty);
                }
            }
            _ => {}
        }
    }

    fn infer_stmt_value_type(&self, stmt_id: &luna_ast::StmtId) -> Option<SemanticTypeId> {
        match &self.arena.stmts[stmt_id.0 as usize] {
            Stmt::Block { body, tail_expr } => {
                if let Some(expr) = tail_expr {
                    return self.ctx.tables.expr_types.get(expr).copied();
                }
                for item in body.iter().rev() {
                    if let Item::Stmt(stmt) = item {
                        if let Some(ty) = self.infer_stmt_value_type(stmt) {
                            return Some(ty);
                        }
                    }
                }
                None
            }
            Stmt::Return { value: Some(expr) } => self.ctx.tables.expr_types.get(expr).copied(),
            Stmt::If { then_branch, else_branch: Some(else_branch), .. } => {
                self.infer_stmt_value_type(then_branch).or_else(|| self.infer_stmt_value_type(else_branch))
            }
            Stmt::Unsafe { body } => self.infer_stmt_value_type(body),
            _ => None,
        }
    }

    fn select_protocol_impl(
        &mut self,
        trait_id: luna_common::ids::SymbolId,
        self_ty: SemanticTypeId,
        span: luna_common::Span,
    ) -> Option<(crate::semantic_tables::TraitImplEntry, crate::ty::Substitution)> {
        let mut matches = Vec::new();
        for entry in &self.ctx.tables.trait_impl_entries {
            if entry.trait_id != trait_id {
                continue;
            }
            let mut subst = crate::ty::Substitution::new();
            if self.ctx.matches_impl_pattern(
                entry.self_type,
                self_ty,
                &entry.generic_params,
                &mut subst,
            ) {
                matches.push((entry.clone(), subst));
            }
        }

        match matches.len() {
            1 => matches.pop(),
            0 => {
                let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!(
                        "Type `{:?}` does not implement required language protocol `{}`",
                        self.ctx.types.get(self_ty),
                        trait_name,
                    ))
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
                );
                None
            }
            _ => {
                let trait_name = self.ctx.symbol_table.get_symbol(trait_id).name.clone();
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!(
                        "Multiple implementations of language protocol `{}` match `{:?}`",
                        trait_name,
                        self.ctx.types.get(self_ty),
                    ))
                    .with_code(DiagnosticCode::ConflictingTraitImpl)
                    .with_span(span),
                );
                None
            }
        }
    }

    fn protocol_impl_method(
        &mut self,
        trait_id: luna_common::ids::SymbolId,
        entry: &crate::semantic_tables::TraitImplEntry,
        span: luna_common::Span,
    ) -> Option<luna_common::ids::SymbolId> {
        let required_methods = self
            .ctx
            .tables
            .trait_methods
            .get(&trait_id)
            .cloned()
            .unwrap_or_default();
        if required_methods.len() != 1 {
            self.ctx.diagnostics.push(
                Diagnostic::error("Iterator language protocols must define exactly one required method")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return None;
        }
        let required_name = self
            .ctx
            .symbol_table
            .get_symbol(required_methods[0])
            .name
            .clone();

        if let Some(impl_decl) = entry.decl_id {
            if let Some(Decl::Impl { methods, .. }) = self.arena.decls.get(impl_decl.0 as usize) {
                for method_decl in methods {
                    if let Some(&method_sym) = self.ctx.tables.decl_symbols.get(method_decl) {
                        if self.ctx.symbol_table.get_symbol(method_sym).name == required_name {
                            return Some(method_sym);
                        }
                    }
                }
            }
        }

        let self_key = match self.ctx.types.get(entry.self_type) {
            SemanticType::Struct(sym, ..) | SemanticType::Enum(sym, ..) => {
                Some(crate::semantic_tables::ImplSelfTypeKey::Nominal(*sym))
            }
            SemanticType::Primitive(kind) => {
                Some(crate::semantic_tables::ImplSelfTypeKey::Primitive(*kind))
            }
            _ => None,
        };
        if let Some(self_type_def) = self_key {
            let key = crate::semantic_tables::ImplKey {
                trait_id: Some(trait_id),
                self_type_def,
            };
            if let Some(methods) = self.ctx.tables.impl_methods.get(&key) {
                for &method_sym in methods {
                    if self.ctx.symbol_table.get_symbol(method_sym).name == required_name {
                        return Some(method_sym);
                    }
                }
            }
        }

        self.ctx.diagnostics.push(
            Diagnostic::error(format!(
                "Implementation of language protocol method `{}` is missing",
                required_name,
            ))
            .with_code(DiagnosticCode::UnresolvedTraitImpl)
            .with_span(span),
        );
        None
    }

    fn resolve_for_each_protocol(
        &mut self,
        stmt_id: luna_ast::StmtId,
        iterable_ty: SemanticTypeId,
        pattern: luna_ast::PatId,
        span: luna_common::Span,
    ) {
        use crate::lang_item::LangItem;

        let Some(into_iterator_trait) = self.ctx.lang_items.get(LangItem::IntoIterator) else {
            self.ctx.diagnostics.push(
                Diagnostic::error("language item `into_iterator` is required for `for-in`")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return;
        };
        let Some(iterator_trait) = self.ctx.lang_items.get(LangItem::Iterator) else {
            self.ctx.diagnostics.push(
                Diagnostic::error("language item `iterator` is required for `for-in`")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return;
        };
        let Some(option_sym) = self.ctx.lang_items.get(LangItem::Option) else {
            self.ctx.diagnostics.push(
                Diagnostic::error("language item `option` is required for `for-in`")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return;
        };

        let Some((into_entry, into_subst)) =
            self.select_protocol_impl(into_iterator_trait, iterable_ty, span)
        else {
            return;
        };
        if into_entry.trait_args.len() != 2 {
            self.ctx.diagnostics.push(
                Diagnostic::error("IntoIterator language protocol requires item and iterator type arguments")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return;
        }
        let item_ty = self.ctx.types.subst(into_entry.trait_args[0], &into_subst);
        let iterator_ty = self.ctx.types.subst(into_entry.trait_args[1], &into_subst);
        let Some(into_iter_method) =
            self.protocol_impl_method(into_iterator_trait, &into_entry, span)
        else {
            return;
        };

        let Some((iterator_entry, next_subst)) =
            self.select_protocol_impl(iterator_trait, iterator_ty, span)
        else {
            return;
        };
        if iterator_entry.trait_args.len() != 1 {
            self.ctx.diagnostics.push(
                Diagnostic::error("Iterator language protocol requires one item type argument")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return;
        }
        let next_item_ty = self.ctx.types.subst(iterator_entry.trait_args[0], &next_subst);
        if let Err(error) = self.unify(item_ty, next_item_ty) {
            self.ctx.diagnostics.push(Diagnostic::error(error).with_code(DiagnosticCode::TypeMismatch).with_span(span));
            return;
        }
        let Some(next_method) = self.protocol_impl_method(iterator_trait, &iterator_entry, span)
        else {
            return;
        };

        let Some(&next_fn_ty) = self.ctx.tables.symbol_types.get(&next_method) else {
            self.ctx.diagnostics.push(
                Diagnostic::error("Iterator protocol method has no semantic function type")
                    .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                    .with_span(span),
            );
            return;
        };
        let (next_receiver_type, option_type) =
            match self.ctx.types.get(next_fn_ty).clone() {
                SemanticType::Function { params, return_type, .. } if !params.is_empty() => (
                    self.ctx.types.subst(params[0], &next_subst),
                    self.ctx.types.subst(return_type, &next_subst),
                ),
                _ => {
                    self.ctx.diagnostics.push(
                        Diagnostic::error("Iterator protocol method must have a receiver")
                            .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                            .with_span(span),
                    );
                    return;
                }
            };
        match self.ctx.types.get(option_type).clone() {
            SemanticType::Enum(sym, args, _) if sym == option_sym && args.len() == 1 => {
                if let Err(error) = self.unify(item_ty, args[0]) {
                    self.ctx.diagnostics.push(Diagnostic::error(error).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    return;
                }
            }
            _ => {
                self.ctx.diagnostics.push(
                    Diagnostic::error("Iterator protocol method must return the Option language contract")
                        .with_code(DiagnosticCode::TraitBoundNotSatisfied)
                        .with_span(span),
                );
                return;
            }
        }

        let item_ty = self.ctx.types.resolve(item_ty);
        let iterator_ty = self.ctx.types.resolve(iterator_ty);
        let option_type = self.ctx.types.resolve(option_type);
        self.typecheck_pattern(&pattern, item_ty);
        self.ctx.tables.for_loop_next.insert(stmt_id, next_method);
        self.ctx.tables.for_loop_subst.insert(stmt_id, next_subst.clone());
        self.ctx.tables.for_loop_resolutions.insert(
            stmt_id,
            crate::semantic_tables::ForLoopResolution {
                into_iter_method,
                into_iter_subst: into_subst,
                next_method,
                next_subst,
                iterator_type: iterator_ty,
                item_type: item_ty,
                option_type,
                next_receiver_type,
            },
        );
    }

    fn typecheck_function_body(&mut self, body: &luna_ast::StmtId, expected: SemanticTypeId) {
        self.expected_expr_type.push(expected);
        self.typecheck_stmt(body);
        self.expected_expr_type.pop();
        if let Stmt::Block { tail_expr: Some(tail), .. } = &self.arena.stmts[body.0 as usize] {
            if let Some(&actual) = self.ctx.tables.expr_types.get(tail) {
                // Statement-shaped tails whose branches return have no value;
                // their return legality is checked by the control-flow rules.
                if !matches!(self.ctx.types.get(actual), SemanticType::Void) && !self.try_coerce(*tail, actual, expected) {
                    if let Err(message) = self.unify(expected, actual) {
                        self.ctx.diagnostics.push(Diagnostic::error(message).with_code(DiagnosticCode::TypeMismatch).with_span(self.get_expr_span_for_diag(tail).unwrap_or_default()));
                    }
                }
            }
        }
    }

    fn typecheck_stmt(&mut self, stmt_id: &luna_ast::StmtId) {
        let stmt = &self.arena.stmts[stmt_id.0 as usize];
        match stmt {
            Stmt::Block { body, tail_expr } => {
                // A block's output context belongs to its tail, not unrelated
                // statements or unannotated declarations inside the block.
                let output_context = std::mem::take(&mut self.expected_expr_type);
                self.typecheck_items(body);
                self.expected_expr_type = output_context;
                if let Some(expr) = tail_expr {
                    self.typecheck_expr(expr);
                }
            }
            Stmt::Expr { expr, .. } => {
                self.typecheck_expr(expr);
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let cond_ty = self.typecheck_expr(condition);
                let bool_ty = self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::Bool));
                if self.unify(bool_ty, cond_ty).is_err() {
                    self.ctx.diagnostics.push(Diagnostic::error("if condition must be a boolean").with_code(DiagnosticCode::TypeMismatch));
                }
                self.typecheck_stmt(then_branch);
                if let Some(else_br) = else_branch {
                    self.typecheck_stmt(else_br);
                }
            }
            Stmt::While { condition, body, .. } => {
                let cond_ty = self.typecheck_expr(condition);
                let bool_ty = self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::Bool));
                if self.unify(bool_ty, cond_ty).is_err() {
                    self.ctx.diagnostics.push(Diagnostic::error("while condition must be a boolean").with_code(DiagnosticCode::TypeMismatch));
                }
                self.loop_depth += 1;
                self.typecheck_stmt(body);
                self.loop_depth -= 1;
            }
            Stmt::For { kind, init, cond, step, body, pattern, iterable, .. } => {
                if let Some(item) = init { self.typecheck_item(item); }
                if let Some(c) = cond { 
                    let cond_ty = self.typecheck_expr(c); 
                    let bool_ty = self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::Bool));
                    if self.unify(bool_ty, cond_ty).is_err() {
                        self.ctx.diagnostics.push(Diagnostic::error("for condition must be a boolean").with_code(DiagnosticCode::TypeMismatch));
                    }
                }
                if let Some(s) = step { self.typecheck_expr(s); }
                if let Some(iter) = iterable {
                    let iterable_ty = self.typecheck_expr(iter);
                    if matches!(kind, luna_ast::stmt::ForKind::ForEach)
                        && !matches!(
                            &self.arena.exprs[iter.0 as usize],
                            Expr::Binary {
                                op: luna_ast::expr::BinaryOp::Range | luna_ast::expr::BinaryOp::RangeInc,
                                ..
                            }
                        )
                    {
                        if let Some(pattern) = pattern {
                            let span = self
                                .get_expr_span_for_diag(iter)
                                .unwrap_or_else(luna_common::Span::default);
                            self.resolve_for_each_protocol(*stmt_id, iterable_ty, *pattern, span);
                        }
                    }
                }
                self.loop_depth += 1;
                self.typecheck_stmt(body);
                self.loop_depth -= 1;
            }
            Stmt::Return { value } => {
                let expected_ty = self.current_return_type.last().copied().unwrap_or_else(|| self.ctx.types.intern(SemanticType::Void));
                if let Some(val) = value {
                    self.expected_expr_type.push(expected_ty);
                    let val_ty = self.typecheck_expr(val);
                    self.expected_expr_type.pop();
                    if !self.try_coerce(*val, val_ty, expected_ty) {
                        if let Err(e) = self.unify(expected_ty, val_ty) {
                            let span = self.get_expr_span_for_diag(val).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                        }
                    }
                } else {
                    let void_ty = self.ctx.types.intern(SemanticType::Void);
                    if let Err(e) = self.unify(expected_ty, void_ty) {
                        self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0)));
                    }
                }
            }
            Stmt::Unsafe { body } => {
                let old = self.is_unsafe_context;
                self.is_unsafe_context = true;
                self.typecheck_stmt(body);
                self.is_unsafe_context = old;
            }
            Stmt::Break { span, .. } => {
                if self.loop_depth == 0 {
                    self.ctx.diagnostics.push(
                        Diagnostic::error("`break` outside of a loop")
                            .with_code(DiagnosticCode::LoopControlOutsideLoop)
                            .with_span(*span),
                    );
                }
            }
            Stmt::Continue { span, .. } => {
                if self.loop_depth == 0 {
                    self.ctx.diagnostics.push(
                        Diagnostic::error("`continue` outside of a loop")
                            .with_code(DiagnosticCode::LoopControlOutsideLoop)
                            .with_span(*span),
                    );
                }
            }
            other => {
                self.ctx.diagnostics.push(Diagnostic::error(format!("Unsupported or unrecognized statement construct in semantic phase: {:?}", other)).with_code(DiagnosticCode::InvalidSyntax));
            }
        }
    }

    fn resolve_type(&mut self, ty_id: luna_ast::TypeId) -> SemanticTypeId {
        self.ctx.tables.ast_type_to_semantic.get(&ty_id).copied().unwrap_or_else(|| self.ctx.types.new_inference_var())
    }

    fn bind_matching_generics(
        &self,
        expected: SemanticTypeId,
        actual: SemanticTypeId,
        subst: &mut crate::ty::Substitution,
        has_generics: &mut bool,
    ) {
        let e_ty = self.ctx.types.get(expected).clone();
        let a_ty = self.ctx.types.get(actual).clone();
        match (e_ty, a_ty) {
            (SemanticType::GenericParam(gp), _) => {
                subst.insert(gp, actual);
                *has_generics = true;
            }
            (SemanticType::Pointer(mut1, in1), SemanticType::Pointer(mut2, in2)) if mut1 == mut2 => {
                self.bind_matching_generics(in1, in2, subst, has_generics);
            }
            (SemanticType::Reference(_, mut1, in1), SemanticType::Reference(_, mut2, in2)) if mut1 == mut2 => {
                self.bind_matching_generics(in1, in2, subst, has_generics);
            }
            (SemanticType::Reference(_, _, in1), in2) => {
                self.bind_matching_generics(in1, actual, subst, has_generics);
            }
            (in1, SemanticType::Reference(_, _, in2)) => {
                self.bind_matching_generics(expected, in2, subst, has_generics);
            }
            (SemanticType::Struct(s1, args1, _), SemanticType::Struct(s2, args2, _)) if s1 == s2 => {
                for (&arg1, &arg2) in args1.iter().zip(args2.iter()) {
                    self.bind_matching_generics(arg1, arg2, subst, has_generics);
                }
            }
            (SemanticType::Enum(e1, args1, _), SemanticType::Enum(e2, args2, _)) if e1 == e2 => {
                for (&arg1, &arg2) in args1.iter().zip(args2.iter()) {
                    self.bind_matching_generics(arg1, arg2, subst, has_generics);
                }
            }
            _ => {}
        }
    }

    fn typecheck_expr_expected(&mut self, expr: &luna_ast::ExprId, expected: SemanticTypeId) -> SemanticTypeId {
        self.expected_expr_type.push(expected);
        let result = self.typecheck_expr(expr);
        self.expected_expr_type.pop();
        result
    }

    fn typecheck_expr_without_expected(&mut self, expr: &luna_ast::ExprId) -> SemanticTypeId {
        let parent_context = std::mem::take(&mut self.expected_expr_type);
        let result = self.typecheck_expr(expr);
        self.expected_expr_type = parent_context;
        result
    }

    fn typecheck_integer_literal(&mut self, expr: luna_ast::ExprId, text: &str, negative: bool) -> SemanticTypeId {
        let span = self.get_expr_span_for_diag(&expr).unwrap_or_default();
        let ty = self.typecheck_integer_literal_at(span, text, negative);
        self.ctx.tables.expr_types.insert(expr, ty);
        ty
    }

    fn typecheck_integer_literal_at(&mut self, span: luna_common::Span, text: &str, negative: bool) -> SemanticTypeId {
        let decoded = match luna_lexer::literal::decode_integer(text) {
            Ok(value) => value,
            Err(message) => {
                self.ctx.diagnostics.push(Diagnostic::error(format!("E_INVALID_INTEGER_LITERAL: {message}")).with_code(DiagnosticCode::InvalidSyntax).with_span(span));
                return self.ctx.types.error_id();
            }
        };
        let explicit = decoded.suffix.map(|kind| match kind {
            BuiltinKind::I8 => BuiltinType::I8, BuiltinKind::I16 => BuiltinType::I16,
            BuiltinKind::I32 => BuiltinType::I32, BuiltinKind::I64 => BuiltinType::I64,
            BuiltinKind::I128 => BuiltinType::I128, BuiltinKind::Isize => BuiltinType::Isize,
            BuiltinKind::U8 => BuiltinType::U8, BuiltinKind::U16 => BuiltinType::U16,
            BuiltinKind::U32 => BuiltinType::U32, BuiltinKind::U64 => BuiltinType::U64,
            BuiltinKind::U128 => BuiltinType::U128, BuiltinKind::Usize => BuiltinType::Usize,
            _ => unreachable!("decoder only returns integer suffixes"),
        });
        let contextual = self.expected_expr_type.last().and_then(|&ty| {
            match self.ctx.types.get(self.ctx.types.resolve_inference(ty)) {
                SemanticType::Primitive(b) if b.is_integer() => Some(*b),
                _ => None,
            }
        });
        let builtin = explicit.or(contextual).unwrap_or(BuiltinType::I32);
        let width = crate::IntWidth::from_builtin(builtin);
        let bits = if matches!(builtin, BuiltinType::Isize | BuiltinType::Usize) {
            self.ctx.target_pointer_bits
        } else { width.bit_width() };
        if !luna_lexer::literal::magnitude_fits(decoded.magnitude, negative, bits, width.is_signed()) {
            self.ctx.diagnostics.push(Diagnostic::error(format!("E_INTEGER_LITERAL_RANGE: literal `{}` is outside {:?}", text, builtin)).with_code(DiagnosticCode::TypeMismatch).with_span(span));
            return self.ctx.types.error_id();
        }
        self.ctx.types.intern(SemanticType::Primitive(builtin))
    }

    fn typecheck_expr(&mut self, expr_id: &luna_ast::ExprId) -> SemanticTypeId {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        let ty_id = match expr {
            Expr::Literal(tok, text) => {
                // Determine type based on literal token type
                let kind = match tok.kind {
                    TokenKind::IntegerLiteral => return self.typecheck_integer_literal(*expr_id, text, false),
                    TokenKind::ByteLiteral | TokenKind::ByteStringLiteral => {
                        match luna_lexer::literal::decode_bytes(text, tok.kind == TokenKind::ByteLiteral) {
                            Ok(bytes) => {
                                let byte_ty = self.ctx.types.intern(SemanticType::Primitive(BuiltinType::U8));
                                let ty = if tok.kind == TokenKind::ByteLiteral { byte_ty } else {
                                    self.ctx.types.intern(SemanticType::Array(byte_ty, bytes.len() as u64))
                                };
                                self.ctx.tables.expr_types.insert(*expr_id, ty);
                                return ty;
                            }
                            Err(message) => {
                                self.ctx.diagnostics.push(Diagnostic::error(format!("E_INVALID_BYTE_LITERAL: {message}")).with_code(DiagnosticCode::InvalidSyntax).with_span(tok.span));
                                return self.ctx.types.error_id();
                            }
                        }
                    }
                    TokenKind::FloatLiteral => {
                        if text.ends_with("f32") {
                            SemanticType::Primitive(BuiltinType::F32)
                        } else if text.ends_with("f64") {
                            SemanticType::Primitive(BuiltinType::F64)
                        } else {
                            SemanticType::Primitive(BuiltinType::F64) // Default to f64
                        }
                    }
                    TokenKind::StringLiteral => SemanticType::Primitive(BuiltinType::String),
                    TokenKind::CharLiteral => SemanticType::Primitive(BuiltinType::Char),
                    TokenKind::KwTrue | TokenKind::KwFalse => SemanticType::Primitive(BuiltinType::Bool),
                    _ => {
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Unrecognized literal token '{:?}'", tok.kind)).with_code(DiagnosticCode::InvalidSyntax).with_span(tok.span));
                        SemanticType::Error
                    },
                };
                self.ctx.types.intern(kind)
            }
            Expr::Identifier { generic_args, .. } => {
                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(expr_id) {
                    let mut base_ty = None;
                    if let Some(ty) = self.ctx.tables.symbol_types.get(sym_id) {
                        base_ty = Some(*ty);
                    } else {
                        let symbol = self.ctx.symbol_table.get_symbol(*sym_id);
                        if let crate::SymbolKind::EnumVariant(_) = symbol.kind {
                            if let Some(decl_id) = symbol.decl_id {
                                if let Some(enum_sym_id) = self.ctx.tables.decl_symbols.get(&decl_id) {
                                    if let Some(ty) = self.ctx.tables.symbol_types.get(enum_sym_id) {
                                        base_ty = Some(*ty);
                                    }
                                }
                            }
                        }
                    }
                    if let Some(ty) = base_ty {
                        if generic_args.is_empty() {
                            let sym_kind = self.ctx.symbol_table.get_symbol(*sym_id).kind.clone();
                            if matches!(sym_kind, crate::SymbolKind::EnumVariant(_) | crate::SymbolKind::Enum) {
                                let resolved = self.ctx.types.get(ty).clone();
                                match resolved {
                                    SemanticType::Enum(e_sym, enum_args, original_var_tys) => {
                                        let mut has_generic_param = false;
                                        let mut subst = crate::ty::Substitution::new();
                                        let mut new_args = Vec::new();
                                        for arg in &enum_args {
                                            if let SemanticType::GenericParam(gp) = self.ctx.types.get(*arg).clone() {
                                                has_generic_param = true;
                                                let ivar = self.ctx.types.new_inference_var();
                                                subst.insert(gp, ivar);
                                                new_args.push(ivar);
                                            } else {
                                                new_args.push(*arg);
                                            }
                                        }
                                        if has_generic_param {
                                            let mut variant_tys = Vec::new();
                                            for orig_ty in original_var_tys {
                                                variant_tys.push(self.ctx.types.subst(orig_ty, &subst));
                                            }
                                            self.ctx.types.intern(SemanticType::Enum(e_sym, new_args, variant_tys))
                                        } else {
                                            ty
                                        }
                                    }
                                    _ => ty,
                                }
                            } else {
                                ty
                            }
                        } else {
                            // Substitute generic arguments
                            let symbol = self.ctx.symbol_table.get_symbol(*sym_id);
                            let decl_id_opt = symbol.decl_id.or_else(|| {
                                if let crate::SymbolKind::EnumVariant(_) = symbol.kind {
                                    symbol.decl_id
                                } else {
                                    None
                                }
                            });
                            
                            if let Some(decl_id) = decl_id_opt {
                                let mut concrete_args = Vec::new();
                                for arg in generic_args {
                                    concrete_args.push(self.lower_type(*arg));
                                }
                                let mut subst = crate::ty::Substitution::new();
                                for (idx, _) in generic_args.iter().enumerate() {
                                    if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(decl_id, idx)) {
                                        subst.insert(*gp_sym, concrete_args[idx]);
                                    }
                                }
                                if subst.map.is_empty() {
                                    if let SemanticType::Function { return_type, .. } = self.ctx.types.get(ty).clone() {
                                        if let SemanticType::Struct(_, ref m_args, _) = self.ctx.types.get(return_type).clone() {
                                            for (idx, &arg_ty) in m_args.iter().enumerate() {
                                                if let SemanticType::GenericParam(gp) = self.ctx.types.get(arg_ty) {
                                                    if let Some(&c_arg) = concrete_args.get(idx) {
                                                        subst.insert(*gp, c_arg);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                let resolved_ty = self.ctx.types.get(ty).clone();
                                match resolved_ty {
                                    SemanticType::Struct(s_sym, _, original_field_tys) => {
                                        let mut field_tys = Vec::new();
                                        for orig_ty in original_field_tys {
                                            field_tys.push(self.ctx.types.subst(orig_ty, &subst));
                                        }
                                        self.ctx.types.intern(SemanticType::Struct(s_sym, concrete_args, field_tys))
                                    }
                                    SemanticType::Enum(e_sym, _, original_var_tys) => {
                                        let mut variant_tys = Vec::new();
                                        for orig_ty in original_var_tys {
                                            variant_tys.push(self.ctx.types.subst(orig_ty, &subst));
                                        }
                                        self.ctx.types.intern(SemanticType::Enum(e_sym, concrete_args, variant_tys))
                                    }
                                    SemanticType::Function { params, return_type, is_unsafe } => {
                                        let mut new_params = Vec::new();
                                        for p in params {
                                            new_params.push(self.ctx.types.subst(p, &subst));
                                        }
                                        let new_ret = self.ctx.types.subst(return_type, &subst);
                                        if !subst.map.is_empty() {
                                            self.ctx.tables.expr_substs.insert(*expr_id, subst.clone());
                                        }
                                        self.ctx.types.intern(SemanticType::Function { params: new_params, return_type: new_ret, is_unsafe })
                                    }
                                    _ => self.ctx.types.subst(ty, &subst),
                                }
                            } else {
                                ty
                            }
                        }
                    } else {
                        self.ctx.types.new_inference_var()
                    }
                } else {
                    self.ctx.types.intern(SemanticType::Error)
                }
            }
            Expr::Binary { op, left, right, .. } => {
                // A comparison's bool result is not the type of its operands.
                // In particular it must not constrain a generic method result.
                let comparison = matches!(op, BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge);
                let left_is_literal = matches!(&self.arena.exprs[left.0 as usize], Expr::Literal(tok, _) if tok.kind == TokenKind::IntegerLiteral)
                    || matches!(&self.arena.exprs[left.0 as usize], Expr::Unary { op: luna_ast::expr::UnaryOp::Neg, operand } if matches!(&self.arena.exprs[operand.0 as usize], Expr::Literal(tok, _) if tok.kind == TokenKind::IntegerLiteral));
                let right_is_literal = matches!(&self.arena.exprs[right.0 as usize], Expr::Literal(tok, _) if tok.kind == TokenKind::IntegerLiteral);
                let (l_ty, r_ty) = if left_is_literal && !right_is_literal {
                    let right_ty = if comparison { self.typecheck_expr_without_expected(right) } else { self.typecheck_expr(right) };
                    (self.typecheck_expr_expected(left, right_ty), right_ty)
                } else {
                    let left_ty = if comparison { self.typecheck_expr_without_expected(left) } else { self.typecheck_expr(left) };
                    (left_ty, self.typecheck_expr_expected(right, left_ty))
                };
                
                // Coerce pointer/int comparisons if applicable
                let l_is_ptr = matches!(self.ctx.types.get(l_ty), SemanticType::Pointer(_, _));
                let r_is_ptr = matches!(self.ctx.types.get(r_ty), SemanticType::Pointer(_, _));
                let l_is_int = matches!(self.ctx.types.get(l_ty), SemanticType::Primitive(crate::ty::BuiltinType::I64 | crate::ty::BuiltinType::U64));
                let r_is_int = matches!(self.ctx.types.get(r_ty), SemanticType::Primitive(crate::ty::BuiltinType::I64 | crate::ty::BuiltinType::U64));
                
                if (l_is_ptr && r_is_int) || (r_is_ptr && l_is_int) {
                    let ptr_result_ty = self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::Bool));
                    if matches!(op, BinaryOp::Eq | BinaryOp::Ne) {
                        return ptr_result_ty;
                    }
                }
                
                let l_sem_ty = self.ctx.types.get(l_ty).clone();
                let r_sem_ty = self.ctx.types.get(r_ty).clone();
                
                if let SemanticType::Pointer(_, elem_l) = l_sem_ty {
                    use luna_ast::expr::BinaryOp;
                    if matches!(op, BinaryOp::Add | BinaryOp::Sub) {
                        // G1 Vector 1: Reject pointer arithmetic on unsized pointee types.
                        // sizeof(dyn Trait) and sizeof([T]) are not known at compile-time,
                        // so GEP stride cannot be computed. Must be caught here, not in LLVM.
                        if self.ctx.types.is_unsized(elem_l) {
                            let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error("E_UNSIZED_POINTER_ARITHMETIC: Cannot perform pointer arithmetic on unsized pointee type (dyn Trait or [T] has no compile-time size)")
                                .with_code(DiagnosticCode::TypeMismatch)
                                .with_span(span));
                            self.ctx.tables.expr_types.insert(*expr_id, self.ctx.types.intern(SemanticType::Error));
                            return self.ctx.types.intern(SemanticType::Error);
                        }
                        if !self.is_unsafe_context {
                            let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error("E_POINTER_ARITHMETIC_OUTSIDE_UNSAFE: Pointer arithmetic requires an unsafe block")
                                .with_code(DiagnosticCode::UnsafeOperationOutsideUnsafe)
                                .with_span(span));
                        }
                        let ptr_result_ty = match op {
                            BinaryOp::Add => l_ty,
                            BinaryOp::Sub => {
                                if let SemanticType::Pointer(_, elem_r) = r_sem_ty {
                                    if elem_l != elem_r {
                                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                        self.ctx.diagnostics.push(Diagnostic::error("Cannot subtract pointers to different pointee types").with_code(DiagnosticCode::TypeMismatch).with_span(span));
                                    }
                                    self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::Isize))
                                } else {
                                    l_ty
                                }
                            }
                            _ => unreachable!(),
                        };
                        self.ctx.tables.expr_types.insert(*expr_id, ptr_result_ty);
                        return ptr_result_ty;
                    }
                }
                
                // For simplified logic: require left and right to be same
                if let Err(e) = self.unify(l_ty, r_ty) {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                } 
                
                if let Some(message) = crate::operators::PrimitiveOperator::Binary(*op)
                    .error_for_type(&self.ctx.types, l_ty)
                {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error(message).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    return self.ctx.types.error_id();
                }

                use luna_ast::expr::BinaryOp;
                match op {
                    BinaryOp::Eq | BinaryOp::Ne | BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge | BinaryOp::LogicAnd | BinaryOp::LogicOr => {
                        self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::Bool))
                    }
                    BinaryOp::Range | BinaryOp::RangeInc => {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.ctx.diagnostics.push(Diagnostic::error(format!("E_UNSUPPORTED_FEATURE: Binary operator `{:?}` is not yet supported in the backend", op)).with_code(DiagnosticCode::InvalidBinaryOp).with_span(span));
                        self.ctx.types.intern(SemanticType::Error)
                    }
                    _ => l_ty
                }
            }
            Expr::Call { callee, generic_args: call_generic_args, args } => {
                let callee_expr = &self.arena.exprs[callee.0 as usize];
                // the string name check for drop was removed from here
                
                let callee_ty_id = self.typecheck_expr(callee);
                let mut ret_ty_id = self.ctx.types.new_inference_var();
                let callee_ty = self.ctx.types.get(callee_ty_id).clone();
                let is_dyn_call = self.ctx.tables.dyn_method_indices.contains_key(callee);
                let is_struct_method = if let Expr::Member { .. } = callee_expr { self.ctx.tables.expr_symbols.contains_key(callee) } else { false };
                // Receiver identity comes from the declaration, never from the
                // number of arguments supplied by a potentially invalid call.
                // A callable struct field has no implicit method receiver.
                let receiver_declared = self.ctx.tables.expr_symbols.get(callee)
                    .and_then(|symbol| self.ctx.tables.symbol_decls.get(symbol))
                    .and_then(|decl| self.arena.decls.get(decl.0 as usize))
                    .and_then(|decl| match decl {
                        Decl::Function { params, .. } => params.first(),
                        _ => None,
                    })
                    .and_then(|decl| self.arena.decls.get(decl.0 as usize))
                    .is_some_and(|decl| matches!(decl, Decl::Param { is_self: true, .. }));
                let is_method_call = is_dyn_call || (is_struct_method && receiver_declared);
                let mut subst = crate::ty::Substitution::new();
                
                if let Some(&callee_sym) = self.ctx.tables.expr_symbols.get(callee) {
                    if Some(callee_sym) == self.ctx.lang_items.get(crate::lang_item::LangItem::DropFn) {
                        let diag = Diagnostic::error("Explicit calls to drop() are forbidden. Values are dropped automatically at end of scope.".to_string()).with_code(DiagnosticCode::TypeMismatch);
                        let diag = if let Some(span) = self.get_expr_span_for_diag(callee) { diag.with_span(span) } else { diag };
                        self.ctx.diagnostics.push(diag);
                    }
                }
                
                let mut has_generics = false;
                let mut func_sym_opt = None;

                let id_gen_args = if let Expr::Identifier { generic_args, .. } = callee_expr {
                    generic_args.as_slice()
                } else {
                    &[]
                };

                let effective_gen_args: &[luna_ast::TypeId] = if !call_generic_args.is_empty() {
                    call_generic_args.as_slice()
                } else {
                    id_gen_args
                };

                if let Some(&callee_sym) = self.ctx.tables.expr_symbols.get(callee) {
                    func_sym_opt = Some(callee_sym);
                    if let Some(&callee_decl_id) = self.ctx.tables.symbol_decls.get(&callee_sym) {
                        if !effective_gen_args.is_empty() {
                            let mut concrete_args = Vec::new();
                            for arg in effective_gen_args {
                                concrete_args.push(self.lower_type(*arg));
                            }
                            for (gp_idx, &c_arg) in concrete_args.iter().enumerate() {
                                if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(callee_decl_id, gp_idx)) {
                                    has_generics = true;
                                    subst.insert(*gp_sym, c_arg);
                                }
                            }
                            if subst.map.is_empty() {
                                if let SemanticType::Function { return_type, .. } = callee_ty {
                                    if let Some(&c_arg) = concrete_args.first() {
                                        self.bind_matching_generics(return_type, c_arg, &mut subst, &mut has_generics);
                                    }
                                }
                            }
                        } else {
                            if (callee_decl_id.0 as usize) < self.arena.decls.len() {
                                if let Decl::Function { generic_params: m_gps, .. } = &self.arena.decls[callee_decl_id.0 as usize] {
                                    has_generics = !m_gps.is_empty();
                                    for (gp_idx, _) in m_gps.iter().enumerate() {
                                        if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(callee_decl_id, gp_idx)) {
                                            subst.insert(*gp_sym, self.ctx.types.new_inference_var());
                                        }
                                    }
                                }
                            } else {
                                let mut gp_idx = 0;
                                while let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(callee_decl_id, gp_idx)) {
                                    has_generics = true;
                                    subst.insert(*gp_sym, self.ctx.types.new_inference_var());
                                    gp_idx += 1;
                                }
                            }
                        }
                    }
                    let callee_trait_sym = self.ctx.tables.trait_methods.iter()
                        .find(|(_, meths)| meths.contains(&callee_sym))
                        .map(|(&t, _)| t);
                    if let Some(trait_sym) = callee_trait_sym {
                        if !is_method_call && !subst.map.contains_key(&trait_sym) {
                            subst.insert(trait_sym, self.ctx.types.new_inference_var());
                            has_generics = true;
                        }
                        if let Some(trait_gps) = self.ctx.tables.trait_generic_params.get(&trait_sym) {
                            for gp_sym in trait_gps {
                                if !subst.map.contains_key(gp_sym) {
                                    subst.insert(*gp_sym, self.ctx.types.new_inference_var());
                                    has_generics = true;
                                }
                            }
                        }
                    }
                }

                let is_callee_unsafe = matches!(callee_ty, SemanticType::Function { is_unsafe: true, .. })
                    || if let Some(&callee_sym) = self.ctx.tables.expr_symbols.get(callee) {
                        let sym = self.ctx.symbol_table.get_symbol(callee_sym);
                        self.ctx.tables.unsafe_functions.contains(&callee_sym) || matches!(sym.kind, crate::symbol::SymbolKind::ExternFunction)
                    } else {
                        false
                    };

                if is_callee_unsafe && !self.is_unsafe_context {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error("E_CALL_UNSAFE_FN_OUTSIDE_UNSAFE: call to unsafe function requires an unsafe block")
                        .with_code(DiagnosticCode::UnsafeOperationOutsideUnsafe)
                        .with_span(span));
                }

                if let SemanticType::Function { params, return_type, .. } | SemanticType::Closure(_, params, return_type) = callee_ty {
                    let is_variadic = func_sym_opt
                        .and_then(|symbol| self.ctx.tables.symbol_decls.get(&symbol))
                        .and_then(|decl| self.arena.decls.get(decl.0 as usize))
                        .is_some_and(|decl| matches!(decl, Decl::Function { is_variadic: true, .. }));
                    let required = params.len().saturating_sub(usize::from(is_method_call));
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or_default();
                    let Some(binding) = self.bind_call_arguments(*expr_id, func_sym_opt, is_method_call,
                        required, is_variadic, args, span) else {
                        for arg in args { self.typecheck_expr(&arg.value); }
                        return self.ctx.types.error_id();
                    };
                    ret_ty_id = if has_generics { self.ctx.types.subst(return_type, &subst) } else { return_type };
                    if has_generics {
                        if let Some(expected) = self.expected_expr_type.last().copied() {
                            if !matches!(self.ctx.types.get(expected), SemanticType::Error) {
                                let _ = self.unify(ret_ty_id, expected);
                            }
                        }
                    }
                    let expected_params = if is_method_call {
                        &params[1..]
                    } else {
                        &params[..]
                    };
                    for (i, arg) in args.iter().enumerate() {
                        let parameter = binding.source_to_parameter[i] as usize;
                        let expected = expected_params.get(parameter).map(|&ty| if has_generics { self.ctx.types.subst(ty, &subst) } else { ty });
                        let arg_ty = if let Some(expected) = expected {
                            self.typecheck_expr_expected(&arg.value, expected)
                        } else { self.typecheck_expr(&arg.value) };
                        if let Some(&expected_p) = expected_params.get(parameter) {
                            let expected_p = if has_generics { self.ctx.types.subst(expected_p, &subst) } else { expected_p };
                            if !self.try_coerce(arg.value, arg_ty, expected_p) {
                                if let Err(e) = self.unify(expected_p, arg_ty) {
                                    let span = self.get_expr_span_for_diag(&arg.value).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                    self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                                }
                            }
                        }
                    }
                    if has_generics {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        if let Some(func_sym) = func_sym_opt {
                            self.check_bounds_for_call(func_sym, &subst, span);
                            let owner = self.ctx.tables.trait_methods.iter()
                                .find(|(_, methods)| methods.contains(&func_sym)).map(|(&owner, _)| owner);
                            if let Some(owner) = owner {
                                if let Some(&receiver) = subst.get(owner) {
                                    let parameters = self.ctx.tables.trait_generic_params.get(&owner).cloned().unwrap_or_default();
                                    let arguments: Vec<_> = parameters.into_iter().map(|parameter| {
                                        let parameter = self.ctx.types.intern(SemanticType::GenericParam(parameter));
                                        self.ctx.types.subst(parameter, &subst)
                                    }).collect();
                                    if !self.prove_trait_bound(receiver, owner, &arguments) {
                                        self.ctx.diagnostics.push(Diagnostic::error(format!(
                                            "Qualified call requires an applicable implementation of `{}` with satisfied bounds",
                                            self.ctx.symbol_table.get_symbol(owner).name))
                                            .with_code(DiagnosticCode::TraitBoundNotSatisfied).with_span(span));
                                    }
                                }
                            }
                        }
                        
                        let empty = crate::ty::Substitution::new();
                        let mut resolved_subst = crate::ty::Substitution::new();
                        for (sym, ty) in subst.map {
                            resolved_subst.insert(sym, self.ctx.types.subst(ty, &empty));
                        }
                        if !resolved_subst.map.is_empty() {
                            self.ctx.tables.expr_substs.insert(*expr_id, resolved_subst.clone());
                            ret_ty_id = self.ctx.types.subst(return_type, &resolved_subst);
                        }
                    }
                } else if let SemanticType::Enum(enum_sym_id, enum_args, variants) = callee_ty {
                    if let Some(label) = args.iter().find_map(|argument| argument.label) {
                        self.ctx.diagnostics.push(Diagnostic::error("Enum constructors do not expose named callable parameters")
                            .with_code(DiagnosticCode::TypeMismatch).with_span(label));
                        return self.ctx.types.error_id();
                    }
                    if let Some(func_sym) = func_sym_opt {
                        if let crate::SymbolKind::EnumVariant(variant_idx) = self.ctx.symbol_table.get_symbol(func_sym).kind {
                            let mut enum_subst = crate::ty::Substitution::new();
                            let mut has_enum_generics = false;
                            let mut new_args = Vec::new();
                            
                            has_enum_generics = !enum_args.is_empty();
                            for arg_ty in &enum_args {
                                if let SemanticType::GenericParam(gp_sym) = self.ctx.types.get(*arg_ty).clone() {
                                    let ivar = self.ctx.types.new_inference_var();
                                    enum_subst.insert(gp_sym, ivar);
                                    new_args.push(ivar);
                                } else {
                                    new_args.push(*arg_ty);
                                }
                            }


                            if has_enum_generics {
                                if let Some(expected) = self.expected_expr_type.last().copied() {
                                    let instantiated = self.ctx.types.subst(callee_ty_id, &enum_subst);
                                    let _ = self.unify(instantiated, expected);
                                }
                            }
                            let mut variant_ty = variants[variant_idx as usize];
                            if has_enum_generics {
                                variant_ty = self.ctx.types.subst(variant_ty, &enum_subst);
                            }
                            
                            // Payload arity belongs to the declaration, not its
                            // instantiated type: One<T> remains one argument
                            // when T becomes a tuple or void.
                            let field_count = self.ctx.symbol_table.get_symbol(func_sym).decl_id
                                .and_then(|decl| match &self.arena.decls[decl.0 as usize] {
                                    Decl::Enum { variants, .. } => variants.get(variant_idx as usize).map(|variant| variant.fields.len()),
                                    _ => None,
                                });
                            let field_tys = match field_count {
                                Some(0) => Vec::new(),
                                Some(1) => vec![variant_ty],
                                _ => match self.ctx.types.get(variant_ty).clone() {
                                    SemanticType::Tuple(fields) => fields,
                                    SemanticType::Void => Vec::new(),
                                    _ => vec![variant_ty],
                                },
                            };
                            if args.len() != field_tys.len() {
                                let span = self.get_expr_span_for_diag(expr_id).unwrap_or_default();
                                self.ctx.diagnostics.push(Diagnostic::error(format!("Enum variant expects {} fields, but {} were provided", field_tys.len(), args.len())).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                            }
                            {
                                for (i, arg) in args.iter().enumerate() {
                                    let arg_ty = field_tys.get(i).map(|&expected| self.typecheck_expr_expected(&arg.value, expected))
                                        .unwrap_or_else(|| self.typecheck_expr(&arg.value));
                                    if let Some(&expected_p) = field_tys.get(i) {
                                        if !self.try_coerce(arg.value, arg_ty, expected_p) {
                                            if let Err(e) = self.unify(expected_p, arg_ty) {
                                                let span = self.get_expr_span_for_diag(&arg.value).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                                self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                                            }
                                        }
                                    }
                                }
                            }
                            
                            ret_ty_id = if has_enum_generics {
                                let new_variants: Vec<_> = variants.iter().map(|&v| self.ctx.types.subst(v, &enum_subst)).collect();
                                self.ctx.types.intern(SemanticType::Enum(enum_sym_id, new_args, new_variants))
                            } else {
                                callee_ty_id
                            };
                            
                            if has_enum_generics {
                                let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                self.check_bounds_for_call(enum_sym_id, &enum_subst, span);
                                
                                let empty = crate::ty::Substitution::new();
                                let mut resolved_subst = crate::ty::Substitution::new();
                                for (sym, ty) in enum_subst.map {
                                    resolved_subst.insert(sym, self.ctx.types.subst(ty, &empty));
                                }
                                if !resolved_subst.map.is_empty() {
                                    self.ctx.tables.expr_substs.insert(*expr_id, resolved_subst);
                                }
                            }
                        }
                    }
                } else {
                    for arg in args {
                        self.typecheck_expr(&arg.value);
                    }
                }

                ret_ty_id
            }
            Expr::Assign { op, lvalue, value } => {
                let lvalue_expr = &self.arena.exprs[lvalue.0 as usize];
                if !Self::is_place(lvalue_expr) {
                    let span = self.get_expr_span_for_diag(lvalue).unwrap_or_else(|| self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0)));
                    self.ctx.diagnostics.push(Diagnostic::error(
                        "E_INVALID_LVALUE: Left-hand side of assignment must be an assignable place"
                    ).with_code(DiagnosticCode::InvalidLvalue).with_span(span));
                    self.typecheck_expr(value);
                    return self.ctx.types.error_id();
                }

                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(lvalue).copied() {
                    for lambda_id in &self.active_lambdas {
                        if self.ctx.tables.expr_captures.get(lambda_id).map_or(false, |caps| caps.contains(&sym_id)) {
                            self.ctx.tables.closure_mutated_captures.entry(*lambda_id).or_default().insert(sym_id);
                        }
                    }
                }

                if let Expr::Unary { op, operand } = lvalue_expr {
                    if matches!(op, luna_ast::expr::UnaryOp::Deref | luna_ast::expr::UnaryOp::DerefMut) {
                        let op_ty = self.typecheck_expr(operand);
                        if let SemanticType::Pointer(mutability, _) = self.ctx.types.get(op_ty) {
                            if *mutability == crate::ty::Mutability::Immutable {
                                let span = self.get_expr_span_for_diag(lvalue).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                self.ctx.diagnostics.push(Diagnostic::error("E_CANNOT_MUTATE_IMMUTABLE_POINTER: Cannot assign through immutable raw pointer '*T'")
                                    .with_code(DiagnosticCode::CannotMutateImmutable)
                                    .with_span(span));
                            }
                        }
                    }
                }

                let l_ty = self.typecheck_expr(lvalue);
                let r_ty = self.typecheck_expr_expected(value, l_ty);
                self.enforce_mutability(lvalue);
                if let Some(message) = crate::operators::PrimitiveOperator::Assignment(*op)
                    .error_for_type(&self.ctx.types, l_ty)
                {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error(message).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    return self.ctx.types.error_id();
                }
                if !self.try_coerce(*value, r_ty, l_ty) {
                    if let Err(e) = self.unify(l_ty, r_ty) {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    }
                }
                self.ctx.types.intern(SemanticType::Void)
            }
            Expr::Member { object, member } => {
                let obj_ty_id = self.typecheck_expr(object);
                let obj_ty_id = self.ctx.types.resolve_inference(obj_ty_id);
                let obj_ty = self.ctx.types.get(obj_ty_id).clone();
                let member_name = self.get_span_text(*member);
                
                // 1. Check if obj_ty is DynTrait or pointer/ref to DynTrait
                let dyn_trait_sym = match &obj_ty {
                    SemanticType::DynTrait(sym) => Some(*sym),
                    SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => {
                        if let SemanticType::DynTrait(sym) = self.ctx.types.get(self.ctx.types.resolve_inference(*inner)) {
                            Some(*sym)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(trait_sym) = dyn_trait_sym {
                    if let Some(method_syms) = self.ctx.tables.trait_methods.get(&trait_sym) {
                        for (idx, &m_sym) in method_syms.iter().enumerate() {
                            let sym = self.ctx.symbol_table.get_symbol(m_sym);
                            if sym.name == member_name {
                                self.ctx.tables.dyn_method_indices.insert(*expr_id, idx as u32);
                                if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                    self.ctx.tables.expr_types.insert(*expr_id, m_ty);
                                    return m_ty;
                                }
                            }
                        }
                    }
                    self.ctx.diagnostics.push(Diagnostic::error(format!("Trait does not contain method '{}'", member_name)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(*member));
                    let infer = self.ctx.types.new_inference_var();
                    self.ctx.tables.expr_types.insert(*expr_id, infer);
                    return infer;
                }
                
                let peeled_ty = match &obj_ty {
                    SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => {
                        self.ctx.types.get(self.ctx.types.resolve_inference(*inner)).clone()
                    }
                    _ => obj_ty.clone(),
                };
                
                if let SemanticType::Slice(elem) = peeled_ty {
                    if member_name == "length" || member_name == "len" {
                        self.ctx.tables.expr_member_indices.insert(*expr_id, 1);
                        let f_ty = self.ctx.types.intern(SemanticType::Primitive(BuiltinType::Usize));
                        self.ctx.tables.expr_types.insert(*expr_id, f_ty);
                        return f_ty;
                    } else if member_name == "ptr" || member_name == "data" {
                        self.ctx.tables.expr_member_indices.insert(*expr_id, 0);
                        let ptr_ty = self.ctx.types.intern(SemanticType::Pointer(crate::ty::Mutability::Mutable, elem));
                        self.ctx.tables.expr_types.insert(*expr_id, ptr_ty);
                        return ptr_ty;
                    }
                }

                if let SemanticType::Struct(sym_id, _, field_tys) = peeled_ty {
                    let mut found_field = None;
                    if let Some(decl_id) = self.ctx.tables.symbol_decls.get(&sym_id) {
                        if (decl_id.0 as usize) < self.arena.decls.len() {
                            if let Decl::Struct { fields, .. } = &self.arena.decls[decl_id.0 as usize] {
                                if let Some((index, field)) = fields.iter().enumerate().find(|(_, field)| {
                                    self.get_span_text(field.name) == member_name
                                }) {
                                    found_field = Some((index, field.visibility));
                                }
                            }
                        }
                    }
                    if found_field.is_none() {
                        let struct_sym = self.ctx.symbol_table.get_symbol(sym_id);
                        if let Some(inner_scope) = struct_sym.inner_scope {
                            if let Some(field_sym_id) = self.ctx.symbol_table.lookup_exact_with_ctxt(member_name, luna_common::ids::SyntaxContext::ROOT, inner_scope) {
                                let field_sym = self.ctx.symbol_table.get_symbol(field_sym_id);
                                let index = self.ctx.tables.struct_fields.get(&sym_id)
                                    .and_then(|ids| ids.iter().position(|&id| id == field_sym_id))
                                    .unwrap_or(0);
                                found_field = Some((index, field_sym.visibility));
                            }
                        }
                    }
                    if let Some((index, visibility)) = found_field {
                        let is_accessible = self.check_field_visibility(sym_id, visibility, member_name, *member);
                        if !is_accessible {
                            return self.ctx.types.intern(SemanticType::Error);
                        }
                        self.ctx.tables.expr_member_indices.insert(*expr_id, index as u32);
                        let f_ty = field_tys.get(index).copied().unwrap_or_else(|| self.ctx.types.intern(SemanticType::Error));
                        self.ctx.tables.expr_types.insert(*expr_id, f_ty);
                        return f_ty;
                    }

                    // Look up methods on this struct (from local impl blocks)
                    let target_key = crate::semantic_tables::ImplSelfTypeKey::Nominal(sym_id);
                    let mut impl_candidates: Vec<_> = self.ctx.tables.trait_impls
                        .iter()
                        .filter(|(k, _)| k.self_type_def == target_key)
                        .collect();
                    impl_candidates.sort_by_key(|(k, decl_ids)| {
                        let is_trait = k.trait_id.is_some();
                        let trait_sym_id = k.trait_id.map(|s| s.0).unwrap_or(0);
                        let first_decl = decl_ids.first().map(|d| d.0).unwrap_or(0);
                        (is_trait, trait_sym_id, first_decl)
                    });

                    for search_traits in [false, true] {
                        for (impl_key, impl_decl_ids) in &impl_candidates {
                            if impl_key.trait_id.is_some() != search_traits {
                                continue;
                            }
                            for &impl_decl_id in *impl_decl_ids {
                                if (impl_decl_id.0 as usize) < self.arena.decls.len() {
                                    if let Decl::Impl { methods, .. } = &self.arena.decls[impl_decl_id.0 as usize] {
                                        for m_id in methods {
                                            if (m_id.0 as usize) < self.arena.decls.len() {
                                                if let Decl::Function { name, .. } = &self.arena.decls[m_id.0 as usize] {
                                                    let m_name = self.get_span_text(*name);
                                                    if m_name == member_name {
                                                        if let Some(&m_sym) = self.ctx.tables.decl_symbols.get(&m_id) {
                                                            let is_accessible = if let Some(trait_id) = impl_key.trait_id {
                                                                self.ctx.symbol_table.is_accessible(trait_id, self.current_scope, self.ctx.current_provider)
                                                            } else {
                                                                self.ctx.symbol_table.is_accessible(m_sym, self.current_scope, self.ctx.current_provider)
                                                            };
                                                            if !is_accessible {
                                                                self.ctx.diagnostics.push(Diagnostic::error(format!("Method `{}` is private and cannot be accessed from this scope", member_name)).with_code(DiagnosticCode::PrivateSymbolAccess).with_span(*member));
                                                                return self.ctx.types.intern(SemanticType::Error);
                                                            }
                                                            if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                                                self.ctx.tables.expr_symbols.insert(*expr_id, m_sym);
                                                                self.ctx.tables.expr_types.insert(*expr_id, m_ty);
                                                                return m_ty;
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

                    // Look up methods from external impl_methods table
                    let mut ext_candidates: Vec<_> = self.ctx.tables.impl_methods
                        .iter()
                        .filter(|(k, _)| k.self_type_def == target_key)
                        .collect();
                    ext_candidates.sort_by_key(|(k, method_syms)| {
                        let is_trait = k.trait_id.is_some();
                        let trait_sym_id = k.trait_id.map(|s| s.0).unwrap_or(0);
                        let first_method = method_syms.first().map(|s| s.0).unwrap_or(0);
                        (is_trait, trait_sym_id, first_method)
                    });

                    for search_traits in [false, true] {
                        for (impl_key, method_syms) in &ext_candidates {
                            if impl_key.trait_id.is_some() != search_traits {
                                continue;
                            }
                            for &m_sym in *method_syms {
                                let sym = self.ctx.symbol_table.get_symbol(m_sym);
                                if sym.name == member_name {
                                    let is_accessible = if let Some(trait_id) = impl_key.trait_id {
                                        self.ctx.symbol_table.is_accessible(trait_id, self.current_scope, self.ctx.current_provider)
                                    } else {
                                        self.ctx.symbol_table.is_accessible(m_sym, self.current_scope, self.ctx.current_provider)
                                    };
                                    if !is_accessible {
                                        self.ctx.diagnostics.push(Diagnostic::error(format!("Method `{}` is private and cannot be accessed from this scope", member_name)).with_code(DiagnosticCode::PrivateSymbolAccess).with_span(*member));
                                        return self.ctx.types.intern(SemanticType::Error);
                                    }
                                    if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                        self.ctx.tables.expr_symbols.insert(*expr_id, m_sym);
                                        self.ctx.tables.expr_types.insert(*expr_id, m_ty);
                                        return m_ty;
                                    }
                                }
                            }
                        }
                    }

                    self.ctx.diagnostics.push(Diagnostic::error(format!("Unknown field or method '{}'", member_name)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(*member));
                    let infer = self.ctx.types.new_inference_var();
                    self.ctx.tables.expr_types.insert(*expr_id, infer);
                    return infer;
                }

                if let SemanticType::GenericParam(gp_sym) = &peeled_ty {
                    if let Some(bounds) = self.ctx.tables.trait_bounds.get(gp_sym) {
                        for bound in bounds {
                            if let Some(m_syms) = self.ctx.tables.trait_methods.get(&bound.trait_id) {
                                for &m_sym in m_syms {
                                    let sym = self.ctx.symbol_table.get_symbol(m_sym);
                                    if sym.name == member_name {
                                        if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                            let mut subst = crate::ty::Substitution::new();
                                            let mut m_has_generics = false;
                                            if let Some(trait_gps) = self.ctx.tables.trait_generic_params.get(&bound.trait_id).cloned() {
                                                for (gp_idx, &arg_ty) in bound.trait_args.iter().enumerate() {
                                                    if let Some(&trait_gp_sym) = trait_gps.get(gp_idx) {
                                                        subst.insert(trait_gp_sym, arg_ty);
                                                        m_has_generics = true;
                                                    }
                                                }
                                            } else if let Some(&trait_decl_id) = self.ctx.tables.symbol_decls.get(&bound.trait_id) {
                                                for (gp_idx, &arg_ty) in bound.trait_args.iter().enumerate() {
                                                    if let Some(&trait_gp_sym) = self.ctx.tables.generic_param_symbols.get(&(trait_decl_id, gp_idx)) {
                                                        subst.insert(trait_gp_sym, arg_ty);
                                                        m_has_generics = true;
                                                    }
                                                }
                                            }
                                            let actual_m_ty = if m_has_generics { self.ctx.types.subst(m_ty, &subst) } else { m_ty };
                                            self.ctx.tables.expr_symbols.insert(*expr_id, m_sym);
                                            self.ctx.tables.expr_types.insert(*expr_id, actual_m_ty);
                                            return actual_m_ty;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let SemanticType::Primitive(b) = &peeled_ty {
                    let target_key = crate::semantic_tables::ImplSelfTypeKey::Primitive(*b);
                    let mut impl_candidates: Vec<_> = self.ctx.tables.trait_impls
                        .iter()
                        .filter(|(k, _)| k.self_type_def == target_key)
                        .collect();
                    impl_candidates.sort_by_key(|(k, decl_ids)| {
                        let is_trait = k.trait_id.is_some();
                        let trait_sym_id = k.trait_id.map(|s| s.0).unwrap_or(0);
                        let first_decl = decl_ids.first().map(|d| d.0).unwrap_or(0);
                        (is_trait, trait_sym_id, first_decl)
                    });

                    for search_traits in [false, true] {
                        for (impl_key, impl_decl_ids) in &impl_candidates {
                            if impl_key.trait_id.is_some() != search_traits {
                                continue;
                            }
                            for &impl_decl_id in *impl_decl_ids {
                                if (impl_decl_id.0 as usize) < self.arena.decls.len() {
                                    if let Decl::Impl { methods, .. } = &self.arena.decls[impl_decl_id.0 as usize] {
                                        for m_id in methods {
                                            if (m_id.0 as usize) < self.arena.decls.len() {
                                                if let Decl::Function { name, .. } = &self.arena.decls[m_id.0 as usize] {
                                                    if self.get_span_text(*name) == member_name {
                                                        if let Some(&m_sym) = self.ctx.tables.decl_symbols.get(&m_id) {
                                                            if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                                                self.ctx.tables.expr_symbols.insert(*expr_id, m_sym);
                                                                self.ctx.tables.expr_types.insert(*expr_id, m_ty);
                                                                return m_ty;
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

                    let mut ext_candidates: Vec<_> = self.ctx.tables.impl_methods
                        .iter()
                        .filter(|(k, _)| k.self_type_def == target_key)
                        .collect();
                    ext_candidates.sort_by_key(|(k, method_syms)| {
                        let is_trait = k.trait_id.is_some();
                        let trait_sym_id = k.trait_id.map(|s| s.0).unwrap_or(0);
                        let first_method = method_syms.first().map(|s| s.0).unwrap_or(0);
                        (is_trait, trait_sym_id, first_method)
                    });

                    for search_traits in [false, true] {
                        for (impl_key, method_syms) in &ext_candidates {
                            if impl_key.trait_id.is_some() != search_traits {
                                continue;
                            }
                            for &m_sym in *method_syms {
                                let sym = self.ctx.symbol_table.get_symbol(m_sym);
                                if sym.name == member_name {
                                    if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                        self.ctx.tables.expr_symbols.insert(*expr_id, m_sym);
                                        self.ctx.tables.expr_types.insert(*expr_id, m_ty);
                                        return m_ty;
                                    }
                                }
                            }
                        }
                    }
                }
                self.ctx.diagnostics.push(Diagnostic::error("Member access requires a struct or trait object").with_code(DiagnosticCode::TypeMismatch).with_span(*member));
                self.ctx.types.new_inference_var()
            }
            Expr::StructInit { path, fields, .. } => {
                if path.is_empty() {
                    return self.ctx.types.new_inference_var();
                }
                let full_name = path.iter().map(|s| self.get_span_text(*s)).collect::<Vec<_>>().join("::");
                let span = path.last().copied().unwrap_or(luna_common::Span::default());

                let symbol = if path.len() == 1 {
                    let seg = path[0];
                    let name = self.get_span_text(seg);
                    self.ctx.symbol_table.lookup_with_ctxt(name, seg.ctxt, self.current_scope)
                        .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(name, seg.ctxt, crate::ScopeId(0)))
                } else {
                    let mut scope = self.current_scope;
                    let mut res = None;
                    for (i, seg) in path.iter().enumerate() {
                        let seg_name = self.get_span_text(*seg);
                        let sym_id = if i == 0 {
                            self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, scope)
                                .or_else(|| self.ctx.symbol_table.lookup_with_ctxt(seg_name, seg.ctxt, crate::ScopeId(0)))
                        } else {
                            self.ctx.symbol_table.lookup_exact_with_ctxt(seg_name, seg.ctxt, scope)
                        };
                        if let Some(id) = sym_id {
                            if i > 0 && !self.ctx.symbol_table.is_accessible(id, self.current_scope, self.ctx.current_provider) {
                                self.ctx.diagnostics.push(Diagnostic::error(format!("Struct `{}` is private and cannot be accessed from this scope", seg_name)).with_code(DiagnosticCode::PrivateSymbolAccess).with_span(*seg));
                                return self.ctx.types.new_inference_var();
                            }
                            res = Some(id);
                            if let Some(inner) = self.ctx.symbol_table.symbols[id.0 as usize].inner_scope {
                                scope = inner;
                            } else if i < path.len() - 1 {
                                res = None;
                                break;
                            }
                        } else {
                            res = None;
                            break;
                        }
                    }
                    res
                };

                let Some(symbol) = symbol else {
                    self.ctx.diagnostics.push(Diagnostic::error(format!("Unknown struct '{}'", full_name)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(span));
                    return self.ctx.types.new_inference_var();
                };

                if !self.ctx.symbol_table.is_accessible(symbol, self.current_scope, self.ctx.current_provider) {
                    self.ctx.diagnostics.push(Diagnostic::error(format!("Struct '{}' is private and cannot be accessed from this scope", full_name)).with_code(DiagnosticCode::PrivateSymbolAccess).with_span(span));
                    return self.ctx.types.new_inference_var();
                }

                let base_ty = self.ctx.tables.symbol_types.get(&symbol).copied().unwrap_or_else(|| self.ctx.types.new_inference_var());
                let SemanticType::Struct(sym_id, _, original_field_tys) = self.ctx.types.get(base_ty).clone() else {
                    self.ctx.diagnostics.push(Diagnostic::error(format!("'{}' is not a struct", full_name)).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    return self.ctx.types.new_inference_var();
                };
                let Some(decl_id) = self.ctx.tables.symbol_decls.get(&symbol).copied() else {
                    return base_ty;
                };
                let Decl::Struct { fields: declared_fields, generic_params, .. } = &self.arena.decls[decl_id.0 as usize] else {
                    return base_ty;
                };

                let mut concrete_args = Vec::new();
                if let Expr::StructInit { generic_args, .. } = &self.arena.exprs[expr_id.0 as usize] {
                    for arg in generic_args {
                        concrete_args.push(self.lower_type(*arg));
                    }
                }
                
                let expected_args = generic_params.len();
                if concrete_args.len() < expected_args {
                    for _ in concrete_args.len()..expected_args {
                        concrete_args.push(self.ctx.types.new_inference_var());
                    }
                }

                let mut subst = crate::ty::Substitution::new();
                for (idx, _) in generic_params.iter().enumerate() {
                    if let Some(gp_sym) = self.ctx.tables.generic_param_symbols.get(&(decl_id, idx)) {
                        if idx < concrete_args.len() {
                            subst.insert(*gp_sym, concrete_args[idx]);
                        }
                    }
                }
                
                let mut field_tys = Vec::new();
                for orig_ty in original_field_tys {
                    field_tys.push(self.ctx.types.subst(orig_ty, &subst));
                }

                let struct_ty = self.ctx.types.intern(SemanticType::Struct(sym_id, concrete_args, field_tys.clone()));
                if let Some(expected) = self.expected_expr_type.last().copied() {
                    if matches!(self.ctx.types.get(expected), SemanticType::Struct(expected_sym, ..) if *expected_sym == sym_id) {
                        let _ = self.unify(struct_ty, expected);
                    }
                }

                let mut init_indices = Vec::with_capacity(fields.len());
                for field in fields {
                    let field_name = self.get_span_text(field.name);
                    if let Some((index, declared)) = declared_fields.iter().enumerate().find(|(_, declared)| {
                        self.get_span_text(declared.name) == field_name
                    }) {
                        self.check_field_visibility(symbol, declared.visibility, field_name, field.name);
                        let value_ty = self.typecheck_expr_expected(&field.value, field_tys[index]);
                        if self.unify(field_tys[index], value_ty).is_err() {
                            self.ctx.diagnostics.push(Diagnostic::error(format!("Type mismatch for field '{}'", field_name)).with_code(DiagnosticCode::TypeMismatch).with_span(field.name));
                        }
                        init_indices.push(index as u32);
                    } else {
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Unknown field '{}' for struct '{}'", field_name, full_name)).with_code(DiagnosticCode::UnresolvedSymbol).with_span(field.name));
                        self.typecheck_expr(&field.value);
                        init_indices.push(u32::MAX); // Error recovery
                    }
                }

                // Check that all declared fields are initialized
                for declared in declared_fields {
                    let declared_name = self.get_span_text(declared.name);
                    let provided = fields.iter().any(|f| self.get_span_text(f.name) == declared_name);
                    if !provided {
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Missing field `{}` in initializer of `{}`", declared_name, full_name)).with_code(DiagnosticCode::MissingField).with_span(span));
                    }
                }
                // Field/expected-type inference completes omitted arguments
                // before checking the nominal declaration's bounds.
                if !generic_params.is_empty() {
                    self.check_bounds_for_call(sym_id, &subst, path.first().copied().unwrap_or(span));
                }
                self.ctx.tables.expr_struct_init_indices.insert(*expr_id, init_indices);
                struct_ty
            }
            Expr::ArrayLiteral { elements } => {
                let expected_elem = self.expected_expr_type.last().and_then(|&ty| match self.ctx.types.get(self.ctx.types.resolve_inference(ty)) {
                    SemanticType::Array(elem, _) => Some(*elem), _ => None,
                });
                let mut elem_ty = expected_elem.unwrap_or_else(|| self.ctx.types.new_inference_var());
                for (i, el) in elements.iter().enumerate() {
                    let ty = if let Some(expected) = expected_elem {
                        self.typecheck_expr_expected(el, expected)
                    } else { self.typecheck_expr_without_expected(el) };
                    if i == 0 {
                        elem_ty = ty;
                    } else if self.unify(elem_ty, ty).is_err() {
                        self.ctx.diagnostics.push(Diagnostic::error("Array literal elements must have the same type").with_code(DiagnosticCode::TypeMismatch));
                    }
                }
                self.ctx.types.intern(SemanticType::Array(elem_ty, elements.len() as u64))
            }
            Expr::TupleLiteral { elements } => {
                let mut elem_tys = Vec::new();
                let expected_fields = self.expected_expr_type.last().and_then(|&ty| match self.ctx.types.get(self.ctx.types.resolve_inference(ty)) {
                    SemanticType::Tuple(fields) => Some(fields.clone()), _ => None,
                });
                for (index, el) in elements.iter().enumerate() {
                    elem_tys.push(if let Some(expected) = expected_fields.as_ref().and_then(|fields| fields.get(index)) {
                        self.typecheck_expr_expected(el, *expected)
                    } else { self.typecheck_expr_without_expected(el) });
                }
                self.ctx.types.intern(SemanticType::Tuple(elem_tys))
            }
            Expr::TupleIndex { object, index } => {
                let base_ty_id = self.typecheck_expr(object);
                let base_ty = self.ctx.types.get(base_ty_id).clone();
                let span = self.get_expr_span_for_diag(expr_id).unwrap_or_default();
                match base_ty {
                    SemanticType::Tuple(elem_tys) => {
                        if *index as usize >= elem_tys.len() {
                            self.ctx.diagnostics.push(Diagnostic::error(format!("Tuple index {} out of bounds (tuple has {} elements)", index, elem_tys.len())).with_code(DiagnosticCode::CannotIndex).with_span(span));
                            self.ctx.types.error_id()
                        } else {
                            elem_tys[*index as usize]
                        }
                    }
                    SemanticType::Error => self.ctx.types.error_id(),
                    _ => {
                        self.ctx.diagnostics.push(Diagnostic::error("Cannot index into a non-tuple type").with_code(DiagnosticCode::CannotIndex).with_span(span));
                        self.ctx.types.error_id()
                    }
                }
            }
            Expr::Index { base, index } => {
                let base_ty_id = self.typecheck_expr(base);
                let index_ty_id = self.typecheck_expr_expected(index, self.ctx.types.usize_id());
                
                let base_ty = self.ctx.types.get(base_ty_id).clone();
                let index_ty = self.ctx.types.get(index_ty_id).clone();
                let mut has_err = false;

                // 1. Index validation
                match &index_ty {
                    SemanticType::Primitive(b) if b.is_integer() => {}
                    SemanticType::InferenceVar(_) => {
                        let usize_id = self.ctx.types.usize_id();
                        if let Err(e) = self.unify(index_ty_id, usize_id) {
                            let span = self.get_expr_span_for_diag(index).unwrap_or_else(|| self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0)));
                            self.ctx.diagnostics.push(Diagnostic::error(format!("E_INVALID_INDEX_TYPE: {}", e)).with_code(DiagnosticCode::CannotIndex).with_span(span));
                            has_err = true;
                        }
                    }
                    SemanticType::Error => {
                        has_err = true;
                    }
                    _ => {
                        let span = self.get_expr_span_for_diag(index).unwrap_or_else(|| self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0)));
                        self.ctx.diagnostics.push(Diagnostic::error(format!("E_INVALID_INDEX_TYPE: Expected integer index type, found `{:?}`", index_ty)).with_code(DiagnosticCode::CannotIndex).with_span(span));
                        has_err = true;
                    }
                }

                // 2. Base validation
                let mut curr_base_ty = base_ty.clone();
                while let SemanticType::Reference(_, _, inner) = curr_base_ty {
                    curr_base_ty = self.ctx.types.get(inner).clone();
                }

                let elem_ty = match curr_base_ty {
                    SemanticType::Array(el, _) => el,
                    SemanticType::Slice(el) => el,
                    SemanticType::Error => {
                        has_err = true;
                        self.ctx.types.error_id()
                    }
                    _ => {
                        let span = self.get_expr_span_for_diag(base).unwrap_or_else(|| self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0)));
                        self.ctx.diagnostics.push(Diagnostic::error(format!("E_CANNOT_INDEX: Type `{:?}` cannot be indexed", base_ty)).with_code(DiagnosticCode::CannotIndex).with_span(span));
                        has_err = true;
                        self.ctx.types.error_id()
                    }
                };

                if has_err {
                    self.ctx.types.error_id()
                } else {
                    elem_ty
                }
            }
            Expr::MethodCall { object, method_name, generic_args, args } => {
                let obj_ty_id = self.typecheck_expr_without_expected(object);
                let obj_ty = self.ctx.types.get(obj_ty_id).clone();
                let member_name = self.get_span_text(*method_name);
                
                // 1. Check if obj_ty is DynTrait or pointer/ref to DynTrait
                let dyn_trait_sym = match &obj_ty {
                    SemanticType::DynTrait(sym) => Some(*sym),
                    SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => {
                        if let SemanticType::DynTrait(sym) = self.ctx.types.get(*inner) {
                            Some(*sym)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(trait_sym) = dyn_trait_sym {
                    if let Some(method_syms) = self.ctx.tables.trait_methods.get(&trait_sym) {
                        for (idx, &m_sym) in method_syms.iter().enumerate() {
                            let sym = self.ctx.symbol_table.get_symbol(m_sym);
                            if sym.name == member_name {
                                self.ctx.tables.dyn_method_indices.insert(*expr_id, idx as u32);
                                if Some(trait_sym) == self.ctx.lang_items.get(crate::lang_item::LangItem::Drop)
                                    || Some(m_sym) == self.ctx.lang_items.get(crate::lang_item::LangItem::DropFn)
                                {
                                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                    self.ctx.diagnostics.push(Diagnostic::error("Explicit calls to drop() are forbidden. Values are dropped automatically at end of scope.")
                                        .with_code(DiagnosticCode::TypeMismatch)
                                        .with_span(span));
                                }
                                if self.ctx.tables.unsafe_functions.contains(&m_sym) && !self.is_unsafe_context {
                                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                    self.ctx.diagnostics.push(Diagnostic::error("E_CALL_UNSAFE_FN_OUTSIDE_UNSAFE: Call to unsafe trait method requires an unsafe block")
                                        .with_code(DiagnosticCode::UnsafeOperationOutsideUnsafe)
                                        .with_span(span));
                                }
                                // G1 Vector 5: Check receiver mutability.
                                // If the method requires &rw self but the object is immutable
                                // (&dyn Trait or *dyn Trait), reject the call.
                                if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                    if let SemanticType::Function { params: fn_params, .. } = self.ctx.types.get(m_ty).clone() {
                                        if let Some(&receiver_ty_id) = fn_params.first() {
                                            let receiver_ty = self.ctx.types.get(receiver_ty_id).clone();
                                            let method_needs_rw = matches!(receiver_ty, SemanticType::Reference(_, crate::ty::Mutability::Mutable, _));
                                            if method_needs_rw {
                                                let obj_is_immutable = match &obj_ty {
                                                    SemanticType::Reference(_, crate::ty::Mutability::Immutable, _) => true,
                                                    SemanticType::Pointer(crate::ty::Mutability::Immutable, _) => true,
                                                    _ => false,
                                                };
                                                if obj_is_immutable {
                                                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                                    self.ctx.diagnostics.push(Diagnostic::error("E_CANNOT_MUTATE_IMMUTABLE_POINTER: Cannot call method requiring mutable receiver (&rw self) through immutable reference or pointer").with_code(DiagnosticCode::CannotMutateImmutable)
                                                        .with_span(span));
                                                }
                                            }
                                        }
                                    }
                                }
                                if let Some(&m_ty) = self.ctx.tables.symbol_types.get(&m_sym) {
                                    if let SemanticType::Function { params, return_type, .. } = self.ctx.types.get(m_ty).clone() {
                                        if params.len() != args.len() + 1 || !generic_args.is_empty() {
                                            self.ctx.diagnostics.push(Diagnostic::error(format!(
                                                "Dynamic method requires {} arguments and no generic arguments; got {} arguments",
                                                params.len().saturating_sub(1), args.len()))
                                                .with_code(DiagnosticCode::TypeMismatch).with_span(*method_name));
                                            return self.ctx.types.error_id();
                                        }
                                        let expected_params = &params[1..];
                                        let Some(binding) = self.bind_call_arguments(*expr_id, Some(m_sym), true,
                                            expected_params.len(), false, args, *method_name) else {
                                            return self.ctx.types.error_id();
                                        };
                                        for (i, arg) in args.iter().enumerate() {
                                            let parameter = binding.source_to_parameter[i] as usize;
                                            let arg_ty = expected_params.get(parameter).map(|&expected| self.typecheck_expr_expected(&arg.value, expected))
                                                .unwrap_or_else(|| self.typecheck_expr(&arg.value));
                                            if let Some(&expected_p) = expected_params.get(parameter) {
                                                if !self.try_coerce(arg.value, arg_ty, expected_p) {
                                                    if let Err(e) = self.unify(expected_p, arg_ty) {
                                                        let span = self.get_expr_span_for_diag(&arg.value).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                                                        self.ctx.diagnostics.push(Diagnostic::error(e).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                                                    }
                                                }
                                            }
                                        }
                                        self.ctx.tables.expr_types.insert(*expr_id, return_type);
                                        return return_type;
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Struct method lookup
                let peeled_ty = match &obj_ty {
                    SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => {
                        self.ctx.types.get(*inner).clone()
                    }
                    _ => obj_ty.clone(),
                };

                if let SemanticType::Slice(_) = peeled_ty {
                    if member_name == "len" || member_name == "length" {
                        if !args.is_empty() {
                            let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error("`len()` on slice takes no arguments").with_code(DiagnosticCode::TypeMismatch).with_span(span));
                        }
                        let usize_ty = self.ctx.types.intern(SemanticType::Primitive(BuiltinType::Usize));
                        self.ctx.tables.expr_types.insert(*expr_id, usize_ty);
                        self.ctx.tables.expr_member_indices.insert(*expr_id, 1);
                        return usize_ty;
                    } else if member_name == "is_empty" {
                        if !args.is_empty() {
                            let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error("`is_empty()` on slice takes no arguments").with_code(DiagnosticCode::TypeMismatch).with_span(span));
                        }
                        let bool_ty = self.ctx.types.bool_id();
                        self.ctx.tables.expr_types.insert(*expr_id, bool_ty);
                        self.ctx.tables.expr_member_indices.insert(*expr_id, 2);
                        return bool_ty;
                    }
                }

                if let SemanticType::GenericParam(parameter) = &peeled_ty {
                    return self.resolve_bound_method(*expr_id, *object, obj_ty_id, *parameter,
                        &member_name, *method_name, generic_args, args);
                }

                let target_self_info = match &peeled_ty {
                    SemanticType::Struct(sym_id, struct_args, _) => Some((crate::semantic_tables::ImplSelfTypeKey::Nominal(*sym_id), struct_args.clone(), Some(*sym_id))),
                    SemanticType::Enum(sym_id, enum_args, _) => Some((crate::semantic_tables::ImplSelfTypeKey::Nominal(*sym_id), enum_args.clone(), Some(*sym_id))),
                    SemanticType::Primitive(b) => Some((crate::semantic_tables::ImplSelfTypeKey::Primitive(*b), Vec::new(), None)),
                    SemanticType::Slice(elem) => Some((crate::semantic_tables::ImplSelfTypeKey::Slice, vec![*elem], None)),
                    _ => None,
                };

                if let Some((target_key, _, _)) = target_self_info {
                    return self.resolve_concrete_method(*expr_id, *object, obj_ty_id,
                        target_key, &member_name, *method_name, generic_args, args);
                }

                for arg in args {
                    self.typecheck_expr(&arg.value);
                }

                if !matches!(obj_ty, SemanticType::Error) {
                    self.ctx.diagnostics.push(
                        Diagnostic::error(format!(
                            "Method `{}` not found for type `{:?}`",
                            member_name, obj_ty
                        ))
                        .with_code(DiagnosticCode::UnresolvedSymbol)
                        .with_span(*method_name),
                    );
                }

                self.ctx.types.error_id()
            }
            Expr::Sizeof { target_type } => {
                self.lower_type(*target_type);
                self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::I32))
            }
            Expr::Alignof { target_type } => {
                self.lower_type(*target_type);
                self.ctx.types.intern(SemanticType::Primitive(crate::ty::BuiltinType::I32))
            }
            Expr::Cast { expr: e, target_type } => {
                // A cast converts an independently typed operand; it is not
                // contextual literal typing, even within a typed initializer.
                let source_ty_id = self.typecheck_expr_expected(e, self.ctx.types.error_id());
                let target_ty_id = self.lower_type(*target_type);
                let source_ty = self.ctx.types.get(source_ty_id).clone();
                let target_ty = self.ctx.types.get(target_ty_id).clone();

                let cast_span = self.get_expr_span_for_diag(expr_id).unwrap_or_default();
                let admission = self.ctx.types.cast_admission(source_ty_id, target_ty_id);
                let diagnostic_start = self.ctx.diagnostics.len();
                let invalid_operand = matches!(source_ty, SemanticType::Error) || matches!(target_ty, SemanticType::Error);
                if !invalid_operand {
                    match admission {
                        Err(reason) => {
                            // Preserve the existing unsized-value diagnostic
                            // classification while central cast admission still
                            // rejects the operation in every lowering path.
                            let (code, category) = if self.ctx.types.is_unsized(target_ty_id) {
                                (DiagnosticCode::TypeMismatch, "E_UNSIZED_TYPE_IN_VALUE_POSITION")
                            } else {
                                (DiagnosticCode::InvalidCast, "E_INVALID_CAST")
                            };
                            self.ctx.diagnostics.push(
                                Diagnostic::error(format!("{category}: {reason}"))
                                    .with_code(code).with_span(cast_span));
                        }
                        Ok(crate::cast::CastAdmission::RequiresUnsafe) if !self.is_unsafe_context =>
                            self.ctx.diagnostics.push(Diagnostic::error("Cast requires an unsafe block")
                                .with_code(DiagnosticCode::UnsafeOperationOutsideUnsafe).with_span(cast_span)),
                        _ => {}
                    }
                }
                if invalid_operand || self.ctx.diagnostics.len() != diagnostic_start {
                    self.ctx.types.error_id()
                } else {
                    // Luna `char` denotes a Unicode scalar value, not an arbitrary
                    // 32-bit integer. Keep that semantic invariant in the frontend
                    // so known-invalid values never reach code generation.
                    if matches!(target_ty, SemanticType::Primitive(BuiltinType::Char)) {
                        let source_is_char = matches!(source_ty, SemanticType::Primitive(BuiltinType::Char));
                        let source_is_integer = matches!(source_ty, SemanticType::Primitive(b) if b.is_integer());
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or_default();

                        if source_is_integer {
                            if crate::const_eval::is_const_evaluable(
                                self.arena,
                                self.ctx,
                                self.source_manager,
                                *e,
                            )
                            .is_ok()
                            {
                                if let Ok(crate::ComptimeValue::Int { val, .. }) = self.eval_comptime_expr(*e) {
                                    if !Self::is_unicode_scalar(val) {
                                        self.ctx.diagnostics.push(
                                            Diagnostic::error(
                                                "invalid integer-to-char cast: value is not a Unicode scalar",
                                            )
                                            .with_code(DiagnosticCode::InvalidCast)
                                            .with_span(span),
                                        );
                                    }
                                }
                            }
                        } else if !source_is_char {
                            self.ctx.diagnostics.push(
                                Diagnostic::error(
                                    "invalid cast to char: source must be an integer or char",
                                )
                                .with_code(DiagnosticCode::InvalidCast)
                                .with_span(span),
                            );
                        }
                    }

                    // Diagnose statically-known float-to-integer overflow before lowering.
                    // This keeps comptime casts out of the backend's runtime trap path and
                    // prevents malformed/unsupported numeric literals from reaching codegen.
                    if matches!(source_ty, SemanticType::Primitive(BuiltinType::F32 | BuiltinType::F64)) {
                        if let (Some(value), Some((min, max))) = (self.constant_float_literal(*e), Self::integer_float_bounds(&target_ty)) {
                            let truncated = value.trunc();
                            if !truncated.is_finite() || truncated < min || truncated >= max {
                                let span = self.get_expr_span_for_diag(expr_id).unwrap_or_default();
                                self.ctx.diagnostics.push(
                                    Diagnostic::error("numeric conversion out of range in float-to-integer cast")
                                        .with_code(DiagnosticCode::InvalidCast)
                                        .with_span(span),
                                );
                            }
                        }
                    }

                    if self.ctx.diagnostics.len() != diagnostic_start {
                        self.ctx.types.error_id()
                    } else {
                        target_ty_id
                    }
                }
            }
            Expr::Match { subject, arms, match_span } => {
                let subject_ty_id = self.typecheck_expr_expected(subject, self.ctx.types.error_id());
                let subject_ty = self.ctx.types.get(subject_ty_id).clone();
                let mut result_ty = self.ctx.types.new_inference_var();
                
                let mut covered_variants = std::collections::HashSet::new();
                let mut covered_bools = std::collections::HashSet::new();
                let mut has_wildcard = false;

                for (i, arm) in arms.iter().enumerate() {
                    self.typecheck_pattern(&arm.pattern, subject_ty_id);
                    
                    match &self.arena.pats[arm.pattern.0 as usize] {
                        luna_ast::Pattern::Wildcard => {
                            has_wildcard = true;
                        }
                        luna_ast::Pattern::Identifier { .. } => {
                            if let Some(sym_id) = self.ctx.tables.pat_symbols.get(&arm.pattern) {
                                let sym = self.ctx.symbol_table.get_symbol(*sym_id);
                                if matches!(sym.kind, crate::symbol::SymbolKind::EnumVariant(_)) {
                                    covered_variants.insert(*sym_id);
                                } else {
                                    has_wildcard = true;
                                }
                            } else {
                                has_wildcard = true;
                            }
                        }
                        luna_ast::Pattern::Enum { .. } => {
                            if let Some(sym_id) = self.ctx.tables.pat_symbols.get(&arm.pattern) {
                                covered_variants.insert(*sym_id);
                            }
                        }
                        luna_ast::Pattern::Literal(tok) => {
                            if tok.kind == luna_lexer::TokenKind::KwTrue {
                                covered_bools.insert(true);
                            } else if tok.kind == luna_lexer::TokenKind::KwFalse {
                                covered_bools.insert(false);
                            }
                        }
                        _ => {}
                    }
                    
                    // Typecheck arm body
                    self.typecheck_stmt(&arm.body);
                    // Extract arm result type
                    let mut arm_ty = self.ctx.types.intern(SemanticType::Void);
                    match &self.arena.stmts[arm.body.0 as usize] {
                        Stmt::Block { tail_expr: Some(tail), .. } => {
                            if let Some(ty) = self.ctx.tables.expr_types.get(tail).copied() { arm_ty = ty; }
                        }
                        Stmt::Expr { expr, has_semicolon: false } => {
                            if let Some(ty) = self.ctx.tables.expr_types.get(expr).copied() { arm_ty = ty; }
                        }
                        _ => {}
                    }
                    
                    if let Err(err) = self.unify(result_ty, arm_ty) {
                        self.ctx.diagnostics.push(luna_common::diagnostic::Diagnostic::error(err).with_code(DiagnosticCode::TypeMismatch).with_span(*match_span));
                    }
                }
                
                // Exhaustiveness check
                if !has_wildcard {
                    match subject_ty {
                        SemanticType::Primitive(crate::ty::BuiltinType::Bool) => {
                            if !covered_bools.contains(&true) || !covered_bools.contains(&false) {
                                self.ctx.diagnostics.push(
                                    luna_common::diagnostic::Diagnostic::error("Match is not exhaustive. Missing boolean values.")
                                        .with_code(DiagnosticCode::NonExhaustivePattern)
                                        .with_span(*match_span)
                                );
                            }
                        }
                        SemanticType::Enum(sym_id, _, _) => {
                            let decl_id = self.ctx.symbol_table.get_symbol(sym_id).decl_id;
                            if let Some(decl_id) = decl_id {
                                if let luna_ast::Decl::Enum { variants, .. } = &self.arena.decls[decl_id.0 as usize] {
                                    if covered_variants.len() < variants.len() {
                                        self.ctx.diagnostics.push(
                                            luna_common::diagnostic::Diagnostic::error(format!("Match is not exhaustive. Covered {}/{} enum variants.", covered_variants.len(), variants.len()))
                                                .with_code(DiagnosticCode::NonExhaustivePattern)
                                                .with_span(*match_span)
                                        );
                                    }
                                }
                            }
                        }
                        SemanticType::InferenceVar(_) | SemanticType::Error => {}
                        _ => {
                            self.ctx.diagnostics.push(
                                luna_common::diagnostic::Diagnostic::error("Match is not exhaustive. A wildcard pattern `_` or variable binding is required for this type.")
                                    .with_code(DiagnosticCode::NonExhaustivePattern)
                                    .with_span(*match_span)
                            );
                        }
                    }
                }
                
                self.ctx.types.resolve(result_ty)
            }
            Expr::Lambda { body, params, return_type, is_move } => {
                let mut param_tys = Vec::new();
                for param_id in params {
                    if let luna_ast::Decl::Param { ty, .. } = &self.arena.decls[param_id.0 as usize] {
                        let ty_id = if let Some(param_ty) = ty {
                            self.lower_type(*param_ty)
                        } else {
                            self.ctx.types.new_inference_var()
                        };
                        if let Some(&sym_id) = self.ctx.tables.decl_symbols.get(param_id) {
                            self.ctx.tables.symbol_types.insert(sym_id, ty_id);
                        }
                        param_tys.push(ty_id);
                    } else {
                        param_tys.push(self.ctx.types.new_inference_var());
                    }
                }
                let ret_ty_id = if let Some(ret_ty) = return_type {
                    self.lower_type(*ret_ty)
                } else {
                    self.ctx.types.new_inference_var()
                };
                
                self.active_lambdas.push(*expr_id);
                self.current_return_type.push(ret_ty_id);
                self.typecheck_function_body(body, ret_ty_id);
                self.current_return_type.pop();
                self.active_lambdas.pop();
                if let Some(body_ty) = self.infer_stmt_value_type(body) {
                    if let Err(_) = self.unify(ret_ty_id, body_ty) {
                        let mut diag = Diagnostic::error("Lambda return type mismatch").with_code(DiagnosticCode::TypeMismatch);
                        if let Some(span) = self.get_expr_span_for_diag(expr_id) {
                            diag.span = Some(span);
                        }
                        self.ctx.diagnostics.push(diag);
                    }
                } else if return_type.is_none()
                    && matches!(self.ctx.types.get(self.ctx.types.resolve_inference(ret_ty_id)), SemanticType::InferenceVar(_))
                {
                    // A statement-only closure with no value return produces
                    // void. Do not leave its output unconstrained at mono.
                    let void_ty = self.ctx.types.intern(SemanticType::Void);
                    let _ = self.unify(ret_ty_id, void_ty);
                }
                let ret_ty_id = self.ctx.types.resolve_inference(ret_ty_id);
                
                let mut env_tys = Vec::new();
                let mut capture_bindings = Vec::new();
                if let Some(captures) = self.ctx.tables.expr_captures.get(expr_id).cloned() {
                    for (field, sym_id) in captures.into_iter().enumerate() {
                        let sym_ty = self.ctx.tables.symbol_types.get(&sym_id).copied().unwrap_or_else(|| self.ctx.types.intern(SemanticType::Error));
                        let is_mutated = self.ctx.tables.closure_mutated_captures.get(expr_id).map_or(false, |mutated| mutated.contains(&sym_id));
                        let mode = if *is_move {
                            crate::semantic_tables::CaptureMode::Move
                        } else if is_mutated {
                            crate::semantic_tables::CaptureMode::MutableBorrow
                        } else {
                            crate::semantic_tables::CaptureMode::SharedBorrow
                        };
                        
                        let env_ty = match mode {
                            crate::semantic_tables::CaptureMode::MutableBorrow => {
                                self.ctx.types.intern(SemanticType::Pointer(crate::ty::Mutability::Mutable, sym_ty))
                            }
                            crate::semantic_tables::CaptureMode::SharedBorrow => {
                                self.ctx.types.intern(SemanticType::Pointer(crate::ty::Mutability::Immutable, sym_ty))
                            }
                            crate::semantic_tables::CaptureMode::Move => sym_ty,
                        };
                        env_tys.push(env_ty);
                        let is_mutated = self.ctx.tables.closure_mutated_captures.get(expr_id).map_or(false, |mutated| mutated.contains(&sym_id));
                        let is_mutable = self.ctx.tables.symbol_decls.get(&sym_id).and_then(|decl_id| match &self.arena.decls[decl_id.0 as usize] {
                            Decl::Var { is_mutable, .. } => Some(*is_mutable),
                            Decl::Param { .. } => Some(true),
                            _ => None,
                        }).unwrap_or(false);
                        if is_mutated && !is_mutable {
                            let mut diag = Diagnostic::error("Cannot mutably capture immutable variable").with_code(DiagnosticCode::CannotMutateImmutable);
                            if let Some(span) = self.get_expr_span_for_diag(expr_id) {
                                diag.span = Some(span);
                            }
                            self.ctx.diagnostics.push(diag);
                        }
                        capture_bindings.push(crate::semantic_tables::CaptureBinding {
                            symbol: sym_id,
                            mode,
                            env_field: field as u32,
                            ty: sym_ty,
                            env_ty,
                        });
                    }
                }
                self.ctx.tables.closure_capture_bindings.insert(*expr_id, capture_bindings);
                let env_tuple_ty = self.ctx.types.intern(SemanticType::Tuple(env_tys));
                self.ctx.tables.closure_env_types.insert(*expr_id, env_tuple_ty);
                let env_ptr_ty = self.ctx.types.intern(SemanticType::Pointer(
                    crate::ty::Mutability::Immutable,
                    env_tuple_ty,
                ));
                self.ctx.tables.closure_env_ptr_types.insert(*expr_id, env_ptr_ty);
                
                self.ctx.types.intern(SemanticType::Closure(
                    *expr_id,
                    param_tys,
                    ret_ty_id,
                ))
            }
            Expr::Try { expr: e, .. } => {
                let inner_ty_id = self.typecheck_expr(e);
                let try_span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                
                let try_sym = self.ctx.lang_items.get(crate::lang_item::LangItem::Try);
                let from_residual_sym = self.ctx.lang_items.get(crate::lang_item::LangItem::FromResidual);
                
                if try_sym.is_none() || from_residual_sym.is_none() {
                    let mut diag = Diagnostic::error("Missing language item: `Try` or `FromResidual`").with_code(DiagnosticCode::InvalidTryOperator);
                    diag.span = Some(try_span);
                    self.ctx.diagnostics.push(diag);
                    return self.ctx.types.intern(SemanticType::Error);
                }
                
                let try_sym = try_sym.unwrap();
                let from_residual_sym = from_residual_sym.unwrap();

                let resolved_inner = self.ctx.types.resolve_inference(inner_ty_id);
                let mut found_try_impl = false;
                match self.ctx.types.get(resolved_inner) {
                    SemanticType::GenericParam(gp_sym) => {
                        if let Some(bounds) = self.ctx.tables.trait_bounds.get(gp_sym) {
                            if bounds.iter().any(|b| b.trait_id == try_sym) {
                                found_try_impl = true;
                            }
                        }
                    }
                    SemanticType::Struct(sym, _, _) | SemanticType::Enum(sym, _, _) => {
                        let key = crate::semantic_tables::ImplKey {
                            trait_id: Some(try_sym),
                            self_type_def: (*sym).into(),
                        };
                        if self.ctx.tables.trait_impls.contains_key(&key) {
                            found_try_impl = true;
                        }
                    }
                    SemanticType::InferenceVar(_) | SemanticType::Error => {
                        found_try_impl = true;
                    }
                    _ => {}
                }

                // If nominal concrete type, look up try branch method
                let mut inner_def_sym = None;
                if let SemanticType::Enum(sym, _, _) | SemanticType::Struct(sym, _, _) = self.ctx.types.get(resolved_inner) {
                    inner_def_sym = Some(*sym);
                }
                
                let try_branch_name = self.ctx.lang_items.get(crate::lang_item::LangItem::TryBranch)
                    .map(|s| self.ctx.symbol_table.get_symbol(s).name.as_str().to_string());
                
                if let Some(def_sym) = inner_def_sym {
                    let key = crate::semantic_tables::ImplKey {
                        trait_id: Some(try_sym),
                        self_type_def: def_sym.into(),
                    };
                    if let Some(method_syms) = self.ctx.tables.impl_methods.get(&key) {
                        for &m_sym in method_syms {
                            let m_sym_name = self.ctx.symbol_table.get_symbol(m_sym).name.as_str();
                            if Some(m_sym_name) == try_branch_name.as_deref() {
                                let m_decl = self.ctx.tables.symbol_decls.get(&m_sym).copied().unwrap();
                                self.ctx.tables.try_branch_methods.insert(*expr_id, m_decl);
                                if let Some(subst) = self.impl_method_substitution(m_decl, try_sym, resolved_inner, &[]) {
                                    let method_ty = self.ctx.tables.symbol_types.get(&m_sym).copied();
                                    if let Some(method_ty) = method_ty {
                                        let instantiated_ty = self.ctx.types.subst(method_ty, &subst);
                                        if let SemanticType::Function { return_type, .. } = self.ctx.types.get(instantiated_ty) {
                                            self.ctx.tables.try_branch_return_types.insert(*expr_id, *return_type);
                                        }
                                    }
                                    self.ctx.tables.try_branch_substs.insert(*expr_id, subst);
                                }
                                found_try_impl = true;
                                break;
                            }
                        }
                    }
                }
                
                let mut resolved_residual_ty = None;
                let output_ty = if !found_try_impl {
                    let mut diag = Diagnostic::error("The `?` operator can only be applied to types that implement `Try`").with_code(DiagnosticCode::InvalidTryOperator);
                    if let Some(span) = self.get_expr_span_for_diag(e) {
                        diag.span = Some(span);
                    }
                    self.ctx.diagnostics.push(diag);
                    self.ctx.types.intern(SemanticType::Error)
                } else {
                    let output_assoc_sym = self.ctx.tables.assoc_type_names.get(&(try_sym, "Output".to_string())).copied();
                    let residual_assoc_sym = self.ctx.tables.assoc_type_names.get(&(try_sym, "Residual".to_string())).copied();

                    if let (Some(out_sym), Some(res_sym)) = (output_assoc_sym, residual_assoc_sym) {
                        let residual_ty = self.normalize_projection(inner_ty_id, try_sym, res_sym, try_span);
                        resolved_residual_ty = Some(residual_ty);
                        self.normalize_projection(inner_ty_id, try_sym, out_sym, try_span)
                    } else {
                        self.ctx.types.intern(SemanticType::Error)
                    }
                };
                
                let current_ret_ty = self.current_return_type.last().copied().unwrap_or_else(|| self.ctx.types.intern(SemanticType::Void));
                let resolved_ret_ty = self.ctx.types.resolve_inference(current_ret_ty);
                let mut found_from_residual_impl = false;

                match self.ctx.types.get(resolved_ret_ty) {
                    SemanticType::GenericParam(gp_sym) => {
                        if let Some(bounds) = self.ctx.tables.trait_bounds.get(gp_sym) {
                            if bounds.iter().any(|b| b.trait_id == from_residual_sym) {
                                found_from_residual_impl = true;
                            }
                        }
                    }
                    SemanticType::InferenceVar(_) | SemanticType::Error => {
                        found_from_residual_impl = true;
                    }
                    _ => {
                        let mut ret_def_sym = None;
                        if let SemanticType::Enum(sym, _, _) | SemanticType::Struct(sym, _, _) = self.ctx.types.get(resolved_ret_ty) {
                            ret_def_sym = Some(*sym);
                        }
                        
                        let from_residual_name = self.ctx.lang_items.get(crate::lang_item::LangItem::FromResidualFn)
                            .map(|s| self.ctx.symbol_table.get_symbol(s).name.as_str().to_string());
                            
                        if let Some(def_sym) = ret_def_sym {
                            let key = crate::semantic_tables::ImplKey {
                                trait_id: Some(from_residual_sym),
                                self_type_def: def_sym.into(),
                            };
                            
                            if let Some(method_syms) = self.ctx.tables.impl_methods.get(&key) {
                                for &m_sym in method_syms {
                                    let m_sym_name = self.ctx.symbol_table.get_symbol(m_sym).name.as_str();
                                    if Some(m_sym_name) == from_residual_name.as_deref() {
                                        let m_decl = self.ctx.tables.symbol_decls.get(&m_sym).copied().unwrap();
                                        let residual_args = resolved_residual_ty.into_iter().collect::<Vec<_>>();
                                        if let Some(subst) = self.impl_method_substitution(
                                            m_decl,
                                            from_residual_sym,
                                            resolved_ret_ty,
                                            &residual_args,
                                        ) {
                                            self.ctx.tables.try_from_residual_methods.insert(*expr_id, m_decl);
                                            self.ctx.tables.try_from_residual_substs.insert(*expr_id, subst);
                                            found_from_residual_impl = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                if !found_from_residual_impl {
                    let mut diag = Diagnostic::error("The `?` operator cannot be used in a function that returns this type, as it does not implement `FromResidual` for the residual type").with_code(DiagnosticCode::InvalidTryOperator);
                    diag.span = Some(try_span);
                    self.ctx.diagnostics.push(diag);
                }
                
                output_ty
            }
            Expr::Await { expr } => {
                if self.current_async_fn.is_none() {
                    let mut diag = Diagnostic::error("`await` is only allowed inside `async` functions").with_code(DiagnosticCode::AwaitOutsideAsync);
                    if let Some(span) = self.get_expr_span_for_diag(expr) {
                        diag.span = Some(span);
                    }
                    self.ctx.diagnostics.push(diag);
                }
                let inner_ty_id = self.typecheck_expr(expr);
                let inner_ty = self.ctx.types.get(inner_ty_id).clone();
                match inner_ty {
                    SemanticType::Future(out_ty) => {
                        let resolved_out = self.ctx.types.resolve_inference(out_ty);
                        if self.ctx.types.is_unsized(resolved_out) {
                            let mut diag = Diagnostic::error("E_UNSIZED_TYPE_IN_VALUE_POSITION: Cannot await a future yielding unsized type in value position").with_code(DiagnosticCode::TypeMismatch);
                            if let Some(span) = self.get_expr_span_for_diag(expr_id) {
                                diag.span = Some(span);
                            }
                            self.ctx.diagnostics.push(diag);
                            self.ctx.types.intern(SemanticType::Error)
                        } else {
                            out_ty
                        }
                    }
                    SemanticType::InferenceVar(var) => {
                        let out_ty = self.ctx.types.new_inference_var();
                        let fut_ty = self.ctx.types.intern(SemanticType::Future(out_ty));
                        self.ctx.types.inference_bindings.insert(var, fut_ty);
                        out_ty
                    }
                    SemanticType::Error => self.ctx.types.intern(SemanticType::Error),
                    _ => {
                        let mut diag = Diagnostic::error("Cannot await a non-future type").with_code(DiagnosticCode::TypeMismatch);
                        if let Some(span) = self.get_expr_span_for_diag(expr) {
                            diag.span = Some(span);
                        }
                        self.ctx.diagnostics.push(diag);
                        self.ctx.types.intern(SemanticType::Error)
                    }
                }
            }
            Expr::Unary { op, operand } => {
                let inner_ty = if *op == luna_ast::expr::UnaryOp::Neg {
                    if let Expr::Literal(token, text) = &self.arena.exprs[operand.0 as usize] {
                        if token.kind == TokenKind::IntegerLiteral {
                            self.typecheck_integer_literal(*operand, text, true)
                        } else { self.typecheck_expr(operand) }
                    } else { self.typecheck_expr(operand) }
                } else if matches!(op, luna_ast::expr::UnaryOp::Ref | luna_ast::expr::UnaryOp::RefMut) {
                    let expected_inner = self.expected_expr_type.last().and_then(|&expected| {
                        match self.ctx.types.get(self.ctx.types.resolve_inference(expected)) {
                            SemanticType::Reference(_, _, inner) => Some(*inner),
                            _ => None,
                        }
                    });
                    if let Some(expected) = expected_inner { self.typecheck_expr_expected(operand, expected) }
                    else { self.typecheck_expr_expected(operand, self.ctx.types.error_id()) }
                } else { self.typecheck_expr(operand) };
                if let Some(message) = crate::operators::PrimitiveOperator::Unary(*op)
                    .error_for_type(&self.ctx.types, inner_ty)
                {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error(message).with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    return self.ctx.types.error_id();
                }

                use luna_ast::expr::UnaryOp;
                match op {
                    UnaryOp::Ref => self.ctx.types.intern(SemanticType::Reference(crate::ty::LifetimeId(0), crate::ty::Mutability::Immutable, inner_ty)),
                    UnaryOp::RefMut => {
                        self.enforce_mutability(operand);
                        self.ctx.types.intern(SemanticType::Reference(crate::ty::LifetimeId(0), crate::ty::Mutability::Mutable, inner_ty))
                    },
                    UnaryOp::Deref | UnaryOp::DerefMut => {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        let inner_ty_kind = self.ctx.types.get(inner_ty).clone();
                        match inner_ty_kind {
                            SemanticType::Pointer(mutability, pointee) => {
                                if !self.is_unsafe_context {
                                    self.ctx.diagnostics.push(Diagnostic::error("E_UNSAFE_DEREF_OUTSIDE_UNSAFE: Dereference of raw pointer requires an unsafe block").with_code(DiagnosticCode::UnsafeOperationOutsideUnsafe)
                                        .with_span(span));
                                }
                                if *op == UnaryOp::DerefMut && mutability == crate::ty::Mutability::Immutable {
                                    self.ctx.diagnostics.push(Diagnostic::error("Cannot perform mutable dereference on immutable raw pointer `*T`").with_code(DiagnosticCode::CannotMutateImmutable)
                                        .with_span(span));
                                }
                                pointee
                            }
                            SemanticType::Reference(_, mutability, pointee) => {
                                if *op == UnaryOp::DerefMut && mutability == crate::ty::Mutability::Immutable {
                                    self.ctx.diagnostics.push(Diagnostic::error("Cannot perform mutable dereference on immutable reference `&T`").with_code(DiagnosticCode::CannotMutateImmutable)
                                        .with_span(span));
                                }
                                pointee
                            }
                            SemanticType::Error => self.ctx.types.error_id(),
                            _ => {
                                self.ctx.diagnostics.push(Diagnostic::error(format!("E_CANNOT_DEREFERENCE: Type `{:?}` cannot be dereferenced", self.ctx.types.get(inner_ty))).with_code(DiagnosticCode::CannotDereference)
                                    .with_span(span));
                                self.ctx.types.error_id()
                            }
                        }
                    }
                    UnaryOp::Neg => {
                        // Check if inner_ty supports Negation (i32, f64, etc.)
                        // For now, since we only have basic types, we just return inner_ty
                        // But we should ban Neg on bools
                        if self.ctx.types.get(inner_ty) == &SemanticType::Primitive(crate::ty::BuiltinType::Bool) {
                            let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                            self.ctx.diagnostics.push(Diagnostic::error("E_INVALID_UNARY_OP: Cannot apply unary operator `-` to type `bool`").with_code(DiagnosticCode::InvalidUnaryOp).with_span(span));
                            self.ctx.types.intern(SemanticType::Error)
                        } else {
                            inner_ty
                        }
                    }
                    UnaryOp::Not | UnaryOp::BitNot | UnaryOp::PostInc | UnaryOp::PostDec => {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Unary operator `{:?}` is not yet supported", op)).with_code(DiagnosticCode::InvalidUnaryOp).with_span(span));
                        self.ctx.types.intern(SemanticType::Error)
                    }
                }
            }
            Expr::Comptime { body } => {
                if self.ctx.tables.expr_types.get(expr_id).is_some_and(|&ty| matches!(self.ctx.types.get(ty), SemanticType::Error)) {
                    return self.ctx.types.error_id();
                }
                self.typecheck_stmt(body);
                let ty = self.infer_stmt_value_type(body).unwrap_or_else(|| self.ctx.types.intern(SemanticType::Void));
                match self.eval_comptime_stmt(*body) {
                    Ok(v) => { self.ctx.comptime_values.insert(*expr_id, v); ty }
                    Err(e) => {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.report_comptime_error(e, span, "cannot evaluate comptime block");
                        self.ctx.types.error_id()
                    }
                }
            }
            other => {
                let mut diag = Diagnostic::error(format!("Unsupported or unrecognized expression construct in semantic phase: {:?}", other)).with_code(DiagnosticCode::InvalidSyntax);
                if let Some(span) = self.get_expr_span_for_diag(expr_id) {
                    diag.span = Some(span);
                }
                self.ctx.diagnostics.push(diag);
                self.ctx.types.intern(SemanticType::Error)
            }
        };
        
        self.ctx.tables.expr_types.insert(*expr_id, ty_id);
        ty_id
    }

    fn constant_float_literal(&self, expr_id: luna_ast::ExprId) -> Option<f64> {
        match &self.arena.exprs[expr_id.0 as usize] {
            Expr::Literal(_, raw) => {
                let normalized = raw.replace('_', "");
                let number = normalized
                    .strip_suffix("f32")
                    .or_else(|| normalized.strip_suffix("f64"))
                    .unwrap_or(&normalized);
                number.parse::<f64>().ok()
            }
            Expr::Unary { op: luna_ast::expr::UnaryOp::Neg, operand } => {
                self.constant_float_literal(*operand).map(|value| -value)
            }
            _ => None,
        }
    }

    fn is_unicode_scalar(value: i128) -> bool {
        (0..=0x10_FFFF).contains(&value) && !(0xD800..=0xDFFF).contains(&value)
    }

    fn impl_method_substitution(
        &self,
        method_decl: luna_ast::DeclId,
        trait_id: luna_common::ids::SymbolId,
        concrete_self: SemanticTypeId,
        concrete_trait_args: &[SemanticTypeId],
    ) -> Option<crate::ty::Substitution> {
        let impl_decl = self.ctx.tables.method_to_impl_decl.get(&method_decl).copied()?;
        let entry = self.ctx.tables.trait_impl_entries.iter().find(|entry| {
            entry.decl_id == Some(impl_decl) && entry.trait_id == trait_id
        })?;
        if entry.trait_args.len() != concrete_trait_args.len() {
            return None;
        }

        let mut subst = crate::ty::Substitution::new();
        if !self.ctx.matches_impl_pattern(entry.self_type, concrete_self, &entry.generic_params, &mut subst) {
            return None;
        }
        for (&pattern_arg, &concrete_arg) in entry.trait_args.iter().zip(concrete_trait_args) {
            if !self.ctx.matches_impl_pattern(pattern_arg, concrete_arg, &entry.generic_params, &mut subst) {
                return None;
            }
        }
        Some(subst)
    }

    fn integer_float_bounds(ty: &SemanticType) -> Option<(f64, f64)> {
        match ty {
            // Upper bounds are exclusive powers of two, avoiding precision loss
            // when i64::MAX/u64::MAX are not exactly representable as f64.
            SemanticType::Primitive(BuiltinType::I8) => Some((-(2.0f64).powi(7), (2.0f64).powi(7))),
            SemanticType::Primitive(BuiltinType::I16) => Some((-(2.0f64).powi(15), (2.0f64).powi(15))),
            SemanticType::Primitive(BuiltinType::I32) => Some((-(2.0f64).powi(31), (2.0f64).powi(31))),
            SemanticType::Primitive(BuiltinType::I64) => Some((-(2.0f64).powi(63), (2.0f64).powi(63))),
            SemanticType::Primitive(BuiltinType::Isize) => Some((-(2.0f64).powi(isize::BITS as i32 - 1), (2.0f64).powi(isize::BITS as i32 - 1))),
            SemanticType::Primitive(BuiltinType::U8) => Some((0.0, (2.0f64).powi(8))),
            SemanticType::Primitive(BuiltinType::U16) => Some((0.0, (2.0f64).powi(16))),
            SemanticType::Primitive(BuiltinType::U32) => Some((0.0, (2.0f64).powi(32))),
            SemanticType::Primitive(BuiltinType::U64) => Some((0.0, (2.0f64).powi(64))),
            SemanticType::Primitive(BuiltinType::Usize) => Some((0.0, (2.0f64).powi(usize::BITS as i32))),
            _ => None,
        }
    }

    fn get_expr_span_for_diag(&self, expr_id: &luna_ast::ExprId) -> Option<luna_common::Span> {
        let expr = self.arena.exprs.get(expr_id.0 as usize)?;
        match expr {
            Expr::Literal(tok, _) => Some(tok.span),
            Expr::Identifier { segments, .. } => segments.first().copied(),
            Expr::Call { callee, .. } => self.get_expr_span_for_diag(callee),
            Expr::MethodCall { method_name, .. } => Some(*method_name),
            Expr::Unary { operand, .. } => self.get_expr_span_for_diag(operand),
            Expr::Cast { expr, .. } | Expr::Await { expr } => self.get_expr_span_for_diag(expr),
            Expr::TupleIndex { object, .. } => self.get_expr_span_for_diag(object),
            Expr::Index { base, .. } => self.get_expr_span_for_diag(base),
            Expr::Member { member, .. } => Some(*member),
            Expr::Binary { left, .. } => self.get_expr_span_for_diag(left),
            Expr::Assign { lvalue, .. } => self.get_expr_span_for_diag(lvalue),
            Expr::StructInit { path, .. } => path.first().copied(),
            Expr::Match { match_span, .. } => Some(*match_span),
            Expr::MacroCall { span, .. } => Some(*span),
            Expr::Try { try_span, .. } => Some(*try_span),
            _ => None,
        }
    }

    pub fn is_place(expr: &Expr) -> bool {
        match expr {
            Expr::Identifier { .. } => true,
            Expr::Unary { op, .. } => matches!(op, luna_ast::expr::UnaryOp::Deref | luna_ast::expr::UnaryOp::DerefMut),
            Expr::Member { .. } => true,
            Expr::TupleIndex { .. } => true,
            Expr::Index { .. } => true,
            _ => false,
        }
    }

    fn enforce_mutability(&mut self, expr_id: &luna_ast::ExprId) {
        let expr = &self.arena.exprs[expr_id.0 as usize];
        match expr {
            luna_ast::Expr::Identifier { segments, .. } => {
                if let Some(sym_id) = self.ctx.tables.expr_symbols.get(expr_id) {
                    let symbol = self.ctx.symbol_table.get_symbol(*sym_id);
                    if matches!(symbol.kind, crate::symbol::SymbolKind::Constant) {
                        let mut diag = luna_common::diagnostic::Diagnostic::error("Cannot mutate immutable variable").with_code(DiagnosticCode::CannotMutateImmutable);
                        if let Some(&span) = segments.first() {
                            diag.span = Some(span);
                        }
                        self.ctx.diagnostics.push(diag);
                    }
                }
            }
            luna_ast::Expr::Member { object, .. } | luna_ast::Expr::TupleIndex { object, .. } => {
                let obj_ty = self.ctx.tables.expr_types.get(object).copied().unwrap_or(crate::ty::SemanticTypeId(0));
                let obj_ty = self.ctx.types.resolve_inference(obj_ty);
                match self.ctx.types.get(obj_ty) {
                    SemanticType::Reference(_, crate::ty::Mutability::Immutable, _) => {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.ctx.diagnostics.push(Diagnostic::error("Cannot mutate through an immutable reference `&T`").with_code(DiagnosticCode::CannotMutateImmutable)
                            .with_span(span));
                    }
                    SemanticType::Reference(_, crate::ty::Mutability::Mutable, _) => {
                        // Mutable reference provides write capability through dereference;
                        // does not mutate the variable holding the reference.
                    }
                    _ => {
                        self.enforce_mutability(object);
                    }
                }
            }
            luna_ast::Expr::Index { base, .. } => {
                let base_ty = self.ctx.tables.expr_types.get(base).copied().unwrap_or(crate::ty::SemanticTypeId(0));
                let base_ty = self.ctx.types.resolve_inference(base_ty);
                match self.ctx.types.get(base_ty) {
                    SemanticType::Reference(_, crate::ty::Mutability::Immutable, _) => {
                        let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                        self.ctx.diagnostics.push(Diagnostic::error("Cannot mutate through an immutable reference `&T`").with_code(DiagnosticCode::CannotMutateImmutable)
                            .with_span(span));
                    }
                    SemanticType::Reference(_, crate::ty::Mutability::Mutable, _) => {}
                    _ => {
                        self.enforce_mutability(base);
                    }
                }
            }
            luna_ast::Expr::Unary { op: luna_ast::expr::UnaryOp::Deref | luna_ast::expr::UnaryOp::DerefMut, operand } => {
                let ptr_ty = self.ctx.tables.expr_types.get(operand).copied().unwrap_or(crate::ty::SemanticTypeId(0));
                let ptr_ty = self.ctx.types.resolve_inference(ptr_ty);
                if let SemanticType::Pointer(crate::ty::Mutability::Immutable, _) = self.ctx.types.get(ptr_ty) {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error("E_CANNOT_MUTATE_IMMUTABLE_POINTER: Cannot borrow through an immutable raw pointer as mutable").with_code(DiagnosticCode::CannotMutateImmutable)
                        .with_span(span));
                }
                if let SemanticType::Reference(_, crate::ty::Mutability::Immutable, _) = self.ctx.types.get(ptr_ty) {
                    let span = self.get_expr_span_for_diag(expr_id).unwrap_or(luna_common::Span::new(luna_common::ids::FileId(0), 0, 0));
                    self.ctx.diagnostics.push(Diagnostic::error("Cannot mutate through an immutable reference `&T`").with_code(DiagnosticCode::CannotMutateImmutable)
                        .with_span(span));
                }
            }
            _ => {}
        }
    }

    pub fn check_field_visibility(
        &mut self,
        struct_sym_id: luna_common::ids::SymbolId,
        field_vis: luna_ast::Visibility,
        field_name: &str,
        span: luna_common::Span,
    ) -> bool {
        let struct_sym = self.ctx.symbol_table.get_symbol(struct_sym_id);
        let is_accessible = match field_vis {
            luna_ast::Visibility::Public => true,
            luna_ast::Visibility::Internal => struct_sym.provider_id == self.ctx.current_provider,
            luna_ast::Visibility::Private => {
                if struct_sym.provider_id.is_some() && struct_sym.provider_id != self.ctx.current_provider {
                    false
                } else {
                    let sym_mod = self.ctx.symbol_table.enclosing_module_scope(struct_sym.scope);
                    let cur_mod = self.ctx.symbol_table.enclosing_module_scope(self.current_scope);
                    sym_mod == cur_mod || self.ctx.symbol_table.is_ancestor(sym_mod, cur_mod)
                }
            }
        };
        if !is_accessible {
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "Field `{}` of struct `{}` is private and cannot be accessed from this scope",
                    field_name, struct_sym.name
                ))
                .with_code(DiagnosticCode::PrivateSymbolAccess)
                .with_span(span),
            );
            false
        } else {
            true
        }
    }
}

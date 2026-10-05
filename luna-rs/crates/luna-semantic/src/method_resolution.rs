//! Concrete method candidate collection and isolated applicability checks.
use super::{AssociatedTypeEqObligation, TypeChecker};
use crate::semantic_tables::{ImplKey, ImplSelfTypeKey, TraitBound};
use crate::ty::{Mutability, SemanticType, SemanticTypeId, Substitution};
use luna_ast::{CallArg, Decl, DeclId, Expr, ExprId, TypeId};
use luna_common::ids::SymbolId;
use luna_common::{Diagnostic, DiagnosticCode, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    method: SymbolId,
    key: ImplKey,
    implementation: Option<DeclId>,
    bound: Option<TraitBound>,
}

impl<'a> TypeChecker<'a> {
    fn concrete_method_candidates(&self, head: ImplSelfTypeKey, name: &str) -> Vec<Candidate> {
        let mut candidates = Vec::new();
        for (key, declarations) in &self.ctx.tables.trait_impls {
            if key.self_type_def != head {
                continue;
            }
            for &implementation in declarations {
                let Some(Decl::Impl { methods, .. }) =
                    self.arena.decls.get(implementation.0 as usize)
                else {
                    continue;
                };
                for declaration in methods {
                    let Some(&method) = self.ctx.tables.decl_symbols.get(declaration) else {
                        continue;
                    };
                    if self.ctx.symbol_table.get_symbol(method).name != name {
                        continue;
                    }
                    let candidate = Candidate {
                        method,
                        key: key.clone(),
                        implementation: Some(implementation),
                        bound: None,
                    };
                    if !candidates.contains(&candidate) {
                        candidates.push(candidate);
                    }
                }
            }
        }
        for (key, methods) in &self.ctx.tables.impl_methods {
            if key.self_type_def != head {
                continue;
            }
            for &method in methods {
                if self.ctx.symbol_table.get_symbol(method).name != name {
                    continue;
                }
                let implementation = self
                    .ctx
                    .tables
                    .method_sym_to_impl_decl
                    .get(&method)
                    .copied();
                let candidate = Candidate {
                    method,
                    key: key.clone(),
                    implementation,
                    bound: None,
                };
                if !candidates
                    .iter()
                    .any(|existing| existing.method == method && existing.key == *key)
                {
                    candidates.push(candidate);
                }
            }
        }
        // Presentation order only. Selection below checks all candidates in a
        // priority class; a session-local numeric identity never chooses one.
        candidates.sort_by_key(|candidate| self.method_candidate_name(candidate));
        candidates
    }

    fn method_candidate_name(&self, candidate: &Candidate) -> String {
        let owner = candidate
            .key
            .trait_id
            .or_else(|| match candidate.key.self_type_def {
                ImplSelfTypeKey::Nominal(symbol) => Some(symbol),
                _ => None,
            });
        let owner = owner
            .map(|symbol| {
                self.ctx
                    .symbol_table
                    .get_full_logical_path(symbol)
                    .join("::")
            })
            .unwrap_or_else(|| format!("{:?}", candidate.key.self_type_def));
        format!(
            "{owner}::{}",
            self.ctx.symbol_table.get_symbol(candidate.method).name
        )
    }

    pub(super) fn resolve_concrete_method(
        &mut self,
        expression: ExprId,
        object: ExprId,
        object_type: SemanticTypeId,
        head: ImplSelfTypeKey,
        name: &str,
        span: Span,
        generics: &[TypeId],
        arguments: &[CallArg],
    ) -> SemanticTypeId {
        let candidates = self.concrete_method_candidates(head, name);
        self.resolve_method_candidates(
            candidates,
            expression,
            object,
            object_type,
            name,
            span,
            generics,
            arguments,
        )
    }

    pub(super) fn resolve_bound_method(
        &mut self,
        expression: ExprId,
        object: ExprId,
        object_type: SemanticTypeId,
        parameter: SymbolId,
        name: &str,
        span: Span,
        generics: &[TypeId],
        arguments: &[CallArg],
    ) -> SemanticTypeId {
        let mut candidates = Vec::new();
        for bound in self
            .ctx
            .tables
            .trait_bounds
            .get(&parameter)
            .into_iter()
            .flatten()
        {
            for &method in self
                .ctx
                .tables
                .trait_methods
                .get(&bound.trait_id)
                .into_iter()
                .flatten()
            {
                if self.ctx.symbol_table.get_symbol(method).name != name {
                    continue;
                }
                let candidate = Candidate {
                    method,
                    implementation: None,
                    bound: Some(bound.clone()),
                    key: ImplKey {
                        trait_id: Some(bound.trait_id),
                        self_type_def: ImplSelfTypeKey::Nominal(parameter),
                    },
                };
                if !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
        candidates.sort_by_key(|candidate| self.method_candidate_name(candidate));
        self.resolve_method_candidates(
            candidates,
            expression,
            object,
            object_type,
            name,
            span,
            generics,
            arguments,
        )
    }

    fn resolve_method_candidates(
        &mut self,
        candidates: Vec<Candidate>,
        expression: ExprId,
        object: ExprId,
        object_type: SemanticTypeId,
        name: &str,
        span: Span,
        generics: &[TypeId],
        arguments: &[CallArg],
    ) -> SemanticTypeId {
        if candidates.len() == 1 {
            return self.apply_concrete_method(
                &candidates[0],
                expression,
                object,
                object_type,
                span,
                generics,
                arguments,
                false,
            );
        }
        for trait_class in [false, true] {
            let mut applicable = Vec::new();
            for candidate in candidates
                .iter()
                .filter(|candidate| candidate.key.trait_id.is_some() == trait_class)
            {
                let errors = self.probe_concrete_method(
                    candidate,
                    expression,
                    object,
                    object_type,
                    span,
                    generics,
                    arguments,
                );
                if errors.is_empty() {
                    applicable.push(candidate);
                }
            }
            if applicable.len() == 1 {
                return self.apply_concrete_method(
                    applicable[0],
                    expression,
                    object,
                    object_type,
                    span,
                    generics,
                    arguments,
                    false,
                );
            }
            if applicable.len() > 1 {
                let names = applicable
                    .iter()
                    .map(|candidate| self.method_candidate_name(candidate))
                    .collect::<Vec<_>>();
                let mut diagnostic =
                    Diagnostic::error(format!("Ambiguous method `{name}`: {}", names.join(", ")))
                        .with_code(DiagnosticCode::AmbiguousSymbol)
                        .with_span(span);
                for candidate in applicable {
                    diagnostic = diagnostic.with_related(
                        self.ctx.symbol_table.get_symbol(candidate.method).span,
                        format!("candidate {}", self.method_candidate_name(candidate)),
                    );
                }
                self.ctx.diagnostics.push(diagnostic);
                return self.ctx.types.error_id();
            }
        }
        // Replay one canonical rejection through the real checker. Speculative
        // types/IDs from a discarded context must never enter this context.
        if let Some(candidate) = candidates.first() {
            return self.apply_concrete_method(
                candidate,
                expression,
                object,
                object_type,
                span,
                generics,
                arguments,
                false,
            );
        }
        for argument in arguments {
            self.typecheck_expr(&argument.value);
        }
        self.ctx.diagnostics.push(
            Diagnostic::error(format!(
                "Method `{name}` not found for type `{:?}`",
                self.ctx.types.get(object_type)
            ))
            .with_code(DiagnosticCode::UnresolvedSymbol)
            .with_span(span),
        );
        self.ctx.types.error_id()
    }

    fn probe_concrete_method(
        &self,
        candidate: &Candidate,
        expression: ExprId,
        object: ExprId,
        object_type: SemanticTypeId,
        span: Span,
        generics: &[TypeId],
        arguments: &[CallArg],
    ) -> Vec<Diagnostic> {
        let mut context = self.ctx.clone();
        let diagnostic_start = context.diagnostics.len();
        let mut checker = TypeChecker {
            ctx: &mut context,
            arena: self.arena,
            source_manager: self.source_manager,
            is_unsafe_context: self.is_unsafe_context,
            loop_depth: self.loop_depth,
            active_lambdas: self.active_lambdas.clone(),
            current_self_type: self.current_self_type,
            current_async_fn: self.current_async_fn,
            comptime_engine: self.comptime_engine,
            current_return_type: self.current_return_type.clone(),
            expected_expr_type: Vec::new(),
            comptime_prepared_functions: self.comptime_prepared_functions.clone(),
            populating_nominal_types: self.populating_nominal_types.clone(),
            populated_nominal_types: self.populated_nominal_types.clone(),
            current_scope: self.current_scope,
            current_trait: self.current_trait,
            current_trait_impl: self.current_trait_impl,
            normalization_stack: self.normalization_stack.clone(),
            associated_type_obligations: Vec::<AssociatedTypeEqObligation>::new(),
        };
        checker.apply_concrete_method(
            candidate,
            expression,
            object,
            object_type,
            span,
            generics,
            arguments,
            true,
        );
        checker.solve_associated_type_obligations();
        checker.ctx.diagnostics[diagnostic_start..]
            .iter()
            .filter(|diagnostic| {
                diagnostic.level == luna_common::diagnostic::DiagnosticLevel::Error
            })
            .cloned()
            .collect()
    }

    fn method_receiver_type(&self, mut ty: SemanticTypeId) -> SemanticTypeId {
        loop {
            ty = self.ctx.types.resolve(ty);
            match self.ctx.types.get(ty) {
                SemanticType::Reference(_, _, inner) | SemanticType::Pointer(_, inner) => {
                    ty = *inner
                }
                _ => return ty,
            }
        }
    }

    fn method_receiver_is_mutable(&self, expression: ExprId) -> bool {
        if let Some(&ty) = self.ctx.tables.expr_types.get(&expression) {
            match self.ctx.types.get(self.ctx.types.resolve(ty)) {
                SemanticType::Reference(_, mutability, _)
                | SemanticType::Pointer(mutability, _) => {
                    return *mutability == Mutability::Mutable;
                }
                _ => {}
            }
        }
        match &self.arena.exprs[expression.0 as usize] {
            Expr::Identifier { .. } => self
                .ctx
                .tables
                .expr_symbols
                .get(&expression)
                .and_then(|symbol| self.ctx.tables.symbol_decls.get(symbol))
                .and_then(|decl| self.arena.decls.get(decl.0 as usize))
                .map_or(true, |decl| match decl {
                    Decl::Var {
                        is_mutable,
                        is_const,
                        ..
                    } => *is_mutable && !*is_const,
                    _ => true,
                }),
            Expr::Member { object, .. } | Expr::TupleIndex { object, .. } => {
                self.method_receiver_is_mutable(*object)
            }
            Expr::Index { base, .. } => self.method_receiver_is_mutable(*base),
            _ => true,
        }
    }

    fn apply_concrete_method(
        &mut self,
        candidate: &Candidate,
        expression: ExprId,
        object: ExprId,
        object_type: SemanticTypeId,
        span: Span,
        generics: &[TypeId],
        arguments: &[CallArg],
        probe: bool,
    ) -> SemanticTypeId {
        let error_start = self.ctx.diagnostics.len();
        let access = candidate.key.trait_id.unwrap_or(candidate.method);
        if !self.ctx.symbol_table.is_accessible(
            access,
            self.current_scope,
            self.ctx.current_provider,
        ) {
            self.ctx.diagnostics.push(
                Diagnostic::error("Method is private and cannot be accessed from this scope")
                    .with_code(DiagnosticCode::PrivateSymbolAccess)
                    .with_span(span),
            );
            return self.ctx.types.error_id();
        }
        let Some(&function_type) = self.ctx.tables.symbol_types.get(&candidate.method) else {
            self.ctx.diagnostics.push(
                Diagnostic::error("Method candidate has no checked signature")
                    .with_code(DiagnosticCode::TypeMismatch)
                    .with_span(span),
            );
            return self.ctx.types.error_id();
        };
        let SemanticType::Function { params, return_type, .. } = self.ctx.types.get(function_type).clone()
        else {
            self.ctx.diagnostics.push(
                Diagnostic::error("Method candidate is not callable")
                    .with_code(DiagnosticCode::TypeMismatch)
                    .with_span(span),
            );
            return self.ctx.types.error_id();
        };
        let declaration = self.ctx.tables.symbol_decls.get(&candidate.method).copied();
        let receiver_declared = declaration
            .and_then(|declaration| self.arena.decls.get(declaration.0 as usize))
            .and_then(|decl| {
                if let Decl::Function { params, .. } = decl {
                    params.first()
                } else {
                    None
                }
            })
            .and_then(|decl| self.arena.decls.get(decl.0 as usize))
            .is_some_and(|decl| matches!(decl, Decl::Param { is_self: true, .. }));
        if !receiver_declared || params.len() != arguments.len() + 1 {
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "Method requires a receiver and {} arguments; got {}",
                    params.len().saturating_sub(1),
                    arguments.len()
                ))
                .with_code(DiagnosticCode::TypeMismatch)
                .with_span(span),
            );
            return self.ctx.types.error_id();
        }
        let mut subst = Substitution::new();
        let header = candidate.implementation.and_then(|implementation| {
            self.ctx
                .tables
                .checked_impl_headers
                .get(&implementation)
                .cloned()
        });
        let (pattern, impl_parameters) = if let Some(header) = header {
            (Some(header.self_type), header.generic_params)
        } else if candidate.bound.is_some() {
            (None, Vec::new())
        } else if candidate.implementation.is_none() {
            (
                self.ctx.tables.impl_self_types.get(&candidate.key).copied(),
                self.ctx
                    .tables
                    .impl_generic_params
                    .get(&candidate.key)
                    .cloned()
                    .unwrap_or_default(),
            )
        } else {
            self.ctx.diagnostics.push(
                Diagnostic::error("Method implementation has no checked header")
                    .with_code(DiagnosticCode::TypeMismatch)
                    .with_span(span),
            );
            return self.ctx.types.error_id();
        };
        let receiver = self.method_receiver_type(object_type);
        if let Some(pattern) = pattern {
            if !self
                .ctx
                .matches_impl_pattern(pattern, receiver, &impl_parameters, &mut subst)
            {
                self.ctx.diagnostics.push(
                    Diagnostic::error("Method implementation does not apply to this receiver")
                        .with_code(DiagnosticCode::TypeMismatch)
                        .with_span(span),
                );
                return self.ctx.types.error_id();
            }
        }
        if let (Some(trait_id), Some(implementation)) =
            (candidate.key.trait_id, candidate.implementation)
        {
            if let Some(entry) = self
                .ctx
                .tables
                .trait_impl_entries
                .iter()
                .find(|entry| entry.decl_id == Some(implementation))
                .cloned()
            {
                let parameters = self
                    .ctx
                    .tables
                    .trait_generic_params
                    .get(&trait_id)
                    .cloned()
                    .unwrap_or_default();
                for (parameter, argument) in parameters.into_iter().zip(entry.trait_args) {
                    let argument = self.ctx.types.subst(argument, &subst);
                    subst.insert(parameter, argument);
                }
            }
        }
        let mut method_parameters = Vec::new();
        if let Some(declaration) = declaration {
            let mut index = 0;
            while let Some(&parameter) = self
                .ctx
                .tables
                .generic_param_symbols
                .get(&(declaration, index))
            {
                method_parameters.push(parameter);
                index += 1;
            }
        }
        if !generics.is_empty() && generics.len() != method_parameters.len() {
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "Wrong number of method generic arguments: expected {}, got {}",
                    method_parameters.len(),
                    generics.len()
                ))
                .with_code(DiagnosticCode::TypeMismatch)
                .with_span(span),
            );
            return self.ctx.types.error_id();
        }
        for (index, &parameter) in method_parameters.iter().enumerate() {
            let ty = if let Some(&ty) = generics.get(index) {
                self.lower_type(ty)
            } else {
                self.ctx.types.new_inference_var()
            };
            subst.insert(parameter, ty);
        }
        if let Some(bound) = &candidate.bound {
            let parameters = self
                .ctx
                .tables
                .trait_generic_params
                .get(&bound.trait_id)
                .cloned()
                .unwrap_or_else(|| {
                    self.ctx
                        .tables
                        .symbol_decls
                        .get(&bound.trait_id)
                        .map(|declaration| {
                            (0..bound.trait_args.len())
                                .filter_map(|index| {
                                    self.ctx
                                        .tables
                                        .generic_param_symbols
                                        .get(&(*declaration, index))
                                        .copied()
                                })
                                .collect()
                        })
                        .unwrap_or_default()
                });
            for (parameter, &argument) in parameters.into_iter().zip(&bound.trait_args) {
                subst.insert(parameter, argument);
            }
            let mut has_generics = false;
            self.bind_matching_generics(params[0], object_type, &mut subst, &mut has_generics);
        }
        let expected_receiver = self.ctx.types.subst(params[0], &subst);
        if matches!(
            self.ctx.types.get(expected_receiver),
            SemanticType::Reference(_, Mutability::Mutable, _)
                | SemanticType::Pointer(Mutability::Mutable, _)
        ) {
            if !self.method_receiver_is_mutable(object) {
                self.ctx.diagnostics.push(Diagnostic::error("E_CANNOT_MUTATE_IMMUTABLE_POINTER: Cannot call a method requiring a mutable receiver through an immutable reference or pointer")
                    .with_code(DiagnosticCode::CannotMutateImmutable).with_span(span));
            } else if !matches!(
                self.ctx.types.get(self.ctx.types.resolve(object_type)),
                SemanticType::Reference(_, Mutability::Mutable, _)
                    | SemanticType::Pointer(Mutability::Mutable, _)
            ) {
                // Mutating through a capability does not reassign the binding
                // holding that reference or pointer.
                self.enforce_mutability(&object);
            }
        }
        let expected_receiver = self.method_receiver_type(expected_receiver);
        if let Err(reason) = self.unify(expected_receiver, receiver) {
            self.ctx.diagnostics.push(
                Diagnostic::error(reason)
                    .with_code(DiagnosticCode::TypeMismatch)
                    .with_span(span),
            );
        }
        for (&parameter, argument) in params[1..].iter().zip(arguments) {
            let expected = self.ctx.types.subst(parameter, &subst);
            let actual = self.typecheck_expr_expected(&argument.value, expected);
            if !self.try_coerce(argument.value, actual, expected) {
                if let Err(reason) = self.unify(expected, actual) {
                    self.ctx.diagnostics.push(
                        Diagnostic::error(reason)
                            .with_code(DiagnosticCode::TypeMismatch)
                            .with_span(
                                self.get_expr_span_for_diag(&argument.value).unwrap_or(span),
                            ),
                    );
                }
            }
        }
        // A result context may complete inference only after candidate choice.
        // It must never eliminate a same-named trait during applicability probes.
        if !probe {
            if let Some(expected) = self.expected_expr_type.last().copied() {
                let result = self.ctx.types.subst(return_type, &subst);
                let _ = self.unify(result, expected);
            }
            for parameter in &method_parameters {
                if let Some(&inferred) = subst.get(*parameter) {
                    if matches!(
                        self.ctx.types.get(self.ctx.types.resolve(inferred)),
                        SemanticType::InferenceVar(_)
                    ) {
                        self.ctx.diagnostics.push(Diagnostic::error(format!(
                            "E_UNCONSTRAINED_INFERENCE: Generic method type parameter `{}` of `{}` could not be inferred from arguments or expected result",
                            self.ctx.symbol_table.get_symbol(*parameter).name, self.method_candidate_name(candidate)))
                            .with_code(DiagnosticCode::TypeMismatch).with_span(span));
                    }
                }
            }
        }
        if let Some(implementation) = candidate.implementation {
            self.check_bounds_for_decl(implementation, &subst, span);
        }
        self.check_bounds_for_call(candidate.method, &subst, span);
        if !probe {
            if self
                .ctx
                .tables
                .drop_impls
                .values()
                .any(|&method| method == candidate.method)
                || Some(candidate.method)
                    == self.ctx.lang_items.get(crate::lang_item::LangItem::DropFn)
            {
                self.ctx.diagnostics.push(
                    Diagnostic::error("Explicit destructor calls are forbidden")
                        .with_code(DiagnosticCode::ExplicitDropCall)
                        .with_span(span),
                );
            }
            if self.ctx.tables.unsafe_functions.contains(&candidate.method)
                && !self.is_unsafe_context
            {
                self.ctx.diagnostics.push(
                    Diagnostic::error("Call to unsafe method requires an unsafe block")
                        .with_code(DiagnosticCode::UnsafeOperationOutsideUnsafe)
                        .with_span(span),
                );
            }
        }
        if self.ctx.diagnostics[error_start..]
            .iter()
            .any(|diagnostic| diagnostic.level == luna_common::diagnostic::DiagnosticLevel::Error)
        {
            return self.ctx.types.error_id();
        }
        let mut resolved = Substitution::new();
        for (symbol, ty) in &subst.map {
            resolved.insert(
                *symbol,
                self.ctx.types.subst(self.ctx.types.resolve(*ty), &subst),
            );
        }
        let result = self.ctx.types.subst(return_type, &resolved);
        self.ctx
            .tables
            .expr_symbols
            .insert(expression, candidate.method);
        if !resolved.map.is_empty() {
            self.ctx.tables.expr_substs.insert(expression, resolved);
        }
        self.ctx.tables.expr_types.insert(expression, result);
        result
    }
}

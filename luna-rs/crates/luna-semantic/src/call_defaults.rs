use super::TypeChecker;
use luna_ast::{Decl, DeclId};
use luna_common::{Diagnostic, DiagnosticCode};
use luna_lexer::{Lexer, TokenKind};
use std::collections::HashMap;

impl TypeChecker<'_> {
    pub(super) fn parameter_default_contracts(&self, declaration: DeclId) -> Vec<Option<String>> {
        let Decl::Function {
            params,
            generic_params,
            ..
        } = &self.arena.decls[declaration.0 as usize]
        else {
            return Vec::new();
        };
        if params.iter().all(|parameter| {
            matches!(
                self.arena.decls[parameter.0 as usize],
                Decl::Param { default: None, .. }
            )
        }) {
            return vec![None; params.len()];
        }
        let mut roles = HashMap::new();
        let owner = self
            .ctx
            .tables
            .method_to_impl_decl
            .get(&declaration)
            .copied()
            .or_else(|| {
                let method = self.ctx.tables.decl_symbols.get(&declaration)?;
                let trait_owner = self
                    .ctx
                    .tables
                    .trait_methods
                    .iter()
                    .find(|(_, methods)| methods.contains(method))
                    .map(|(&owner, _)| owner)?;
                self.ctx.tables.symbol_decls.get(&trait_owner).copied()
            });
        if let Some(owner) = owner {
            let mut ordinal = 0;
            while let Some(&symbol) = self.ctx.tables.generic_param_symbols.get(&(owner, ordinal)) {
                roles.insert(symbol, format!("$owner{ordinal}"));
                ordinal += 1;
            }
        }
        for (ordinal, _) in generic_params.iter().enumerate() {
            if let Some(&symbol) = self
                .ctx
                .tables
                .generic_param_symbols
                .get(&(declaration, ordinal))
            {
                roles.insert(symbol, format!("$generic{ordinal}"));
            }
        }
        for (ordinal, parameter) in params.iter().enumerate() {
            if let Some(&symbol) = self.ctx.tables.decl_symbols.get(parameter) {
                roles.insert(symbol, format!("$parameter{ordinal}"));
            }
        }
        let mut token_roles = HashMap::new();
        let mut declaration_tokens = HashMap::new();
        for (expression, symbol) in &self.ctx.tables.expr_symbols {
            if let Some(role) = roles.get(symbol) {
                if let luna_ast::Expr::Identifier { segments, .. } =
                    &self.arena.exprs[expression.0 as usize]
                {
                    if let [span] = segments.as_slice() {
                        token_roles.insert((span.file_id, span.start, span.end), role.as_str());
                    }
                }
            } else if matches!(
                self.ctx.symbol_table.get_symbol(*symbol).kind,
                crate::SymbolKind::Function
                    | crate::SymbolKind::ExternFunction
                    | crate::SymbolKind::TraitMethod
                    | crate::SymbolKind::Struct
                    | crate::SymbolKind::Enum
                    | crate::SymbolKind::EnumVariant(_)
                    | crate::SymbolKind::Trait
                    | crate::SymbolKind::AssociatedType
                    | crate::SymbolKind::Alias
            ) || (matches!(
                self.ctx.symbol_table.get_symbol(*symbol).kind,
                crate::SymbolKind::Constant | crate::SymbolKind::Variable
            ) && matches!(
                self.ctx.symbol_table.scopes
                    [self.ctx.symbol_table.get_symbol(*symbol).scope.0 as usize]
                    .kind,
                crate::symbol::ScopeKind::Global | crate::symbol::ScopeKind::Module
            )) {
                if let luna_ast::Expr::Identifier { segments, .. } =
                    &self.arena.exprs[expression.0 as usize]
                {
                    for (ordinal, segment) in segments.iter().enumerate() {
                        let identity = if ordinal == 0 {
                            Some(
                                self.ctx
                                    .symbol_table
                                    .get_full_logical_path(*symbol)
                                    .join("::"),
                            )
                        } else {
                            None
                        };
                        declaration_tokens
                            .insert((segment.file_id, segment.start, segment.end), identity);
                    }
                }
            }
        }
        for (ty, symbol) in &self.ctx.tables.type_symbols {
            if let Some(role) = roles.get(symbol) {
                if let luna_ast::Type::Named { segments, .. } = &self.arena.types[ty.0 as usize] {
                    if let [span] = segments.as_slice() {
                        token_roles.insert((span.file_id, span.start, span.end), role.as_str());
                    }
                }
            }
        }
        params
            .iter()
            .map(|parameter| {
                let Decl::Param {
                    default: Some(default),
                    ..
                } = &self.arena.decls[parameter.0 as usize]
                else {
                    return None;
                };
                let source = self.get_span_text(default.span);
                let mut contract = String::new();
                for token in Lexer::new(source, default.span.file_id) {
                    if token.kind == TokenKind::Eof {
                        break;
                    }
                    let text = &source[token.span.start as usize..token.span.end as usize];
                    let key = (
                        default.span.file_id,
                        default.span.start + token.span.start,
                        default.span.start + token.span.end,
                    );
                    if let Some(identity) = declaration_tokens.get(&key) {
                        if let Some(identity) = identity {
                            contract.push_str(&format!("Binding:{}:{identity}|", identity.len()));
                        }
                        continue;
                    }
                    let canonical = token_roles.get(&key).copied().unwrap_or(text);
                    contract.push_str(&format!(
                        "{:?}:{}:{canonical}|",
                        token.kind,
                        canonical.len()
                    ));
                }
                Some(contract)
            })
            .collect()
    }

    pub(super) fn typecheck_parameter_defaults(&mut self, declaration: DeclId) {
        let Decl::Function { params, name, .. } = &self.arena.decls[declaration.0 as usize] else {
            return;
        };
        let params = params.clone();
        let name = *name;
        let mut has_default = false;
        for parameter in params {
            let Decl::Param {
                default,
                is_self,
                name,
                ty,
                ..
            } = &self.arena.decls[parameter.0 as usize]
            else {
                continue;
            };
            if let Some(default) = default {
                has_default = true;
                if *is_self {
                    self.ctx.diagnostics.push(
                        Diagnostic::error("Receiver parameter `self` cannot have a default")
                            .with_code(DiagnosticCode::TypeMismatch)
                            .with_span(default.span),
                    );
                }
                if let Some(ty) = ty {
                    let expected = self.lower_type(*ty);
                    let actual = self.typecheck_expr_expected(&default.value, expected);
                    if let Err(error) = self.unify(expected, actual) {
                        self.ctx.diagnostics.push(Diagnostic::error(format!("Default value does not satisfy its declaration parameter type: {error}"))
                            .with_code(DiagnosticCode::TypeMismatch).with_span(default.span));
                    }
                }
            } else if has_default {
                self.ctx.diagnostics.push(
                    Diagnostic::error("Required parameters must precede defaulted parameters")
                        .with_code(DiagnosticCode::TypeMismatch)
                        .with_span(*name),
                );
            }
        }
        if let Some(&symbol) = self.ctx.tables.decl_symbols.get(&declaration) {
            // Method selection and definition-site helper identities are now checked.
            self.register_callable_signature(symbol);
            let contract = self
                .ctx
                .tables
                .callable_contract_symbol(symbol, &self.ctx.symbol_table);
            if contract != symbol {
                if let Some(&trait_declaration) = self.ctx.tables.symbol_decls.get(&contract) {
                    let implementation_defaults = self.parameter_default_contracts(declaration);
                    let trait_defaults = self.parameter_default_contracts(trait_declaration);
                    if implementation_defaults
                        .iter()
                        .enumerate()
                        .any(|(ordinal, value)| {
                            value.is_some() && trait_defaults.get(ordinal) != Some(value)
                        })
                    {
                        self.ctx.diagnostics.push(Diagnostic::error("A trait implementation cannot change the trait signature's defaults")
                            .with_code(DiagnosticCode::TypeMismatch).with_span(name));
                    }
                }
            }
        }
    }
}

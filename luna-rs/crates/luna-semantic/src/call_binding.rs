use super::TypeChecker;
use crate::semantic_tables::{CallArgumentBinding, CallableSignature};
use luna_ast::{CallArg, Decl, ExprId};
use luna_common::ids::SymbolId;
use luna_common::{Diagnostic, DiagnosticCode, Span};

impl TypeChecker<'_> {
    pub(super) fn register_callable_signature(&mut self, symbol: SymbolId) {
        let Some(declaration) = self.ctx.tables.symbol_decls.get(&symbol) else {
            return;
        };
        let Some(Decl::Function {
            params,
            is_variadic,
            ..
        }) = self.arena.decls.get(declaration.0 as usize)
        else {
            return;
        };
        let mut parameter_names = Vec::new();
        let default_contracts = self.parameter_default_contracts(*declaration);
        let mut has_receiver = false;
        for (index, parameter) in params.iter().enumerate() {
            if let Decl::Param { name, is_self, .. } = self.arena.decls[parameter.0 as usize] {
                parameter_names.push(self.get_span_text(name).to_owned());
                has_receiver |= index == 0 && is_self;
            }
        }
        self.ctx.tables.callable_signatures.insert(
            symbol,
            CallableSignature {
                parameter_names,
                default_contracts,
                has_receiver,
                is_variadic: *is_variadic,
            },
        );
    }

    pub(super) fn bind_call_arguments(
        &mut self,
        expression: ExprId,
        symbol: Option<SymbolId>,
        implicit_receiver: bool,
        parameter_count: usize,
        variadic: bool,
        arguments: &[CallArg],
        span: Span,
    ) -> Option<CallArgumentBinding> {
        if let Some(symbol) = symbol {
            self.register_callable_signature(symbol);
        }
        let signature = symbol.and_then(|symbol| {
            self.ctx
                .tables
                .callable_signature(symbol, &self.ctx.symbol_table)
                .cloned()
        });
        if signature.as_ref().is_some_and(|signature| {
            signature.parameter_names.len() != parameter_count + usize::from(implicit_receiver)
                || (implicit_receiver && !signature.has_receiver)
        }) {
            self.ctx.diagnostics.push(
                Diagnostic::error(
                    "Callable declaration contract disagrees with its checked parameter types",
                )
                .with_code(DiagnosticCode::BackendInvariantViolation)
                .with_span(span),
            );
            return None;
        }
        let names = signature
            .as_ref()
            .map(|signature| &signature.parameter_names[usize::from(implicit_receiver)..]);
        let mut seen_named = false;
        let mut occupied = vec![false; parameter_count.max(arguments.len())];
        let mut source_to_parameter = Vec::new();
        for (source_index, argument) in arguments.iter().enumerate() {
            let (ordinal, argument_span) = if let Some(label) = argument.label {
                seen_named = true;
                let Some(names) = names else {
                    self.ctx.diagnostics.push(Diagnostic::error("Named arguments require a declaration signature; structural callable types have no parameter names")
                        .with_code(DiagnosticCode::TypeMismatch).with_span(label));
                    return None;
                };
                let name = self.get_span_text(label);
                let Some(ordinal) = names.iter().position(|parameter| parameter == name) else {
                    self.ctx.diagnostics.push(
                        Diagnostic::error(format!("Unknown argument label `{name}`"))
                            .with_code(DiagnosticCode::TypeMismatch)
                            .with_span(label),
                    );
                    return None;
                };
                (ordinal, label)
            } else {
                let argument_span = self.get_expr_span_for_diag(&argument.value).unwrap_or(span);
                if seen_named {
                    self.ctx.diagnostics.push(
                        Diagnostic::error("Positional arguments must precede named arguments")
                            .with_code(DiagnosticCode::TypeMismatch)
                            .with_span(argument_span),
                    );
                    return None;
                }
                (source_index, argument_span)
            };
            if ordinal >= occupied.len() || (!variadic && ordinal >= parameter_count) {
                self.ctx.diagnostics.push(
                    Diagnostic::error(format!(
                        "Callable expects {parameter_count} arguments, but {} were provided",
                        arguments.len()
                    ))
                    .with_code(DiagnosticCode::TypeMismatch)
                    .with_span(argument_span),
                );
                return None;
            }
            if occupied[ordinal] {
                self.ctx.diagnostics.push(
                    Diagnostic::error("A parameter cannot receive more than one argument")
                        .with_code(DiagnosticCode::TypeMismatch)
                        .with_span(argument_span),
                );
                return None;
            }
            occupied[ordinal] = true;
            source_to_parameter.push(ordinal as u32);
        }
        if occupied[..parameter_count].iter().any(|bound| !bound) {
            self.ctx.diagnostics.push(
                Diagnostic::error(format!(
                    "Callable expects {}{parameter_count} arguments, but {} were provided",
                    if variadic { "at least " } else { "" },
                    arguments.len()
                ))
                .with_code(DiagnosticCode::TypeMismatch)
                .with_span(span),
            );
            return None;
        }
        let binding = CallArgumentBinding {
            source_to_parameter,
        };
        self.ctx
            .tables
            .call_argument_bindings
            .insert(expression, binding.clone());
        Some(binding)
    }
}

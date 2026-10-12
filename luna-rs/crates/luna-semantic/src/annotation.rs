//! Attribute processing for the Luna compiler.
//!
//! This module handles:
//! - `#[derive(...)]` - attribute-driven derive macro expansion
//! Contextual application admission is owned by TypeChecker; compiler binding
//! admission is owned by Resolver. This module only transforms declarations.
//!
//! The derive system uses a registry-based approach where derive macros are
//! registered by name and invoked generically. See `derive.rs` for details.

use std::collections::HashSet;
use luna_ast::{AstArena, Decl, DeclId, Expr, Item};
use luna_common::ids::{FileId, Span};

use crate::derive::{AttributeExpansionFailure, DeriveRegistry, DeriveContext, extract_struct_input, extract_enum_input};

/// Attribute transformation only. Application failures remain structured data;
/// TypeChecker's preparation admission emits their diagnostics before resolution.
pub struct AttributeProcessor<'a> {
    arena: &'a mut AstArena,
    source_manager: &'a mut luna_common::source::SourceManager,
    file_id: FileId,
    diagnostics: Vec<AttributeExpansionFailure>,
    /// Registry of available derive macros
    derive_registry: DeriveRegistry,
    /// Tracks which traits have been emitted to prevent duplicates
    emitted_traits: HashSet<String>,
    /// Counter for expansion hygiene
    expansion_counter: u32,
}

impl<'a> AttributeProcessor<'a> {
    /// Create a new AttributeProcessor with standard library derives pre-registered.
    pub fn new(arena: &'a mut AstArena, source_manager: &'a mut luna_common::source::SourceManager, file_id: FileId) -> Self {
        Self {
            arena,
            source_manager,
            file_id,
            diagnostics: Vec::new(),
            derive_registry: DeriveRegistry::with_std_derives(),
            emitted_traits: HashSet::new(),
            expansion_counter: 0,
        }
    }

    /// Create a new AttributeProcessor with a custom derive registry.
    ///
    /// This allows tests or external tools to provide their own derive implementations.
    pub fn with_registry(
        arena: &'a mut AstArena,
        source_manager: &'a mut luna_common::source::SourceManager,
        file_id: FileId,
        registry: DeriveRegistry,
    ) -> Self {
        Self {
            arena,
            source_manager,
            file_id,
            diagnostics: Vec::new(),
            derive_registry: registry,
            emitted_traits: HashSet::new(),
            expansion_counter: 0,
        }
    }

    /// Access the underlying derive registry (for testing or external registration).
    pub fn derive_registry_mut(&mut self) -> &mut DeriveRegistry {
        &mut self.derive_registry
    }

    /// Access the underlying derive registry (immutable reference).
    pub fn derive_registry(&self) -> &DeriveRegistry {
        &self.derive_registry
    }

    /// Expand derives, transporting contextual failures without diagnostic codes.
    pub fn process_items(&mut self, items: Vec<Item>) -> Result<Vec<Item>, Vec<AttributeExpansionFailure>> {
        let mut generated_items = Vec::new();
        let mut out_items = Vec::new();

        for item in &items {
            out_items.push(item.clone());
            if let Item::Decl(decl_id) = item {
                let decl = self.arena.decls[decl_id.0 as usize].clone();
                let extra = self.process_decl(*decl_id, &decl);
                generated_items.extend(extra);
            }
        }

        out_items.extend(generated_items);

        if self.diagnostics.is_empty() {
            Ok(out_items)
        } else {
            Err(self.diagnostics.clone())
        }
    }

    /// Get text from a span.
    fn get_span_text(&self, span: Span) -> &str {
        if (span.end as usize) <= self.source_manager.get_file(span.file_id).unwrap().source.len() && span.start <= span.end {
            &self.source_manager.get_file(span.file_id).unwrap().source[span.start as usize..span.end as usize]
        } else {
            ""
        }
    }

    /// Expand derives; binding and application validation belongs to Resolver
    /// and TypeChecker respectively, including AST materialized after this pass.
    fn process_decl(&mut self, decl_id: DeclId, decl: &Decl) -> Vec<Item> {
        let mut generated = Vec::new();
        for annot in decl.annotations() {
            if self.get_span_text(annot.name) == "derive" {
                generated.extend(self.handle_derive(decl_id, decl, annot));
            }
        }
        generated
    }

    /// Handle #[derive(...)] attribute using the generic derive system.
    ///
    /// This dispatches to the DeriveRegistry rather than hardcoding derive implementations.
    fn handle_derive(&mut self, _decl_id: DeclId, decl: &Decl, annot: &luna_ast::Annotation) -> Vec<Item> {
        // Only structs and enums can derive
        let derive_input = match decl {
            Decl::Struct { .. } => extract_struct_input(self.arena, &self.source_manager.get_file(self.file_id).unwrap().source, decl),
            Decl::Enum { .. } => extract_enum_input(self.arena, &self.source_manager.get_file(self.file_id).unwrap().source, decl),
            _ => {
                self.diagnostics.push(
                    AttributeExpansionFailure::application("`#[derive]` can only be applied to structs or enums", annot.name),
                );
                return Vec::new();
            }
        };

        let Some(input) = derive_input else {
            return Vec::new();
        };

        let mut generated = Vec::new();

        for arg in &annot.args {
            let trait_name = self.get_expr_identifier_text(arg.value);

            // Check if the derive macro is registered
            if !self.derive_registry.is_registered(&trait_name) {
                let span = self.get_expr_span(arg.value).unwrap_or(annot.name);
                let available = self.derive_registry.registered_names();
                let msg = if available.is_empty() {
                    format!("unknown derive trait `{}`", trait_name)
                } else {
                    format!(
                        "unknown derive trait `{}`. Available derives: {}",
                        trait_name,
                        available.join(", ")
                    )
                };
                self.diagnostics.push(AttributeExpansionFailure::application(msg, span));
                continue;
            }

            // Get the derive macro from the registry
            let Some(derive_fn) = self.derive_registry.get(&trait_name) else {
                continue;
            };

            // Create the derive context
            let mut ctx = DeriveContext::new(
                self.arena,
                self.source_manager,
                self.file_id,
                annot.name,
                &mut self.expansion_counter,
                &mut self.diagnostics,
                &mut self.emitted_traits,
            );

            // Invoke the derive macro
            match derive_fn(&mut ctx, &input) {
                Ok(items) => {
                    generated.extend(ctx.extra_items);
                    generated.extend(items);
                }
                Err(diag) => {
                    self.diagnostics.push(diag);
                }
            }
        }

        generated
    }

    /// Extract identifier text from an expression (for attribute arguments).
    fn get_expr_identifier_text(&self, expr_id: luna_ast::ExprId) -> String {
        match &self.arena.exprs[expr_id.0 as usize] {
            Expr::Identifier { segments, .. } => {
                if let Some(first) = segments.first() {
                    self.get_span_text(*first).to_string()
                } else {
                    String::new()
                }
            }
            Expr::Literal(tok, _) => {
                self.get_span_text(tok.span).to_string()
            }
            _ => String::new(),
        }
    }

    /// Get the span for an expression.
    fn get_expr_span(&self, expr_id: luna_ast::ExprId) -> Option<Span> {
        match &self.arena.exprs[expr_id.0 as usize] {
            Expr::Identifier { segments, .. } => segments.first().copied(),
            Expr::Literal(tok, _) => Some(tok.span),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_registry_has_std_derives() {
        let registry = DeriveRegistry::with_std_derives();

        assert!(registry.is_registered("Debug"));
        assert!(registry.is_registered("Clone"));
        assert!(registry.is_registered("PartialEq"));
        assert!(registry.is_registered("Eq"));
        assert!(registry.is_registered("Default"));
    }

    #[test]
    fn test_compiler_attrs_are_separate() {
        // Compiler attributes should not be in the derive registry
        let registry = DeriveRegistry::with_std_derives();

        assert!(!registry.is_registered("repr"));
        assert!(!registry.is_registered("test"));
        assert!(!registry.is_registered("inline"));
        assert!(!registry.is_registered("no_mangle"));
        assert!(!registry.is_registered("link"));
    }

    #[test]
    fn test_can_register_custom_derive() {
        let mut registry = DeriveRegistry::new();

        assert!(!registry.is_registered("CustomTrait"));

        registry.register("CustomTrait", Arc::new(|_ctx, _input| {
            Ok(Vec::new())
        }));

        assert!(registry.is_registered("CustomTrait"));
    }
}

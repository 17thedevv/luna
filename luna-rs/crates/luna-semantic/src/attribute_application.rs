//! TypeChecker-owned application validation, after binding admission. The
//! declaration traversal is the ordinary typechecker traversal, so macro and
//! portable provider declarations cannot bypass these checks.
use super::*;

impl<'a> TypeChecker<'a> {
    pub(super) fn validate_attribute_application(&mut self, decl: &Decl) {
        for annotation in decl.annotations() {
            let name = self.get_span_text(annotation.name);
            let applicable = match name {
                "derive" | "repr" => matches!(decl, Decl::Struct { .. } | Decl::Enum { .. }),
                "test" | "inline" | "no_mangle" => matches!(decl, Decl::Function { .. }),
                "link" => matches!(decl, Decl::Function { .. } | Decl::Extern { .. }),
                "sync_noescape" => matches!(decl, Decl::Param { .. }),
                "lang" => true, // Resolver owns target/trust/binding admission.
                _ => continue, // Parser owns attribute-name grammar.
            };
            if !applicable {
                let target = match name {
                    "derive" | "repr" => "structs or enums",
                    "link" => "functions or extern blocks",
                    "sync_noescape" => "parameters",
                    _ => "functions",
                };
                self.ctx.diagnostics.push(Diagnostic::source_error(
                    DiagnosticCode::InvalidAttributeApplication, annotation.name,
                    format!("`#[{name}]` can only be applied to {target}")));
            }
            if name == "repr" && applicable {
                for argument in &annotation.args {
                    let (text, span) = match &self.arena.exprs[argument.value.0 as usize] {
                        Expr::Identifier { segments, .. } if !segments.is_empty() =>
                            (self.get_span_text(segments[0]), segments[0]),
                        Expr::Literal(token, _) => (self.get_span_text(token.span), token.span),
                        _ => ("", annotation.name),
                    };
                    if !matches!(text, "C" | "transparent" | "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64") {
                        self.ctx.diagnostics.push(Diagnostic::source_error(
                            DiagnosticCode::InvalidAttributeApplication, span,
                            format!("invalid repr argument `{text}`")));
                    }
                }
            }
        }
        if let Decl::Function { params, .. } = decl {
            for parameter in params {
                self.validate_attribute_application(&self.arena.decls[parameter.0 as usize]);
            }
        }
        if let Decl::Enum { variants, .. } = decl {
            for variant in variants {
                for annotation in &variant.annotations {
                    let name = self.get_span_text(annotation.name);
                    if name != "lang" {
                        self.ctx.diagnostics.push(Diagnostic::source_error(
                            DiagnosticCode::InvalidAttributeApplication, annotation.name,
                            format!("`#[{name}]` cannot be applied to enum variants")));
                    }
                }
                for field in &variant.fields {
                    self.validate_attribute_application(&self.arena.decls[field.0 as usize]);
                }
            }
        }
    }
}

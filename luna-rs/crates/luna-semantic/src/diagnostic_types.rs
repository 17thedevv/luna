//! Human-readable semantic types for diagnostics, independent of session IDs.
use crate::ty::{BuiltinType, Mutability, SemanticType, SemanticTypeId};
use crate::{SemanticContext, SymbolId};

impl SemanticContext {
    /// Describe a type without exposing interner or inferred lifetime IDs.
    /// This is a display only: lifetime obligations remain separate diagnostics.
    pub(crate) fn diagnostic_type(&self, ty: SemanticTypeId) -> String {
        TypeDisplay {
            ctx: self,
            remaining: 128,
        }
        .render(ty, 0)
    }
}

struct TypeDisplay<'a> {
    ctx: &'a SemanticContext,
    remaining: usize,
}

impl TypeDisplay<'_> {
    fn name(&self, symbol: SymbolId) -> String {
        let path = self.ctx.symbol_table.get_full_logical_path(symbol);
        if path.is_empty() {
            "<type>".into()
        } else {
            path.join("::")
        }
    }

    fn list(&mut self, types: &[SemanticTypeId], depth: usize) -> String {
        let mut parts = Vec::new();
        for ty in types {
            if self.remaining == 0 {
                parts.push("...".into());
                break;
            }
            parts.push(self.render(*ty, depth));
        }
        parts.join(", ")
    }

    fn render(&mut self, ty: SemanticTypeId, depth: usize) -> String {
        if depth >= 32 || self.remaining == 0 {
            return "...".into();
        }
        self.remaining -= 1;
        let ty = self.ctx.types.resolve(ty);
        let writable = |mutability: &Mutability| {
            if *mutability == Mutability::Mutable {
                "rw "
            } else {
                ""
            }
        };
        match self.ctx.types.get(ty) {
            SemanticType::Primitive(builtin) => match builtin {
                BuiltinType::I8 => "i8",
                BuiltinType::I16 => "i16",
                BuiltinType::I32 => "i32",
                BuiltinType::I64 => "i64",
                BuiltinType::I128 => "i128",
                BuiltinType::Isize => "isize",
                BuiltinType::U8 => "u8",
                BuiltinType::U16 => "u16",
                BuiltinType::U32 => "u32",
                BuiltinType::U64 => "u64",
                BuiltinType::U128 => "u128",
                BuiltinType::Usize => "usize",
                BuiltinType::F32 => "f32",
                BuiltinType::F64 => "f64",
                BuiltinType::Bool => "bool",
                BuiltinType::String => "str",
                BuiltinType::Char => "char",
            }
            .into(),
            SemanticType::Reference(_, mutability, inner) => format!(
                "&{}{}",
                writable(mutability),
                self.render(*inner, depth + 1)
            ),
            SemanticType::Pointer(mutability, inner) => format!(
                "*{}{}",
                writable(mutability),
                self.render(*inner, depth + 1)
            ),
            SemanticType::Slice(inner) => format!("[{}]", self.render(*inner, depth + 1)),
            SemanticType::Array(inner, length) => {
                format!("[{}; {}]", self.render(*inner, depth + 1), length)
            }
            SemanticType::Tuple(elements) => {
                let values = self.list(elements, depth + 1);
                if elements.len() == 1 {
                    format!("({values},)")
                } else {
                    format!("({values})")
                }
            }
            SemanticType::Struct(symbol, args, _) | SemanticType::Enum(symbol, args, _) => {
                let name = self.name(*symbol);
                if args.is_empty() {
                    name
                } else {
                    format!("{name}<{}>", self.list(args, depth + 1))
                }
            }
            SemanticType::GenericParam(symbol) => self.name(*symbol),
            SemanticType::DynTrait(symbol) => format!("dyn {}", self.name(*symbol)),
            SemanticType::Function {
                params,
                return_type,
                is_unsafe,
            } => {
                let prefix = if *is_unsafe { "unsafe " } else { "" };
                format!(
                    "{prefix}fn({}) -> {}",
                    self.list(params, depth + 1),
                    self.render(*return_type, depth + 1)
                )
            }
            SemanticType::Closure(_, params, result) => format!(
                "<closure fn({}) -> {}>",
                self.list(params, depth + 1),
                self.render(*result, depth + 1)
            ),
            SemanticType::Future(output) => {
                format!("<future output {}>", self.render(*output, depth + 1))
            }
            SemanticType::Range(element) => {
                format!("<range of {}>", self.render(*element, depth + 1))
            }
            SemanticType::Projection {
                self_type,
                trait_id,
                assoc_type,
            } => format!(
                "<associated type {}::{} for {}>",
                self.name(*trait_id),
                self.ctx.symbol_table.get_symbol(*assoc_type).name,
                self.render(*self_type, depth + 1)
            ),
            SemanticType::Void => "void".into(),
            SemanticType::Never => "!".into(),
            SemanticType::Error => "<error>".into(),
            SemanticType::InferenceVar(_) => "<inferred type>".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ty::LifetimeId;

    #[test]
    fn displays_luna_type_spelling_without_session_ids() {
        let mut ctx = SemanticContext::new();
        let byte = ctx.types.intern(SemanticType::Primitive(BuiltinType::U8));
        let slice = ctx.types.intern(SemanticType::Slice(byte));
        let reference = ctx.types.intern(SemanticType::Reference(
            LifetimeId(123),
            Mutability::Immutable,
            slice,
        ));
        assert_eq!(ctx.diagnostic_type(reference), "&[u8]");
        let text = ctx
            .types
            .intern(SemanticType::Primitive(BuiltinType::String));
        assert_eq!(ctx.diagnostic_type(text), "str");
        let pointer = ctx
            .types
            .intern(SemanticType::Pointer(Mutability::Mutable, byte));
        let array = ctx.types.intern(SemanticType::Array(pointer, 3));
        let tuple = ctx.types.intern(SemanticType::Tuple(vec![array]));
        assert_eq!(ctx.diagnostic_type(tuple), "([*rw u8; 3],)");
        let function = ctx.types.intern(SemanticType::Function {
            params: vec![reference],
            return_type: text,
            is_unsafe: true,
        });
        assert_eq!(ctx.diagnostic_type(function), "unsafe fn(&[u8]) -> str");
    }

    #[test]
    fn resolves_inference_and_bounds_recursive_type_output() {
        let mut ctx = SemanticContext::new();
        let variable = ctx.types.new_inference_var();
        let SemanticType::InferenceVar(index) = *ctx.types.get(variable) else {
            unreachable!()
        };
        let recursive = ctx.types.intern(SemanticType::Slice(variable));
        ctx.types.inference_bindings.insert(index, recursive);
        let display = ctx.diagnostic_type(variable);
        assert!(display.contains("..."));
        assert!(display.len() < 100);
        let byte = ctx.types.intern(SemanticType::Primitive(BuiltinType::U8));
        ctx.types.inference_bindings.insert(index, byte);
        assert_eq!(ctx.diagnostic_type(variable), "u8");
        let wide = ctx.types.intern(SemanticType::Tuple(vec![byte; 10_000]));
        assert!(ctx.diagnostic_type(wide).len() < 600);
    }

    #[test]
    fn nominal_types_keep_namespace_and_generic_arguments() {
        use crate::symbol::{ScopeId, ScopeKind, SymbolKind};
        use luna_ast::Visibility;
        use luna_common::ids::{FileId, Span};

        let mut ctx = SemanticContext::new();
        let mut diagnostics = Vec::new();
        let module = ctx.symbol_table.declare_symbol(
            "app".into(),
            SymbolKind::Module,
            ScopeId(0),
            Span::new(FileId(0), 0, 1),
            None,
            Visibility::Public,
            &mut diagnostics,
        );
        let scope = ctx
            .symbol_table
            .create_scope(ScopeKind::Module, Some(ScopeId(0)));
        ctx.symbol_table.set_inner_scope(module, scope);
        let symbol = ctx.symbol_table.declare_symbol(
            "Packet".into(),
            SymbolKind::Struct,
            scope,
            Span::new(FileId(0), 2, 3),
            None,
            Visibility::Public,
            &mut diagnostics,
        );
        let byte = ctx.types.intern(SemanticType::Primitive(BuiltinType::U8));
        let packet = ctx
            .types
            .intern(SemanticType::Struct(symbol, vec![byte], vec![]));
        assert_eq!(ctx.diagnostic_type(packet), "app::Packet<u8>");
        // Metadata imports carry the same logical path despite consumer-local IDs.
        ctx.symbol_table
            .imported_paths
            .insert(symbol, vec!["app".into(), "Packet".into()]);
        assert_eq!(ctx.diagnostic_type(packet), "app::Packet<u8>");
        assert!(diagnostics.is_empty());
    }
}

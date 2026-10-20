pub mod value;
pub mod reflect;
pub mod type_repr;

/// Comptime capability admission currently rejects standalone statement form.
/// Expression form has its separate MVIR evaluation path.
pub fn reject_statement(span: luna_common::Span) -> luna_common::Diagnostic {
    luna_common::Diagnostic::source_error(luna_common::DiagnosticCode::ComptimeEvaluationFailed, span,
        "standalone comptime statements are not supported; use a comptime expression")
        .in_phase(luna_common::diagnostic::DiagnosticPhase::AsyncComptime)
}

pub use value::{ComptimeValue, IntWidth, FloatWidth, ComptimeError};
pub use type_repr::{TypeRepr, TypeInfoStruct, TypeKind, TypeInfoField, TypeInfoVariant};

//! Primitive operator restrictions shared by type checking and instantiation.
use crate::ty::{BuiltinType, SemanticType, SemanticTypeId, TypeContext};
use luna_ast::expr::{AssignOp, BinaryOp, UnaryOp};

#[derive(Debug, Clone, Copy)]
pub(crate) enum PrimitiveOperator {
    Binary(BinaryOp),
    Unary(UnaryOp),
    Assignment(AssignOp),
}

impl PrimitiveOperator {
    fn forbids_char(self) -> bool {
        match self {
            Self::Binary(op) => matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::BitAnd
                    | BinaryOp::BitOr
                    | BinaryOp::BitXor
                    | BinaryOp::LShift
                    | BinaryOp::RShift
            ),
            Self::Unary(op) => matches!(
                op,
                UnaryOp::Neg | UnaryOp::BitNot | UnaryOp::PostInc | UnaryOp::PostDec
            ),
            Self::Assignment(op) => op != AssignOp::Assign,
        }
    }

    pub(crate) fn error_for_type(self, types: &TypeContext, ty: SemanticTypeId) -> Option<String> {
        if self.forbids_char() && types.get(ty) == &SemanticType::Primitive(BuiltinType::Char) {
            Some(format!(
                "E_INVALID_CHAR_OPERATOR: {self:?} is not defined for Unicode scalar `char`; cast to an integer explicitly before arithmetic, bitwise, or shift operations"
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_restrictions_do_not_change_comparison_assignment_or_integer_operators() {
        let mut types = TypeContext::new();
        let character = types.intern(SemanticType::Primitive(BuiltinType::Char));
        let integer = types.intern(SemanticType::Primitive(BuiltinType::I32));
        for op in [
            BinaryOp::Add,
            BinaryOp::Sub,
            BinaryOp::Mul,
            BinaryOp::Div,
            BinaryOp::Mod,
            BinaryOp::BitAnd,
            BinaryOp::BitOr,
            BinaryOp::BitXor,
            BinaryOp::LShift,
            BinaryOp::RShift,
        ] {
            assert!(PrimitiveOperator::Binary(op)
                .error_for_type(&types, character)
                .is_some());
            assert!(PrimitiveOperator::Binary(op)
                .error_for_type(&types, integer)
                .is_none());
        }
        for op in [
            BinaryOp::Eq,
            BinaryOp::Ne,
            BinaryOp::Lt,
            BinaryOp::Le,
            BinaryOp::Gt,
            BinaryOp::Ge,
        ] {
            assert!(PrimitiveOperator::Binary(op)
                .error_for_type(&types, character)
                .is_none());
        }
        assert!(PrimitiveOperator::Assignment(AssignOp::Assign)
            .error_for_type(&types, character)
            .is_none());
        assert!(PrimitiveOperator::Unary(UnaryOp::Ref)
            .error_for_type(&types, character)
            .is_none());
    }
}

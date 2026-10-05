//! Language-level cast admission, independent of LLVM representation equality.
use crate::ty::{BuiltinType, Mutability, SemanticType, SemanticTypeId, TypeContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastAdmission {
    Safe,
    RequiresUnsafe,
}

impl TypeContext {
    pub fn cast_admission(
        &self,
        source: SemanticTypeId,
        target: SemanticTypeId,
    ) -> Result<CastAdmission, &'static str> {
        let source = self.resolve(source);
        let target = self.resolve(target);
        let from = self.get(source);
        let to = self.get(target);
        if matches!(from, SemanticType::Error) || matches!(to, SemanticType::Error) {
            return Err("cast operand or target has an invalid type");
        }
        if self.is_unsized(target) {
            return Err("cannot cast to an unsized value type");
        }
        if source == target {
            return Ok(CastAdmission::Safe);
        }
        if matches!(to, SemanticType::Primitive(BuiltinType::Char))
            && !matches!(from, SemanticType::Primitive(b) if b.is_integer() || *b == BuiltinType::Char)
        {
            return Err("invalid cast to char: source must be an integer or char");
        }

        let pointee = |ty: &SemanticType| match ty {
            SemanticType::Pointer(_, inner) | SemanticType::Reference(_, _, inner) => Some(*inner),
            _ => None,
        };
        let from_inner = pointee(from);
        let to_inner = pointee(to);
        let from_fat = from_inner.is_some_and(|inner| self.is_unsized(inner));
        let to_fat = to_inner.is_some_and(|inner| self.is_unsized(inner));
        if from_fat != to_fat {
            return Err("cast cannot strip or fabricate fat pointer metadata");
        }
        if from_fat && from_inner.map(|t| self.resolve(t)) != to_inner.map(|t| self.resolve(t)) {
            return Err("cast cannot reinterpret fat pointer metadata for a different referent");
        }

        use SemanticType::*;
        match (from, to) {
            (Reference(_, from_mut, from_inner), Reference(_, to_mut, to_inner)) => {
                if self.resolve(*from_inner) != self.resolve(*to_inner) {
                    return Err("reference cast must preserve the referent type");
                }
                if *from_mut == Mutability::Immutable && *to_mut == Mutability::Mutable {
                    return Err("reference cast cannot upgrade shared access to mutable access");
                }
                Ok(CastAdmission::Safe)
            }
            (Pointer(_, _), Reference(_, _, _)) => Ok(CastAdmission::RequiresUnsafe),
            (Pointer(_, _), Pointer(_, _)) | (Reference(_, _, _), Pointer(_, _)) => Ok(CastAdmission::Safe),
            // The builtin str representation exposes its byte address; this
            // is separate from a cast between unrelated nominal values.
            (Primitive(BuiltinType::String), Pointer(_, inner))
                if matches!(self.get(self.resolve(*inner)), Primitive(BuiltinType::U8)) => Ok(CastAdmission::Safe),
            (Function { params: a, return_type: ar, is_unsafe: au },
             Function { params: b, return_type: br, is_unsafe: bu }) => {
                if au != bu {
                    return Err("callable cast must preserve its unsafe requirement");
                }
                if a == b && ar == br {
                    Ok(CastAdmission::Safe)
                } else {
                    // Explicit ABI adaptation retains the original callable's
                    // safety requirement. Valid invocation is the unsafe caller's
                    // responsibility, as for an FFI function pointer.
                    Ok(CastAdmission::RequiresUnsafe)
                }
            }
            (Primitive(a), Primitive(b)) if *a != BuiltinType::String && *b != BuiltinType::String => {
                Ok(CastAdmission::Safe)
            }
            (Pointer(_, _) | Reference(_, _, _), Primitive(b)) if b.is_integer() => Ok(CastAdmission::Safe),
            (Primitive(a), Pointer(_, _)) if a.is_integer() => Ok(CastAdmission::Safe),
            _ => Err("cast is not defined between these language types"),
        }
    }
}

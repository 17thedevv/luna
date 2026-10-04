//! Immutable program storage. The payload is independent of semantic-session
//! IDs and VM addresses, so it can travel through portable artifacts.
use luna_semantic::{BuiltinType, ComptimeValue, SemanticContext, SemanticType, SemanticTypeId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaticType {
    Unit,
    Primitive(BuiltinType),
    Aggregate(Vec<StaticType>),
    Array { element: Box<StaticType>, len: u64 },
    Enum(Vec<StaticType>),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaticValue {
    Unit,
    Bool(bool),
    Int(i128),
    Float(u64), // f64 bits; preserves NaNs without host-address serialization
    Char(u32),
    Str(String),
    Aggregate(Vec<StaticValue>),
    Enum {
        tag: u32,
        field_count: u32,
        payload: Box<StaticValue>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticData {
    pub name: String,
    pub ty: StaticType,
    pub value: StaticValue,
}

impl StaticType {
    pub fn from_semantic(ctx: &SemanticContext, ty: SemanticTypeId) -> Result<Self, String> {
        Ok(match ctx.types.get(ctx.types.resolve(ty)) {
            SemanticType::Void => Self::Unit,
            SemanticType::Primitive(b) => Self::Primitive(*b),
            SemanticType::Tuple(fields) | SemanticType::Struct(_, _, fields) => Self::Aggregate(
                fields
                    .iter()
                    .map(|ty| Self::from_semantic(ctx, *ty))
                    .collect::<Result<_, _>>()?,
            ),
            SemanticType::Array(element, len) => Self::Array {
                element: Box::new(Self::from_semantic(ctx, *element)?),
                len: *len,
            },
            SemanticType::Enum(_, _, variants) => Self::Enum(
                variants
                    .iter()
                    .map(|ty| Self::from_semantic(ctx, *ty))
                    .collect::<Result<_, _>>()?,
            ),
            other => {
                return Err(format!(
                    "constant storage has no portable representation for {other:?}"
                ));
            }
        })
    }
}

impl StaticValue {
    pub fn from_comptime(
        ctx: &SemanticContext,
        ty: SemanticTypeId,
        value: &ComptimeValue,
    ) -> Result<Self, String> {
        let sem_ty = ctx.types.get(ctx.types.resolve(ty));
        Ok(match (sem_ty, value) {
            (SemanticType::Void, ComptimeValue::Unit) => Self::Unit,
            (SemanticType::Primitive(BuiltinType::Bool), ComptimeValue::Bool(v)) => Self::Bool(*v),
            (SemanticType::Primitive(b), ComptimeValue::Int { val, .. }) if b.is_integer() => {
                Self::Int(*val)
            }
            (
                SemanticType::Primitive(BuiltinType::F32 | BuiltinType::F64),
                ComptimeValue::Float { val, .. },
            ) => Self::Float(val.to_bits()),
            (SemanticType::Primitive(BuiltinType::Char), ComptimeValue::Char(v)) => {
                Self::Char(*v as u32)
            }
            (SemanticType::Primitive(BuiltinType::Char), ComptimeValue::Int { val, .. }) => {
                let value: u32 = (*val)
                    .try_into()
                    .map_err(|_| "constant char out of range")?;
                char::from_u32(value).ok_or("constant char is not a Unicode scalar")?;
                Self::Char(value)
            }
            (SemanticType::Primitive(BuiltinType::String), ComptimeValue::Str(v)) => {
                Self::Str(v.clone())
            }
            // The VM represents aggregate memory as declaration-order tuples;
            // recover its shape from the checked declaration type, never from
            // a nominal/session ID embedded in the evaluated value.
            (
                SemanticType::Tuple(types) | SemanticType::Struct(_, _, types),
                ComptimeValue::Tuple(values),
            ) if types.len() == values.len() => Self::Aggregate(
                types
                    .iter()
                    .zip(values)
                    .map(|(ty, v)| Self::from_comptime(ctx, *ty, v))
                    .collect::<Result<_, _>>()?,
            ),
            (SemanticType::Array(elem, len), ComptimeValue::Array { elements, .. })
                if *len == elements.len() as u64 =>
            {
                Self::Aggregate(
                    elements
                        .iter()
                        .map(|v| Self::from_comptime(ctx, *elem, v))
                        .collect::<Result<_, _>>()?,
                )
            }
            (SemanticType::Array(elem, len), ComptimeValue::Tuple(elements))
                if *len == elements.len() as u64 =>
            {
                Self::Aggregate(
                    elements
                        .iter()
                        .map(|v| Self::from_comptime(ctx, *elem, v))
                        .collect::<Result<_, _>>()?,
                )
            }
            (SemanticType::Struct(sym, _, types), ComptimeValue::Struct { fields, .. }) => {
                let names = ctx
                    .tables
                    .struct_field_names
                    .get(sym)
                    .ok_or("constant struct lacks canonical field names")?;
                if names.len() != types.len() || fields.len() != types.len() {
                    return Err("constant struct field count differs from its type".into());
                }
                Self::Aggregate(
                    names
                        .iter()
                        .zip(types)
                        .map(|(name, ty)| {
                            let v = fields
                                .iter()
                                .find(|(n, _)| n == name)
                                .ok_or_else(|| format!("constant struct lacks field {name}"))?;
                            Self::from_comptime(ctx, *ty, &v.1)
                        })
                        .collect::<Result<_, _>>()?,
                )
            }
            (
                SemanticType::Enum(_, _, variants),
                ComptimeValue::Enum {
                    variant_index,
                    payload,
                    ..
                },
            ) => {
                let payload_ty = *variants
                    .get(*variant_index as usize)
                    .ok_or("invalid constant enum tag")?;
                let payload_value = match payload.as_slice() {
                    [] => ComptimeValue::Unit,
                    [v] => v.clone(),
                    values => ComptimeValue::Tuple(values.to_vec()),
                };
                Self::Enum {
                    tag: *variant_index,
                    field_count: payload.len() as u32,
                    payload: Box::new(Self::from_comptime(ctx, payload_ty, &payload_value)?),
                }
            }
            _ => {
                return Err(format!(
                    "constant initializer does not match {sem_ty:?}: {value:?}"
                ));
            }
        })
    }
}

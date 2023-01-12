use crate::core::Function;
use crate::lang::parser::{Located, SourceLocation};
use crate::runtime::TypeName;
use serde::{Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::sync::Arc;

pub mod builder;
pub mod hir;
pub mod lir;
pub mod mir;
pub mod parser;

#[derive(Debug, Clone, Serialize)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(TypeName, #[serde(skip)] Arc<dyn Function>),
}

impl Hash for PrimordialType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PrimordialType::Integer => "integer".hash(state),
            PrimordialType::Decimal => "decimal".hash(state),
            PrimordialType::Boolean => "boolean".hash(state),
            PrimordialType::String => "string".hash(state),
            PrimordialType::Function(name, _) => name.hash(state),
        }
    }
}

impl PartialEq for PrimordialType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Integer, Self::Integer) => true,
            (Self::Decimal, Self::Decimal) => true,
            (Self::Boolean, Self::Boolean) => true,
            (Self::String, Self::String) => true,
            (Self::Function(lhs, _), Self::Function(rhs, _)) => lhs.eq(rhs),
            _ => false,
        }
    }
}

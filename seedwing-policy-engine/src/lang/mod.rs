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
pub enum SyntacticSugar {
    None,
    And,
    Or,
    Refine,
    Traverse,
    Chain,
    Not,
}

impl From<TypeName> for SyntacticSugar {
    fn from(name: TypeName) -> Self {
        match name.as_type_str().as_str() {
            "lang::And" => Self::And,
            "lang::Or" => Self::Or,
            "lang::Refine" => Self::Refine,
            "lang::Traverse" => Self::Traverse,
            "lang::Chain" => Self::Chain,
            "lang::Not" => Self::Not,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(SyntacticSugar, TypeName, #[serde(skip)] Arc<dyn Function>),
}

impl PrimordialType {
    fn order(&self) -> u8 {
        match self {
            Self::Function(_, _, f) => f.order(),
            Self::String => 2,
            Self::Decimal => 1,
            _ => 0,
        }
    }
}

impl Hash for PrimordialType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PrimordialType::Integer => "integer".hash(state),
            PrimordialType::Decimal => "decimal".hash(state),
            PrimordialType::Boolean => "boolean".hash(state),
            PrimordialType::String => "string".hash(state),
            PrimordialType::Function(_, name, _) => name.hash(state),
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
            (Self::Function(_, lhs, _), Self::Function(_, rhs, _)) => lhs.eq(rhs),
            _ => false,
        }
    }
}

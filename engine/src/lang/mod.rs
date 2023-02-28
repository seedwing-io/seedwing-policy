//! Dogma language implementation.
use crate::core::Function;

use crate::runtime::PatternName;
use serde::{Deserialize, Serialize};

use std::hash::{Hash, Hasher};

use std::sync::Arc;

pub mod builder;
pub(crate) mod hir;
pub(crate) mod lir;
pub(crate) mod mir;
pub(crate) mod parser;

pub use lir::{Expr, ValuePattern};

/// Native functions that have syntactic sugar.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SyntacticSugar {
    None,
    And,
    Or,
    Refine,
    Traverse,
    Chain,
    Not,
}

impl From<PatternName> for SyntacticSugar {
    fn from(name: PatternName) -> Self {
        match name.as_type_str().as_str() {
            "lang::and" => Self::And,
            "lang::or" => Self::Or,
            "lang::refine" => Self::Refine,
            "lang::traverse" => Self::Traverse,
            "lang::chain" => Self::Chain,
            "lang::not" => Self::Not,
            _ => Self::None,
        }
    }
}

/// Primordial patterns are the basic building blocks.
#[derive(Debug, Clone, Serialize)]
pub enum PrimordialPattern {
    /// Match integers.
    Integer,

    /// Match decimals.
    Decimal,

    /// Match booleans.
    Boolean,

    /// Match strings.
    String,

    /// Match builtin sugared functions.
    Function(
        SyntacticSugar,
        PatternName,
        #[serde(skip)] Arc<dyn Function>,
    ),
}

impl PrimordialPattern {
    fn order(&self) -> u8 {
        match self {
            Self::Function(_, _, f) => f.order(),
            Self::String => 2,
            Self::Decimal => 1,
            _ => 0,
        }
    }
}

impl Hash for PrimordialPattern {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            PrimordialPattern::Integer => "integer".hash(state),
            PrimordialPattern::Decimal => "decimal".hash(state),
            PrimordialPattern::Boolean => "boolean".hash(state),
            PrimordialPattern::String => "string".hash(state),
            PrimordialPattern::Function(_, name, _) => name.hash(state),
        }
    }
}

impl PartialEq for PrimordialPattern {
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

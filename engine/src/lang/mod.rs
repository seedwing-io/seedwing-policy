//! Dogma language implementation.
//!
//! Types used for compiling policies and evaluating them.
use crate::core::Function;

use crate::runtime::{EvaluationResult, PatternName};
use serde::{Deserialize, Serialize};

use std::fmt;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub mod builder;
pub(crate) mod hir;
pub(crate) mod lir;
pub(crate) mod mir;
pub(crate) mod parser;

pub use lir::{Expr, ValuePattern};

mod meta;
pub use meta::*;

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

impl fmt::Display for SyntacticSugar {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SyntacticSugar::None => write!(f, "None"),
            SyntacticSugar::And => write!(f, "And"),
            SyntacticSugar::Or => write!(f, "Or"),
            SyntacticSugar::Refine => write!(f, "Refine"),
            SyntacticSugar::Traverse => write!(f, "Traverse"),
            SyntacticSugar::Chain => write!(f, "Chain"),
            SyntacticSugar::Not => write!(f, "Not"),
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

/// Severity of the outcome
///
/// | Value     | Satisfied |
/// | --------- | :-------: |
/// | `None`    | ✅        |
/// | `Advice`  | ✅        |
/// | `Warning` | ✅        |
/// | `Error`   | ❌        |
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Good
    #[default]
    None,
    /// All good, but there is something you might want to know
    Advice,
    /// Good, but smells fishy
    Warning,
    /// Boom! Bad!
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Advice => f.write_str("advice"),
            Self::Warning => f.write_str("warning"),
            Self::Error => f.write_str("error"),
        }
    }
}

/// Allow building a severity from a list of severities, the highest will win.
impl FromIterator<Severity> for Severity {
    fn from_iter<T: IntoIterator<Item = Severity>>(iter: T) -> Self {
        let mut highest = Severity::None;

        for s in iter {
            if s == Severity::Error {
                return Severity::Error;
            }
            if s > highest {
                highest = s;
            }
        }

        highest
    }
}

impl<'a> FromIterator<&'a EvaluationResult> for Severity {
    fn from_iter<T: IntoIterator<Item = &'a EvaluationResult>>(iter: T) -> Self {
        iter.into_iter().map(EvaluationResult::severity).collect()
    }
}

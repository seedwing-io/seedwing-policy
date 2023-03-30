use crate::{
    lang::{lir::Bindings, Severity},
    runtime::EvaluationResult,
};
use std::collections::HashMap;

/// Rationale for a policy decision.
#[derive(Debug, Clone)]
pub enum Rationale {
    Anything,
    Nothing,
    Chain(Vec<EvaluationResult>),
    Object(HashMap<String, Option<EvaluationResult>>),
    List(Vec<EvaluationResult>),
    NotAnObject,
    NotAList,
    MissingField(String),
    InvalidArgument(String),
    Const(bool),
    Primordial(bool),
    Expression(bool),
    Function {
        severity: Severity,
        rationale: Option<Box<Rationale>>,
        supporting: Vec<EvaluationResult>,
    },
    Refinement(Box<EvaluationResult>, Option<Box<EvaluationResult>>),
    Bound(Box<Rationale>, Bindings),
}

impl Rationale {
    pub fn severity(&self) -> Severity {
        match self {
            Rationale::Anything => Severity::None,
            Rationale::Nothing => Severity::Error,
            Rationale::Object(fields) => fields
                .values()
                .map(|r| r.as_ref().map(|e| e.severity()).unwrap_or(Severity::Error))
                .collect(),
            Rationale::List(items) => items.iter().collect(),
            Rationale::NotAnObject => Severity::Error,
            Rationale::NotAList => Severity::Error,
            Rationale::MissingField(_) => Severity::Error,
            Rationale::InvalidArgument(_) => Severity::Error,
            Rationale::Const(val) | Rationale::Primordial(val) | Rationale::Expression(val) => {
                match *val {
                    true => Severity::None,
                    false => Severity::Error,
                }
            }
            Rationale::Function {
                severity,
                rationale: _,
                supporting: _,
            } => *severity,
            Rationale::Chain(terms) => terms.iter().collect(),
            // TODO: check if this is still used
            Rationale::Refinement(primary, refinement) => {
                if matches!(primary.severity(), Severity::Error) {
                    Severity::Error
                } else if let Some(refinement) = refinement {
                    refinement.severity()
                } else {
                    // TODO: check if this is really error, maybe it should be "ok"?
                    Severity::Error
                }
            }
            Rationale::Bound(inner, _) => inner.severity(),
        }
    }

    pub fn reason(&self) -> String {
        match self {
            Rationale::Anything => "anything is satisfied by anything".into(),
            Rationale::Nothing => "Nothing".into(),
            Rationale::Const(_) => {
                if self.severity() < Severity::Error {
                    "The input matches the expected constant value expected in the pattern"
                } else {
                    "The input does not match the constant value expected in the pattern"
                }
            }
            .into(),
            Rationale::Primordial(_) => {
                if self.severity() < Severity::Error {
                    "The primordial type defined in the pattern is satisfied"
                } else {
                    "The primordial type defined in the pattern is not satisfied"
                }
            }
            .into(),
            Rationale::Expression(_) => if self.severity() < Severity::Error {
                "The expression defined in the pattern is satisfied"
            } else {
                "The expression defined in the pattern is not satisfied"
            }
            .into(),
            Rationale::Object(_) => if self.severity() < Severity::Error {
                "Because all fields were satisfied"
            } else {
                "Because not all fields were satisfied"
            }
            .into(),
            Rationale::List(_terms) => if self.severity() < Severity::Error {
                "because all members were satisfied"
            } else {
                "because not all members were satisfied"
            }
            .into(),
            Rationale::Chain(_terms) => if self.severity() < Severity::Error {
                "because the chain was satisfied"
            } else {
                "because the chain was not satisfied"
            }
            .into(),
            Rationale::NotAnObject => "not an object".into(),
            Rationale::NotAList => "not a list".into(),
            Rationale::MissingField(name) => format!("missing field: {name}"),
            Rationale::InvalidArgument(name) => format!("invalid argument: {name}"),
            Rationale::Function {
                severity: _,
                rationale,
                supporting: _,
            } => match rationale {
                Some(x) => x.reason(),
                None => if self.severity() < Severity::Error {
                    "The input satisfies the function"
                } else {
                    "The input does not satisfy the function"
                }
                .to_string(),
            },
            Rationale::Refinement(_, _) => String::new(),
            Rationale::Bound(inner, _) => inner.reason(),
        }
    }
}

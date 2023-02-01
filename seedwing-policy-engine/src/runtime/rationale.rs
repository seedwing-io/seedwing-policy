use crate::runtime::EvaluationResult;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Rationale {
    Anything,
    Nothing,
    Chain(Vec<EvaluationResult>),
    Object(HashMap<String, EvaluationResult>),
    List(Vec<EvaluationResult>),
    NotAnObject,
    NotAList,
    MissingField(String),
    InvalidArgument(String),
    Const(bool),
    Primordial(bool),
    Expression(bool),
    Function(bool, Option<Box<Rationale>>, Vec<EvaluationResult>),
    Refinement(Box<EvaluationResult>, Option<Box<EvaluationResult>>),
}

impl Rationale {
    pub fn satisfied(&self) -> bool {
        match self {
            Rationale::Anything => true,
            Rationale::Nothing => false,
            Rationale::Object(fields) => fields.values().all(|e| e.satisfied()),
            Rationale::List(items) => items.iter().all(|e| e.satisfied()),
            Rationale::NotAnObject => false,
            Rationale::NotAList => false,
            Rationale::MissingField(_) => false,
            Rationale::InvalidArgument(_) => false,
            Rationale::Const(val) => *val,
            Rationale::Primordial(val) => *val,
            Rationale::Expression(val) => *val,
            Rationale::Function(val, rational, _) => *val,
            Rationale::Chain(terms) => terms.iter().all(|e| e.satisfied()),
            Rationale::Refinement(primary, refinement) => {
                if !primary.satisfied() {
                    false
                } else if let Some(refinement) = refinement {
                    refinement.satisfied()
                } else {
                    false
                }
            }
        }
    }
}

use crate::runtime::EvaluationResult;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Rationale {
    Anything,
    Nothing,
    Join(Vec<EvaluationResult>),
    Meet(Vec<EvaluationResult>),
    Object(HashMap<String, EvaluationResult>),
    NotAnObject,
    MissingField(String),
    InvalidArgument(String),
    Const(bool),
    Primordial(bool),
    Expression(bool),
    Function(bool, Vec<EvaluationResult>),
    Refinement(Box<EvaluationResult>, Option<Box<EvaluationResult>>),
}

impl Rationale {
    pub fn satisfied(&self) -> bool {
        match self {
            Rationale::Anything => true,
            Rationale::Nothing => false,
            Rationale::Join(terms) => terms.iter().any(|e| e.satisfied()),
            Rationale::Meet(terms) => terms.iter().all(|e| e.satisfied()),
            Rationale::Object(fields) => fields.values().all(|e| e.satisfied()),
            Rationale::NotAnObject => false,
            Rationale::MissingField(_) => false,
            Rationale::InvalidArgument(_) => false,
            Rationale::Const(val) => *val,
            Rationale::Primordial(val) => *val,
            Rationale::Expression(val) => *val,
            Rationale::Function(val, _) => *val,
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

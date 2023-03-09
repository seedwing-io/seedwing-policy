//! Response handling a policy decision.

use crate::runtime::Output;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{rationale::Rationale, EvaluationResult, PatternName};

/// A response is used to transform a policy result into different formats.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<PatternName>,
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    pub satisfied: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rationale: Vec<Response>,
}

impl From<EvaluationResult> for Response {
    fn from(result: EvaluationResult) -> Self {
        Self::new(&result)
    }
}

impl Response {
    pub fn new(result: &EvaluationResult) -> Self {
        let output = match &result.output {
            Output::Identity if result.satisfied() => Some(result.input.as_json()),
            Output::Transform(val) if result.satisfied() => Some(val.as_json()),
            _ => None,
        };
        Self {
            name: result.ty().name(),
            input: result.input().as_json(),
            output,
            satisfied: result.satisfied(),
            reason: reason(result.rationale()),
            rationale: support(result),
        }
    }
    pub fn collapse(mut self) -> Self {
        self.rationale = if self.satisfied {
            Vec::new()
        } else {
            deeply_unsatisfied(self.rationale)
        };
        self.input = serde_json::json!("<collapsed>");
        self.output = self.output.map(|_| serde_json::json!("<collapsed>"));
        self
    }
    fn has_input(&self) -> bool {
        match &self.input {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Array(v) => !v.is_empty(),
            Value::Object(m) => !m.is_empty(),
            _ => true,
        }
    }
}

fn deeply_unsatisfied(tree: Vec<Response>) -> Vec<Response> {
    let mut result = Vec::new();
    for i in tree.into_iter() {
        if !i.satisfied {
            // We want the deepest relevant response with input
            if i.has_input() && i.rationale.iter().all(|x| x.satisfied || !x.has_input()) {
                // We're assuming no descendents have unsatisfied
                // input if no children do. If wrong, we must recur.
                result.push(i);
            } else {
                result.append(&mut deeply_unsatisfied(i.rationale.clone()));
            }
        }
    }
    result
}

fn reason(rationale: &Rationale) -> String {
    let tmp;
    match rationale {
        Rationale::Anything => "anything is satisfied by anything",
        Rationale::Nothing
        | Rationale::Const(_)
        | Rationale::Primordial(_)
        | Rationale::Expression(_) => "",
        Rationale::Object(_) => {
            if rationale.satisfied() {
                "because all fields were satisfied"
            } else {
                "because not all fields were satisfied"
            }
        }
        Rationale::List(_terms) => {
            if rationale.satisfied() {
                "because all members were satisfied"
            } else {
                "because not all members were satisfied"
            }
        }
        Rationale::Chain(_terms) => {
            if rationale.satisfied() {
                "because the chain was satisfied"
            } else {
                "because the chain was not satisfied"
            }
        }
        Rationale::NotAnObject => "not an object",
        Rationale::NotAList => "not a list",
        Rationale::MissingField(name) => {
            tmp = format!("missing field: {name}");
            &tmp
        }
        Rationale::InvalidArgument(name) => {
            tmp = format!("invalid argument: {name}");
            &tmp
        }
        Rationale::Function(_, _, _) | Rationale::Refinement(_, _) => "",
    }
    .into()
}

fn support(result: &EvaluationResult) -> Vec<Response> {
    match result.rationale() {
        Rationale::Object(fields) => fields
            .iter()
            .filter_map(|(_, r)| r.as_ref().map(Response::new))
            .collect(),
        Rationale::List(terms) | Rationale::Chain(terms) | Rationale::Function(_, _, terms) => {
            terms.iter().map(Response::new).collect()
        }
        Rationale::Refinement(primary, refinement) => match refinement {
            Some(r) => vec![Response::new(primary), Response::new(r)],
            None => vec![Response::new(primary)],
        },
        Rationale::Anything
        | Rationale::Nothing
        | Rationale::NotAnObject
        | Rationale::NotAList
        | Rationale::MissingField(_)
        | Rationale::InvalidArgument(_)
        | Rationale::Const(_)
        | Rationale::Primordial(_)
        | Rationale::Expression(_) => Vec::new(),
    }
}

#[cfg(test)]
mod test {
    use super::Response;
    use crate::runtime::testutil::test_pattern;
    use serde_json::json;

    #[tokio::test]
    async fn bindings() {
        let result = test_pattern(r#"lang::or<["x", "y"]>"#, "foo").await;
        assert!(!result.satisfied());
        assert_eq!(
            r#"{"name":"lang::or","input":"foo","satisfied":false,"rationale":[{"input":"foo","satisfied":false},{"input":"foo","satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn happy_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 42, 99])).await;
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":"list::any","input":[1,42,99],"output":[1,42,99],"satisfied":true,"rationale":[{"input":1,"satisfied":false},{"input":42,"output":42,"satisfied":true},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":"list::any","input":"<collapsed>","output":"<collapsed>","satisfied":true}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn sad_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 99])).await;
        assert!(!result.satisfied());
        assert_eq!(
            r#"{"name":"list::any","input":[1,99],"satisfied":false,"rationale":[{"input":1,"satisfied":false},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":"list::any","input":"<collapsed>","satisfied":false,"rationale":[{"input":1,"satisfied":false},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }
}

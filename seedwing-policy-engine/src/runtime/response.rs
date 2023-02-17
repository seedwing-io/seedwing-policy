use serde::{Deserialize, Serialize};

use super::{rationale::Rationale, EvaluationResult, TypeName};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<TypeName>,
    input: serde_json::Value,
    satisfied: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rationale: Vec<Response>,
}

impl Response {
    pub fn new(result: &EvaluationResult) -> Self {
        Self {
            name: result.ty().name(),
            input: result.input().as_json(),
            satisfied: result.satisfied(),
            reason: reason(result.rationale()),
            rationale: support(result),
        }
    }
    pub fn collapse(mut self) -> Self {
        self.rationale = if self.satisfied {
            Vec::new()
        } else {
            unsatisfied_leaves(self.rationale)
        };
        self.input = serde_json::json!("<collapsed>");
        self
    }
}

fn unsatisfied_leaves(tree: Vec<Response>) -> Vec<Response> {
    let mut result = Vec::new();
    for i in tree.into_iter() {
        if i.satisfied {
            continue;
        }
        if i.rationale.is_empty() {
            result.push(i);
        } else {
            result.append(&mut unsatisfied_leaves(i.rationale.clone()));
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
        Rationale::Function(_, _, _) => "",
        Rationale::Refinement(_primary, _refinement) => todo!(),
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
        Rationale::Anything
        | Rationale::Nothing
        | Rationale::NotAnObject
        | Rationale::NotAList
        | Rationale::MissingField(_)
        | Rationale::InvalidArgument(_)
        | Rationale::Const(_)
        | Rationale::Primordial(_)
        | Rationale::Refinement(_, _)
        | Rationale::Expression(_) => Vec::new(),
    }
}

#[cfg(test)]
mod test {
    use super::Response;
    use crate::lang::{builder::Builder, lir::EvalContext};
    use crate::runtime::sources::Ephemeral;
    use serde_json::json;

    #[tokio::test]
    async fn happy_any_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern foo = list::any<42>
        "#,
        );
        let mut builder = Builder::new();
        let _ = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::foo", json!([1, 42, 99]), EvalContext::default())
            .await
            .unwrap();
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":"list::any","input":[1,42,99],"satisfied":true,"rationale":[{"input":1,"satisfied":false},{"input":42,"satisfied":true},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":"list::any","input":"<collapsed>","satisfied":true}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn sad_any_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern foo = list::any<42>
        "#,
        );
        let mut builder = Builder::new();
        let _ = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::foo", json!([1, 99]), EvalContext::default())
            .await
            .unwrap();
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

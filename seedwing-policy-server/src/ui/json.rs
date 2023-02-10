use seedwing_policy_engine::runtime::{rationale::Rationale, EvaluationResult, TypeName};
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Result {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<TypeName>,
    input: serde_json::Value,
    satisfied: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rationale: Vec<Result>,
}

impl Result {
    pub fn new(result: &EvaluationResult) -> Self {
        let input = match result.input() {
            Some(input) => input.as_json(),
            None => serde_json::Value::Null,
        };

        Self {
            name: result.ty().name(),
            input,
            satisfied: result.satisfied(),
            reason: reason(result.rationale()),
            rationale: support(result),
        }
    }
}

fn reason(rationale: &Rationale) -> String {
    match rationale {
        Rationale::Anything => "anything is satisfied by anything".to_string(),
        Rationale::Nothing
        | Rationale::Const(_)
        | Rationale::Primordial(_)
        | Rationale::Expression(_) => "".to_string(),
        Rationale::Object(_) => if rationale.satisfied() {
            "because all fields were satisfied"
        } else {
            "because not all fields were satisfied"
        }
        .to_string(),
        Rationale::List(_terms) => if rationale.satisfied() {
            "because all members were satisfied"
        } else {
            "because not all members were satisfied"
        }
        .to_string(),
        Rationale::Chain(_terms) => if rationale.satisfied() {
            "because the chain was satisfied"
        } else {
            "because the chain was not satisfied"
        }
        .to_string(),
        Rationale::NotAnObject => "not an object".to_string(),
        Rationale::NotAList => "not a list".to_string(),
        Rationale::MissingField(name) => format!("missing field: {name}"),
        Rationale::InvalidArgument(name) => format!("invalid argument: {name}"),
        Rationale::Function(_, _, _) => String::new(),
        Rationale::Refinement(_primary, _refinement) => todo!(),
    }
}

fn support(result: &EvaluationResult) -> Vec<Result> {
    match result.rationale() {
        Rationale::Object(fields) => fields.iter().map(|(_, r)| Result::new(r)).collect(),
        Rationale::List(terms) | Rationale::Chain(terms) | Rationale::Function(_, _, terms) => {
            terms.iter().map(Result::new).collect()
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
    use seedwing_policy_engine::lang::builder::Builder;
    use seedwing_policy_engine::lang::lir::EvalContext;
    use seedwing_policy_engine::runtime::sources::Ephemeral;
    use serde_json::json;

    #[tokio::test]
    async fn any_literal() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern any = list::any<42>
        "#,
        );
        let mut builder = Builder::new();
        let _ = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::any", json!([1, 42, 99]), EvalContext::default())
            .await
            .unwrap();
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":"list::any","input":[1,42,99],"satisfied":true,"rationale":[{"input":1,"satisfied":false},{"input":42,"satisfied":true},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&super::Result::new(&result)).unwrap()
        );
    }
}

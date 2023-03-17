//! Response handling a policy decision.

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use crate::{lang::PrimordialPattern, runtime::Output};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::lang::lir::{Bindings, InnerPattern, ValuePattern};

use super::{rationale::Rationale, EvaluationResult, PatternName};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Name {
    #[serde(rename = "pattern")]
    Pattern(Option<PatternName>),
    #[serde(rename = "field")]
    Field(String),
}

impl Name {
    pub fn is_empty(&self) -> bool {
        self.to_string().is_empty()
    }
}

impl Default for Name {
    fn default() -> Self {
        Self::Pattern(None)
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pattern(Some(p)) => p.fmt(f),
            Self::Pattern(None) => write!(f, ""),
            Self::Field(s) => s.fmt(f),
        }
    }
}

/// A response is used to transform a policy result into different formats.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Name::is_empty")]
    pub name: Name,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bindings: HashMap<String, Value>,
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
        let none = Bindings::default();
        let (rationale, bindings) = match result.rationale() {
            Rationale::Bound(i, b) => (i.as_ref(), b),
            r => (r, &none),
        };
        Self {
            name: Name::Pattern(result.ty().name()),
            input: result.input().as_json(),
            output,
            satisfied: result.satisfied(),
            reason: reason(rationale),
            rationale: support(rationale),
            bindings: bound(bindings),
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

fn bound(bindings: &Bindings) -> HashMap<String, Value> {
    bindings
        .iter()
        .map(|(k, v)| (k.clone(), display(v.inner())))
        .collect()
}

fn display(inner: &InnerPattern) -> Value {
    match inner {
        InnerPattern::Object(p) => json!(p
            .fields()
            .iter()
            .map(|f| (f.name(), display(f.ty().inner())))
            .collect::<HashMap<String, Value>>()),
        InnerPattern::Ref(_, _, v) | InnerPattern::List(v) => {
            json!(v
                .iter()
                .map(|p| display(p.inner()))
                // empty lists are noisy
                .filter(|v| match v {
                    Value::Array(a) => !a.is_empty(),
                    _ => true,
                })
                // deeply nested single-element lists are noisy
                .map(|v| match v {
                    Value::Array(ref a) =>
                        if a.len() == 1 {
                            a[0].clone()
                        } else {
                            v
                        },
                    _ => v,
                })
                .collect::<Value>())
        }
        InnerPattern::Const(c) => match c {
            ValuePattern::Null => Value::Null,
            ValuePattern::String(v) => json!(v),
            ValuePattern::Integer(v) => json!(v),
            ValuePattern::Decimal(v) => json!(v),
            ValuePattern::Boolean(v) => json!(v),
            ValuePattern::List(v) => json!(v),
            ValuePattern::Octets(v) => json!(v),
        },
        InnerPattern::Bound(p, b) => json!(vec![
            json!(p.name()),
            json!(b
                .iter()
                .map(|(n, i)| (n, display(i.inner())))
                .collect::<HashMap<&String, Value>>())
        ]),
        InnerPattern::Deref(p) => match p.name() {
            Some(name) => json!(HashMap::from([(name, display(p.inner()))])),
            None => display(p.inner()),
        },
        InnerPattern::Primordial(PrimordialPattern::Function(_, name, _)) => {
            json!(name.as_type_str())
        }
        ip => json!(ip),
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
        Rationale::Function(_, r, _) => match r {
            Some(x) => {
                tmp = reason(x);
                &tmp
            }
            None => "",
        },
        Rationale::Refinement(_, _) => "",
        Rationale::Bound(inner, _) => {
            tmp = reason(inner);
            &tmp
        }
    }
    .into()
}

fn support(rationale: &Rationale) -> Vec<Response> {
    match rationale {
        Rationale::Object(fields) => fields
            .iter()
            .filter_map(|(n, r)| {
                r.as_ref().map(|er| {
                    let v = Response::new(er);
                    if v.rationale.is_empty() {
                        let mut x = v.clone();
                        x.name = Name::Field(n.to_string());
                        x.rationale = vec![v];
                        x
                    } else {
                        v
                    }
                })
            })
            .collect(),
        Rationale::List(terms) | Rationale::Chain(terms) | Rationale::Function(_, _, terms) => {
            terms.iter().map(Response::new).collect()
        }
        Rationale::Refinement(primary, refinement) => match refinement {
            Some(r) => vec![Response::new(primary), Response::new(r)],
            None => vec![Response::new(primary)],
        },
        Rationale::Bound(inner, _) => support(inner),
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
    use crate::runtime::{testutil::test_pattern, PatternName};
    use serde_json::json;

    #[tokio::test]
    async fn bindings() {
        let pat = r#"
            person<65>
            pattern person<AGE> = {
              age: AGE
            }
            "#;
        let result = test_pattern(pat, json!({"age": 65})).await;
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":{"pattern":"test::person"},"bindings":{"AGE":65},"input":{"age":65},"output":{"age":65},"satisfied":true,"reason":"because all fields were satisfied","rationale":[{"name":{"field":"age"},"input":65,"output":65,"satisfied":true,"rationale":[{"input":65,"output":65,"satisfied":true}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        let result = test_pattern(pat, json!({"age": 42})).await;
        assert!(!result.satisfied());
        assert_eq!(
            r#"{"name":{"pattern":"test::person"},"bindings":{"AGE":65},"input":{"age":42},"satisfied":false,"reason":"because not all fields were satisfied","rationale":[{"name":{"field":"age"},"input":42,"satisfied":false,"rationale":[{"input":42,"satisfied":false}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn happy_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 42, 99])).await;
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"input":[1,42,99],"output":[1,42,99],"satisfied":true,"rationale":[{"input":1,"satisfied":false},{"input":42,"output":42,"satisfied":true},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"input":"<collapsed>","output":"<collapsed>","satisfied":true}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn sad_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 99])).await;
        assert!(!result.satisfied());
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"input":[1,99],"satisfied":false,"rationale":[{"input":1,"satisfied":false},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"input":"<collapsed>","satisfied":false,"rationale":[{"input":1,"satisfied":false},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn invalid_argument() {
        let result = test_pattern("uri::purl", "https:://google.com").await;

        assert_eq!(
            r#"{"name":{"pattern":"uri::purl"},"input":"https:://google.com","satisfied":false,"reason":"invalid argument: input is not a URL: empty host"}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn object_field_names() {
        let result = test_pattern(r#"{ trained: boolean }"#, json!({"trained": "true"})).await;

        assert_eq!(
            r#"{"name":{"pattern":"test::test-pattern"},"input":{"trained":"true"},"satisfied":false,"reason":"because not all fields were satisfied","rationale":[{"name":{"field":"trained"},"input":"true","satisfied":false,"rationale":[{"name":{"pattern":"boolean"},"input":"true","satisfied":false}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );

        let result = test_pattern(
            r#"
            {
              trained: is_trained
            }
            pattern is_trained = true
            "#,
            json!({"trained": "true"}),
        )
        .await;

        assert_eq!(
            r#"{"name":{"pattern":"test::test-pattern"},"input":{"trained":"true"},"satisfied":false,"reason":"because not all fields were satisfied","rationale":[{"name":{"field":"trained"},"input":"true","satisfied":false,"rationale":[{"name":{"pattern":"test::is_trained"},"input":"true","satisfied":false}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        )
    }

    #[tokio::test]
    async fn nested_any() {
        let result = test_pattern("list::any<list::none<98>>", json!([[1, 99]])).await;
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":["list::none",{"pattern":98}]},"input":"<collapsed>","output":"<collapsed>","satisfied":true}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn name_serialization() {
        let none = super::Name::Pattern(None);
        assert_eq!(
            none,
            serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap()
        );
        let none = super::Name::Pattern(Some(PatternName::new(None, String::new())));
        assert_eq!(
            none,
            serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap()
        );
        let none = super::Name::Field(String::new());
        assert_eq!(
            none,
            serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap()
        );
    }

    #[tokio::test]
    async fn primordial_function() {
        let result = test_pattern(
            "base64::base64(x509::pem)",
            "LS0tLS1CRUdJTiBDRVJUSUZJQ0FURS0tLS0tCk1JSUNFakNDQVhzQ0FnMzZNQTBHQ1NxR1NJYjNEUUVCQlFVQU1JR2JNUXN3Q1FZRFZRUUdFd0pLVURFT01Bd0cKQTFVRUNCTUZWRzlyZVc4eEVEQU9CZ05WQkFjVEIwTm9kVzh0YTNVeEVUQVBCZ05WQkFvVENFWnlZVzVyTkVSRQpNUmd3RmdZRFZRUUxFdzlYWldKRFpYSjBJRk4xY0hCdmNuUXhHREFXQmdOVkJBTVREMFp5WVc1ck5FUkVJRmRsCllpQkRRVEVqTUNFR0NTcUdTSWIzRFFFSkFSWVVjM1Z3Y0c5eWRFQm1jbUZ1YXpSa1pDNWpiMjB3SGhjTk1USXcKT0RJeU1EVXlOalUwV2hjTk1UY3dPREl4TURVeU5qVTBXakJLTVFzd0NRWURWUVFHRXdKS1VERU9NQXdHQTFVRQpDQXdGVkc5cmVXOHhFVEFQQmdOVkJBb01DRVp5WVc1ck5FUkVNUmd3RmdZRFZRUUREQTkzZDNjdVpYaGhiWEJzClpTNWpiMjB3WERBTkJna3Foa2lHOXcwQkFRRUZBQU5MQURCSUFrRUFtL3hta0htRVFydXJFLzByZS9qZUZSTGwKOFpQakJvcDd1TEhobmlhN2xRRy81ekR0WklVQzNSVnBxRFN3QnV3L05Ud2VHeXVQK284QUc5OEh4cXhUQndJRApBUUFCTUEwR0NTcUdTSWIzRFFFQkJRVUFBNEdCQUJTMlRMdUJlVFBtY2FUYVVXL0xDQjJOWU95OEdNZHpSMW14CjhpQkl1Mkg2L0UydGlZM1JJZXZWMk9XNjFxWTIvWFJRZzdZUHh4M2ZmZVV1Z1g5RjRKL2lQbm51MXpBeHh5QnkKMlZndUt2NFNXalJGb1JrSWZJbEhYMHFWdmlNaFNsTnkyaW9GTHk3SmNQWmIrdjNmdERHeXdVcWNCaVZEb2VhMApIbitHbXhaQQotLS0tLUVORCBDRVJUSUZJQ0FURS0tLS0tCg==",
        )
        .await;
        assert_eq!(
            r#"{"name":{"pattern":"test::test-pattern"},"bindings":{"terms":[]},"input":"<collapsed>","output":"<collapsed>","satisfied":true}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }
}

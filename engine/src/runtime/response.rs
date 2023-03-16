//! Response handling a policy decision.

use std::fmt::{Display, Formatter};

use crate::runtime::Output;
use serde::{de::Unexpected, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::{rationale::Rationale, EvaluationResult, PatternName};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Name {
    Pattern(Option<PatternName>),
    Field(String),
}

impl Default for Name {
    fn default() -> Self {
        Self::Pattern(None)
    }
}

impl Name {
    pub fn is_empty(&self) -> bool {
        self.to_string().is_empty()
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

impl Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Self::Pattern(Some(p)) => format!("pat>{}", p.as_type_str()),
            Self::Pattern(None) => String::new(),
            Self::Field(s) => format!("fld>{s}"),
        };
        s.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        //
        let s: &str = Deserialize::deserialize(deserializer)?;
        if s.is_empty() {
            Ok(Self::Pattern(None))
        } else {
            match s.split_once('>') {
                Some(("pat", name)) => Ok(Self::Pattern(Some(name.to_string().into()))),
                Some(("fld", name)) => Ok(Self::Field(name.to_string())),
                None | Some(_) => Err(serde::de::Error::invalid_value(
                    Unexpected::Str(s),
                    &"'pat' or 'fld' prefixed name",
                )),
            }
        }
    }
}

/// A response is used to transform a policy result into different formats.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Name::is_empty")]
    pub name: Name,
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
            name: Name::Pattern(result.ty().name()),
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
        Rationale::Function(_, r, _) => match r {
            Some(x) => {
                tmp = reason(x);
                &tmp
            }
            None => "",
        },
        Rationale::Refinement(_, _) => "",
    }
    .into()
}

fn support(result: &EvaluationResult) -> Vec<Response> {
    match result.rationale() {
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
            r#"{"name":"pat>lang::or","input":"foo","satisfied":false,"rationale":[{"input":"foo","satisfied":false},{"input":"foo","satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn happy_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 42, 99])).await;
        assert!(result.satisfied());
        assert_eq!(
            r#"{"name":"pat>list::any","input":[1,42,99],"output":[1,42,99],"satisfied":true,"rationale":[{"input":1,"satisfied":false},{"input":42,"output":42,"satisfied":true},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":"pat>list::any","input":"<collapsed>","output":"<collapsed>","satisfied":true}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn sad_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 99])).await;
        assert!(!result.satisfied());
        assert_eq!(
            r#"{"name":"pat>list::any","input":[1,99],"satisfied":false,"rationale":[{"input":1,"satisfied":false},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":"pat>list::any","input":"<collapsed>","satisfied":false,"rationale":[{"input":1,"satisfied":false},{"input":99,"satisfied":false}]}"#,
            serde_json::to_string(&Response::new(&result).collapse()).unwrap()
        );
    }

    #[tokio::test]
    async fn invalid_argument() {
        let result = test_pattern("uri::purl", "https:://google.com").await;

        assert_eq!(
            r#"{"name":"pat>uri::purl","input":"https:://google.com","satisfied":false,"reason":"invalid argument: input is not a URL: empty host"}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn object_field_names() {
        let result = test_pattern(r#"{ trained: boolean }"#, json!({"trained": "true"})).await;

        assert_eq!(
            r#"{"name":"pat>test::test-pattern","input":{"trained":"true"},"satisfied":false,"reason":"because not all fields were satisfied","rationale":[{"name":"fld>trained","input":"true","satisfied":false,"rationale":[{"name":"pat>boolean","input":"true","satisfied":false}]}]}"#,
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
            r#"{"name":"pat>test::test-pattern","input":{"trained":"true"},"satisfied":false,"reason":"because not all fields were satisfied","rationale":[{"name":"fld>trained","input":"true","satisfied":false,"rationale":[{"name":"pat>test::is_trained","input":"true","satisfied":false}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }
}

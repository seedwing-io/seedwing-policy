//! Response handling a policy decision.

mod collector;

pub use collector::*;

use super::{rationale::Rationale, EvaluationResult, PatternName};
use crate::{
    lang::{
        lir::{Bindings, InnerPattern, ValuePattern},
        PrimordialPattern, Severity,
    },
    runtime::{is_default, Output},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_view::View;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Name {
    Pattern(Option<PatternName>),
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, View)]
pub struct Response {
    #[serde(default, skip_serializing_if = "Name::is_empty")]
    pub name: Name,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bindings: HashMap<String, Value>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub input: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    /// An indicator that this reason considers itself authoritative.
    ///
    /// A setting of `true` indicates that this reason is informational enough for the end-user
    /// to understand what happened and that rationale entries below this one don't help much
    /// in getting a better understanding (except when debugging).
    #[serde(default, skip_serializing_if = "is_default")]
    pub authoritative: bool,
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
        let (severity, reason) = result.outcome();
        let output = match &result.output {
            Output::Identity if !matches!(severity, Severity::Error) => {
                Some(result.input.as_json())
            }
            Output::Transform(val) if !matches!(severity, Severity::Error) => Some(val.as_json()),
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
            severity,
            reason,
            authoritative: result.ty.metadata().reporting.authoritative,
            rationale: support(rationale),
            bindings: bound(bindings),
        }
    }

    /// Collapse the tree of reasons.
    ///
    /// This collects the reasons using [`Self::collect`] and replaces the current rationale with
    /// them.
    pub fn collapse(mut self, severity: Severity) -> Self {
        self.rationale = Collector::new(&self).with_severity(severity).collect();
        self
    }

    /// Walk the tree of reasons.
    ///
    /// The callback can return `true` if it want to keep descending into the children of
    /// the current response, or `false` otherwise.
    pub fn walk_tree<F>(&self, mut f: F)
    where
        F: FnMut(&Response) -> bool,
    {
        self.walk_tree_internal(&mut f);
    }

    /// An internal version of `walk_tree`, which takes a reference to the callback, so
    /// that it can be called recursively.
    fn walk_tree_internal<F>(&self, f: &mut F)
    where
        F: FnMut(&Response) -> bool,
    {
        if f(self) {
            for x in &self.rationale {
                x.walk_tree_internal(f);
            }
        }
    }

    /// Evaluate if the reason is "satisfied"
    ///
    /// A reason is satisfied if its severity is lower than the requested severity.
    fn satisfied(&self, severity: Severity) -> bool {
        self.severity < severity
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

fn support(rationale: &Rationale) -> Vec<Response> {
    match rationale {
        Rationale::Object(fields) => {
            let mut result = fields
                .iter()
                .filter_map(|(n, r)| {
                    r.as_ref().map(|er| {
                        let v = Response::new(er);
                        if v.rationale.is_empty() {
                            let mut x = v.clone();
                            let (severity, reason) = er.outcome();
                            x.name = Name::Field(n.to_string());
                            x.severity = severity;
                            x.reason = reason;
                            x.rationale = vec![v];
                            x
                        } else {
                            v
                        }
                    })
                })
                .collect::<Vec<_>>();

            result.sort_unstable_by(|a, b| a.name.cmp(&b.name));

            result
        }
        Rationale::List(terms)
        | Rationale::Chain(terms)
        | Rationale::Function {
            supporting: terms, ..
        } => terms.iter().map(Response::new).collect(),
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
    use crate::lang::Severity;
    use crate::runtime::{response::Name, testutil::test_pattern, PatternName};
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;
    use serde_view::View;

    #[tokio::test]
    async fn bindings() {
        let pat = r#"
            person<65>
            pattern person<AGE> = {
              age: AGE
            }
            "#;
        let result = test_pattern(pat, json!({"age": 65})).await;
        assert_satisfied!(&result);
        assert_eq!(
            r#"{"name":{"pattern":"test::person"},"bindings":{"AGE":65},"input":{"age":65},"output":{"age":65},"severity":"none","reason":"Because all fields were satisfied","rationale":[{"name":{"field":"age"},"input":65,"output":65,"severity":"none","reason":"The input matches the expected constant value expected in the pattern","rationale":[{"input":65,"output":65,"severity":"none","reason":"The input matches the expected constant value expected in the pattern"}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        let result = test_pattern(pat, json!({"age": 42})).await;
        assert_not_satisfied!(&result);
        assert_eq!(
            r#"{"name":{"pattern":"test::person"},"bindings":{"AGE":65},"input":{"age":42},"severity":"error","reason":"Because not all fields were satisfied","rationale":[{"name":{"field":"age"},"input":42,"severity":"error","reason":"The input does not match the constant value expected in the pattern","rationale":[{"input":42,"severity":"error","reason":"The input does not match the constant value expected in the pattern"}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn happy_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 42, 99])).await;
        assert_satisfied!(&result);
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"input":[1,42,99],"reason":"The input satisfies the function"}"#,
            serde_json::to_string(
                &Response::new(&result)
                    .as_view()
                    .with_fields("name,reason,input".split(","))
            )
            .unwrap()
        );
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"severity":"none"}"#,
            serde_json::to_string(
                &Response::new(&result)
                    .collapse(Severity::Error)
                    .as_view()
                    .with_fields("name,bindings,severity".split(","))
            )
            .unwrap()
        );
    }

    #[tokio::test]
    async fn sad_any_literal() {
        let result = test_pattern("list::any<42>", json!([1, 99])).await;
        assert_not_satisfied!(&result);
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"input":[1,99],"severity":"error","reason":"The input does not satisfy the function","rationale":[{"input":1,"severity":"error","reason":"The input does not match the constant value expected in the pattern"},{"input":99,"severity":"error","reason":"The input does not match the constant value expected in the pattern"}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":42},"reason":"The input does not satisfy the function","rationale":[{"input":1,"severity":"error","reason":"The input does not match the constant value expected in the pattern"},{"input":99,"severity":"error","reason":"The input does not match the constant value expected in the pattern"}]}"#,
            serde_json::to_string(
                &Response::new(&result)
                    .collapse(Severity::Error)
                    .as_view()
                    .with_fields("name,bindings,reason,rationale".split(","))
            )
            .unwrap()
        );
    }

    #[tokio::test]
    async fn invalid_argument() {
        let result = test_pattern("uri::purl", "https:://google.com").await;

        assert_eq!(
            r#"{"name":{"pattern":"uri::purl"},"input":"https:://google.com","severity":"error","reason":"invalid argument: input is not a URL: empty host"}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        );
    }

    #[tokio::test]
    async fn object_field_names() {
        let result = test_pattern(r#"{ trained: boolean }"#, json!({"trained": "true"})).await;

        assert_eq!(
            r#"{"name":{"pattern":"test::test-pattern"},"input":{"trained":"true"},"severity":"error","reason":"Because not all fields were satisfied","rationale":[{"name":{"field":"trained"},"input":"true","severity":"error","reason":"The primordial type defined in the pattern is not satisfied","rationale":[{"name":{"pattern":"boolean"},"input":"true","severity":"error","reason":"The primordial type defined in the pattern is not satisfied"}]}]}"#,
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
            r#"{"name":{"pattern":"test::test-pattern"},"input":{"trained":"true"},"severity":"error","reason":"Because not all fields were satisfied","rationale":[{"name":{"field":"trained"},"input":"true","severity":"error","reason":"The input does not match the constant value expected in the pattern","rationale":[{"name":{"pattern":"test::is_trained"},"input":"true","severity":"error","reason":"The input does not match the constant value expected in the pattern"}]}]}"#,
            serde_json::to_string(&Response::new(&result)).unwrap()
        )
    }

    #[tokio::test]
    async fn nested_any() {
        let result = test_pattern("list::any<list::none<98>>", json!([[1, 99]])).await;
        assert_satisfied!(&result);
        assert_eq!(
            r#"{"name":{"pattern":"list::any"},"bindings":{"pattern":["list::none",{"pattern":98}]},"input":[[1,99]],"output":[[1,99]],"severity":"none","reason":"The input satisfies the function"}"#,
            serde_json::to_string(&Response::new(&result).collapse(Severity::Error)).unwrap()
        );
    }

    #[tokio::test]
    async fn name_serialization() {
        let none = Name::Pattern(None);
        assert_eq!(
            none,
            serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap()
        );
        let none = Name::Pattern(Some(PatternName::new(None, String::new())));
        assert_eq!(
            none,
            serde_json::from_str(&serde_json::to_string(&none).unwrap()).unwrap()
        );
        let none = Name::Field(String::new());
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
            r#"{"name":{"pattern":"test::test-pattern"},"bindings":{"terms":[]},"severity":"none"}"#,
            serde_json::to_string(
                &Response::new(&result)
                    .collapse(Severity::Error)
                    .as_view()
                    .with_fields("name,bindings,severity".split(","))
            )
            .unwrap()
        );
    }

    #[test]
    fn test_ord() {
        let mut names = vec![
            Name::Pattern(Some("baz::foo".into())),
            Name::Pattern(None),
            Name::Field("foo".to_string()),
            Name::Field("bar".to_string()),
        ];

        names.sort();

        // convert to strings
        let names = names.into_iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // and then to &str, making it easier to assert
        let names = names.iter().map(|s| s.as_str()).collect::<Vec<_>>();

        assert_eq!(names, vec!["", "baz::foo", "bar", "foo"]);
    }
}

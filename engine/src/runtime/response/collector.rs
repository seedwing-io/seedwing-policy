use crate::{lang::Severity, runtime::Response};

/// Collect the list of reasons.
///
/// This builds a list of reasons which are reachable from the root through only failed
/// nodes. Capturing only the deepest failed nodes, which have no more failures underneath them.
/// But it does not capture its (succeeded) children.
pub struct Collector<'r> {
    pub response: &'r Response,
    pub severity: Severity,
    pub ignore_authoritative: bool,
}

impl<'r> Collector<'r> {
    /// Create a new collector with default settings for the provided response.
    pub fn new(response: &'r Response) -> Self {
        Self {
            response,
            severity: Severity::Error,
            ignore_authoritative: false,
        }
    }

    /// Override the severity which defines if a reason is considered failed.
    ///
    /// The default is [`Severity::Error`], a reason is considered failed if its severity is equal
    /// or greater than the provided severity.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Use the severity of the response.
    ///
    /// This helps getting the "highest" severity entries automatically.
    ///
    /// **NOTE:** This will only work properly of the severity of the response is higher than
    /// [`Severity::None`], otherwise you will get all leaf entries.
    pub fn highest_severity(mut self) -> Self {
        self.severity = self.response.severity;
        self
    }

    /// Ignore the authoritative flag.
    pub fn ignore_authoritative(self) -> Self {
        self.with_ignore_authoritative(true)
    }

    /// Define whether the authoritative flag should be respected, or not.
    pub fn with_ignore_authoritative(mut self, ignore_authoritative: bool) -> Self {
        self.ignore_authoritative = ignore_authoritative;
        self
    }

    /// Perform the collection process.
    pub fn collect(&self) -> Vec<Response> {
        let mut rationale = vec![];

        self.response.walk_tree(|response| {
            if response.satisfied(self.severity) {
                // we are satisfied, so we don't descend and skip it
                return false;
            }

            if response.authoritative
                || response
                    .rationale
                    .iter()
                    .all(|r| r.satisfied(self.severity))
            {
                // we are not satisfied, but all our children are, or we are authoritative
                // -> record ourself as an outer most (failed) entry
                let mut response = response.clone();
                response.rationale = vec![];
                rationale.push(response);
                return false;
            }

            // keep descending
            true
        });

        rationale
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::assert_not_satisfied;
    use crate::runtime::testutil::test_common;
    use crate::test::Reason;
    use serde_json::json;

    #[tokio::test]
    async fn collect_authoritative() {
        let result = test_common(
            r#"
pattern test = list::all<inner>

#[authoritative]
#[explain("find me")]
pattern inner = {
    values?: find,
}

pattern find = list::none<predicate>

pattern predicate = "foo"
"#,
            json!([
                { "values": ["foo", "bar"] },
                { "values": ["bar", "baz"] },
                { "values": ["baz"] }
            ]),
        )
        .await;
        assert_not_satisfied!(&result);

        let result = Response::new(&result);

        println!("{}", serde_json::to_string_pretty(&result).unwrap());

        let collect = Collector::new(&result).collect();
        let collect = collect.into_iter().map(Reason::from).collect::<Vec<_>>();
        assert_eq!(
            collect.as_slice(),
            &[Reason {
                name: "test::inner".to_string(),
                severity: Severity::Error,
                reason: "find me".to_string(),
                rationale: vec![],
            }]
        );
    }
}

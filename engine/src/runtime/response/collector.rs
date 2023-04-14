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

use crate::{
    lang::{hir, Severity},
    runtime::{is_default, metadata::Documentation, BuildError},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Documentation,
    pub unstable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<Deprecation>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub reporting: Reporting,
}

impl PatternMeta {
    #[inline]
    pub fn is_deprecated(&self) -> bool {
        self.deprecation.is_some()
    }
}

impl TryFrom<hir::Metadata> for PatternMeta {
    type Error = BuildError;

    fn try_from(mut value: hir::Metadata) -> Result<Self, Self::Error> {
        Ok(Self {
            reporting: (&value).into(),
            documentation: Documentation(value.documentation),
            unstable: value.attributes.contains_key("unstable"),
            deprecation: value.attributes.remove("deprecated").map(Into::into),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Deprecation {
    pub reason: Option<String>,
    pub since: Option<String>,
}

impl From<hir::AttributeValues> for Deprecation {
    fn from(mut value: hir::AttributeValues) -> Self {
        // use the first flag type entry as reason
        let reason = value.flags().next().map(ToString::to_string);
        let since = value.values.remove("since").flatten();
        Deprecation { reason, since }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reporting {
    /// Override severity
    ///
    /// Everything other than [`Severity::None`] will override a non [`Severity::None`] value with
    /// the severity with this value.
    pub severity: Severity,
    /// In case of a non [`Severity::None`] value, this can be used to override the "reason".
    pub explanation: Option<String>,
}

impl From<&hir::Metadata> for Reporting {
    fn from(value: &hir::Metadata) -> Self {
        // try "explain", then "warning", then "advice"
        value
            .attributes
            .get("explain")
            .map(|attr| (Severity::Error, attr))
            .or_else(|| {
                value
                    .attributes
                    .get("warning")
                    .map(|attr| (Severity::Warning, attr))
            })
            .or_else(|| {
                value
                    .attributes
                    .get("advice")
                    .map(|attr| (Severity::Advice, attr))
            })
            .map(|(severity, attr)| Reporting {
                severity,
                explanation: attr.flags().next().map(ToString::to_string),
            })
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

impl PackageMeta {
    /// add documentation, append in necessary
    pub(crate) fn add_documentation(&mut self, docs: &str) {
        match &mut self.documentation {
            Some(current) => {
                current.push_str(docs);
            }
            None => self.documentation = Some(docs.to_string()),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::lang::Severity;

    #[test]
    fn test_highest() {
        let s: Severity = vec![Severity::Warning, Severity::Warning, Severity::Advice]
            .into_iter()
            .collect();
        assert_eq!(Severity::Warning, s);
    }

    #[test]
    fn test_empty() {
        let s: Severity = Vec::<Severity>::new().into_iter().collect();
        assert_eq!(Severity::None, s);
    }

    #[test]
    fn test_err() {
        let s: Severity = vec![Severity::Warning, Severity::Error, Severity::Advice]
            .into_iter()
            .collect();
        assert_eq!(Severity::Error, s);
    }
}

use crate::{
    lang::hir,
    runtime::{metadata::Documentation, BuildError},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Documentation,
    pub unstable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecation: Option<Deprecation>,
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
            documentation: Documentation(value.documentation),
            unstable: value.attributes.contains_key("unstable"),
            deprecation: value.attributes.remove("deprecated").map(Into::into),
        })
    }
}

impl From<hir::AttributeValues> for Deprecation {
    fn from(mut value: hir::AttributeValues) -> Self {
        // use the first flag type entry as reason
        let reason = value.flags().next().map(ToString::to_string);
        let since = value.values.remove("since").flatten();
        Deprecation { reason, since }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Deprecation {
    pub reason: Option<String>,
    pub since: Option<String>,
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

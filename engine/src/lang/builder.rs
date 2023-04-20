//! Builder for creating a policy engine from a set of policies and data sources.
//!
//! A builder creates a World - a representation of all policies and patterns known by an engine.
use crate::data::DataSource;
use crate::lang::hir;
use crate::lang::parser::SourceLocation;
use crate::runtime;
use crate::runtime::cache::SourceCache;
use crate::runtime::config::{ConfigContext, ConfigValue};
use crate::runtime::BuildError;

/// Builder representing the entire world of policies.
#[derive(Clone)]
pub struct Builder {
    hir: hir::World,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    /// Create a new builder with an empty world.
    pub fn new() -> Self {
        Self {
            hir: hir::World::new(),
        }
    }

    pub fn new_with_config(config: ConfigContext) -> Self {
        Self {
            hir: hir::World::new_with_config(config),
        }
    }

    /// Build policies found in the provided sources.
    pub fn build<S, SrcIter>(&mut self, sources: SrcIter) -> Result<(), Vec<BuildError>>
    where
        Self: Sized,
        S: Into<String>,
        SrcIter: Iterator<Item = (SourceLocation, S)>,
    {
        self.hir.build(sources)
    }

    /// Compile all policies into a runtime World that can be used for policy evaluation.
    pub async fn finish(&mut self) -> Result<runtime::World, Vec<BuildError>> {
        let mir = self.hir.lower()?;
        let runtime = mir.lower()?;
        Ok(runtime)
    }

    /// The source cache with all known sources for this builder.
    pub fn source_cache(&self) -> &SourceCache {
        self.hir.source_cache()
    }

    /// Add a data source to the builder.
    pub fn data<D: DataSource + 'static>(&mut self, src: D) {
        self.hir.data(src)
    }

    pub fn config<S: Into<String>, V: Into<ConfigValue>>(&mut self, key: S, val: V) {
        self.hir.config(key.into(), val.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;

    use crate::assert_satisfied;
    use serde_json::json;

    #[tokio::test]
    async fn basic_smoke_test() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        pattern named<name> = {
            name: name
        }

        pattern jim = named<"Jim">
        pattern bob = named<"Bob">

        pattern folks = jim || bob

        "#,
        );

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 52,
                    }
                ),
                EvalContext::default(),
            )
            .await;

        assert_satisfied!(result.unwrap());
    }
}

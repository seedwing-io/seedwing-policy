use crate::data::DataSource;
use crate::lang::parser::SourceLocation;
use crate::lang::{hir, lir};
use crate::runtime;
use crate::runtime::cache::SourceCache;
use crate::runtime::BuildError;

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
    pub fn new() -> Self {
        Self {
            hir: hir::World::new(),
        }
    }

    pub fn build<S, SrcIter>(&mut self, sources: SrcIter) -> Result<(), Vec<BuildError>>
    where
        Self: Sized,
        S: Into<String>,
        SrcIter: Iterator<Item = (SourceLocation, S)>,
    {
        self.hir.build(sources)
    }

    pub async fn finish(&mut self) -> Result<runtime::World, Vec<BuildError>> {
        let mir = self.hir.lower()?;
        let runtime = mir.lower()?;
        Ok(runtime)
    }

    pub fn source_cache(&self) -> &SourceCache {
        self.hir.source_cache()
    }

    pub fn data<D: DataSource + 'static>(&mut self, src: D) {
        self.hir.data(src)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::lir::EvalContext;
    use crate::runtime::sources::Ephemeral;
    use crate::value::RationaleResult;
    use serde_json::json;

    #[actix_rt::test]
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
        let result = builder.build(src.iter());
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

        assert!(result.unwrap().satisfied());
    }
}

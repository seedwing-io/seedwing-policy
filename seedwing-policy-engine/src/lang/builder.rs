use crate::lang::parser::SourceLocation;
use crate::lang::{hir, lir};
use crate::runtime::cache::SourceCache;
use crate::runtime::BuildError;

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

    pub async fn finish(&mut self) -> Result<lir::World, Vec<BuildError>> {
        let mir = self.hir.lower().await?;
        println!("MIR {:?}", mir);
        mir.lower().await
    }

    pub fn source_cache(&self) -> &SourceCache {
        self.hir.source_cache()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runtime::sources::Ephemeral;
    use serde_json::json;

    #[actix_rt::test]
    async fn basic_smoke_test() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        type named<name> = {
            name: name
        }

        type jim = named<"Jim">
        type bob = named<"Bob">

        type folks = jim || bob

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 52,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }
}

use crate::{
    lang::lir::Pattern,
    package::Package,
    runtime::{PackagePath, RuntimeError, World},
    value::RuntimeValue,
};
use std::sync::Arc;

use crate::lang::lir::EvalContext;

pub mod all;
pub mod any;
pub mod head;
pub mod none;
pub mod some;
pub mod tail;

const COUNT: &str = "count";
const PATTERN: &str = "pattern";

pub fn package() -> Package {
    let mut pkg = Package::new(PackagePath::from_parts(vec!["list"]));
    pkg.register_function("any".into(), any::Any);
    pkg.register_function("all".into(), all::All);
    pkg.register_function("none".into(), none::None);
    pkg.register_function("some".into(), some::Some);
    pkg.register_function("head".into(), head::Head);
    pkg.register_function("tail".into(), tail::Tail);
    pkg
}

pub(crate) async fn split_fill<I>(
    ctx: &EvalContext,
    world: &World,
    mut i: I,
    count: Option<Arc<Pattern>>,
) -> Result<(Vec<Arc<RuntimeValue>>, Vec<Arc<RuntimeValue>>), RuntimeError>
where
    I: Iterator<Item = Arc<RuntimeValue>> + DoubleEndedIterator,
{
    let mut greedy = Vec::new();

    loop {
        let satisfied = match &count {
            Some(expected_count) => {
                let expected_result = expected_count
                    .evaluate(
                        Arc::new((greedy.len()).into()),
                        ctx,
                        &Default::default(),
                        world,
                    )
                    .await?;
                expected_result.satisfied()
            }
            None => !greedy.is_empty(),
        };

        if satisfied {
            break;
        }

        match i.next() {
            Some(n) => {
                greedy.push(n);
            }
            None => {
                break;
            }
        }
    }

    Ok((greedy, i.collect()))
}

#[cfg(test)]
mod test {
    use crate::lang::builder::Builder;
    use crate::lang::lir::EvalContext;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvaluationResult;
    use serde_json::Value;

    pub(crate) async fn test_pattern(pattern: &str, value: Value) -> EvaluationResult {
        let src = Ephemeral::new("test", format!("pattern test-pattern = {pattern}"));

        let mut builder = Builder::new();
        builder.build(src.iter()).unwrap();
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::test-pattern", value, EvalContext::default())
            .await;

        result.unwrap()
    }
}

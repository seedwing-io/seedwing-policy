use seedwing_policy_engine::{
    runtime::{EvalContext, EvaluationResult, RuntimeError, World},
    value::RuntimeValue,
};
use std::sync::Arc;

pub struct Eval<'a> {
    world: &'a World,
    name: &'a str,
    value: Arc<RuntimeValue>,
}

impl<'a> Eval<'a> {
    pub fn new<V: Into<RuntimeValue>>(world: &'a World, name: &'a str, value: V) -> Self {
        Self {
            world,
            name,
            value: Arc::new(value.into()),
        }
    }

    pub async fn run(&self) -> Result<EvaluationResult, RuntimeError> {
        self.world
            .evaluate_fast(self.name, self.value.clone(), EvalContext::default())
            .await
    }
}

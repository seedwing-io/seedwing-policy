use seedwing_policy_engine::runtime::EvalContext;
use seedwing_policy_engine::runtime::{EvaluationResult, RuntimeError, World};
use seedwing_policy_engine::value::RuntimeValue;

pub struct Eval<'a> {
    world: &'a World,
    name: &'a str,
    value: RuntimeValue,
}

impl<'a> Eval<'a> {
    pub fn new<V: Into<RuntimeValue>>(world: &'a World, name: &'a str, value: V) -> Self {
        Self {
            world,
            name,
            value: value.into(),
        }
    }

    pub async fn run(&self) -> Result<EvaluationResult, RuntimeError> {
        self.world
            .evaluate(self.name, self.value.clone(), EvalContext::default())
            .await
    }
}

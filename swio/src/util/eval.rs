use seedwing_policy_engine::runtime::EvalContext;
use seedwing_policy_engine::runtime::{EvaluationResult, RuntimeError, World};
use seedwing_policy_engine::value::RuntimeValue;

pub struct Eval {
    world: World,
    name: String,
    value: RuntimeValue,
}

impl Eval {
    pub fn new<N: Into<String>, V: Into<RuntimeValue>>(world: World, name: N, value: V) -> Self {
        Self {
            world,
            name: name.into(),
            value: value.into(),
        }
    }

    pub async fn run(&self) -> Result<EvaluationResult, RuntimeError> {
        self.world
            .evaluate(
                self.name.clone(),
                self.value.clone(),
                EvalContext::default(),
            )
            .await
    }
}

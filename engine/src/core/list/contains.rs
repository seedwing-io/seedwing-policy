use crate::core::{Function, FunctionEvaluationResult};
use crate::lang::lir::{Bindings, InnerPattern};
use crate::runtime::{EvalContext, Output, RuntimeError, World};
use crate::value::RuntimeValue;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

const DOCUMENTATION: &str = include_str!("contains-all.adoc");
const PARAMETER: &str = "parameter";

#[derive(Debug)]
pub struct ContainsAll;

impl Function for ContainsAll {
    fn order(&self) -> u8 {
        128
    }

    fn documentation(&self) -> Option<String> {
        Some(DOCUMENTATION.into())
    }

    fn parameters(&self) -> Vec<String> {
        vec![PARAMETER.to_string()]
    }

    fn call<'v>(
        &'v self,
        input: Arc<RuntimeValue>,
        _ctx: &'v EvalContext,
        bindings: &'v Bindings,
        _world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<FunctionEvaluationResult, RuntimeError>> + 'v>> {
        Box::pin(async move {
            if let Some(binding) = bindings.get(PARAMETER) {
                if let InnerPattern::List(items) = binding.inner() {
                    if let Some(list) = input.try_get_list() {
                        // We could make this more efficient using a HashSet, but
                        // that would require us excluding f64 values from the set or live with a string hash.
                        //
                        // This should be benchmarked with more realistic input data but for small amounts it's
                        // likely to not matter.
                        for item in items.iter() {
                            if let Some(item) = item.try_get_resolved_value() {
                                let item: RuntimeValue = (&item).into();
                                let mut found = false;
                                for s in list {
                                    if item.eq(s) {
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    return Ok(Output::None.into());
                                }
                            } else {
                                return Err(RuntimeError::InvalidState);
                            }
                        }
                        return Ok(Output::Identity.into());
                    }
                }
            }
            Ok(Output::None.into())
        })
    }
}

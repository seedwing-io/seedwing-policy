use crate::core::{Function, FunctionError};
use crate::runtime::{Bindings, EvaluationResult};
use crate::value::Value;
use async_mutex::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub struct Some;

impl Function for Some {
    fn parameters(&self) -> Vec<String> {
        vec!["count".into(), "pattern".into()]
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Option::Some(list) = input.try_get_list() {
                let expected_count = bindings.get(&"count".into()).unwrap();
                let pattern = bindings.get(&"pattern".into()).unwrap();

                let mut count: u32 = 0;

                for item in list {
                    let result = pattern.evaluate(item.clone(), &Default::default()).await;

                    match result {
                        Ok(Option::Some(_)) => {
                            count += 1;
                        }
                        Ok(Option::None) => continue,
                        _ => return Err(FunctionError::InvalidInput),
                    }

                    match expected_count
                        .evaluate(Arc::new(Mutex::new(count.into())), &Default::default())
                        .await
                    {
                        Ok(Option::Some(_)) => return Ok(input.clone()),
                        Ok(Option::None) => {
                            continue;
                        }
                        Err(_) => return Err(FunctionError::InvalidInput),
                    }
                }
                Err(FunctionError::InvalidInput)
            } else {
                Err(FunctionError::InvalidInput)
            }
        })
    }
}

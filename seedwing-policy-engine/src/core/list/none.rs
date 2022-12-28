use crate::core::list::PATTERN;
use crate::core::{Function, FunctionError};
use crate::runtime::Bindings;
use crate::value::Value;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct None;

impl Function for None {
    fn parameters(&self) -> Vec<String> {
        vec![PATTERN.into()]
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(PATTERN).unwrap();
                for item in list {
                    let result = pattern.evaluate(item.clone(), &Default::default()).await;

                    match result {
                        Ok(Option::None) => continue,
                        Err(_) => return Err(FunctionError::InvalidInput),
                        Ok(Option::Some(_)) => return Err(FunctionError::InvalidInput),
                    }
                }
                Ok(input.clone())
            } else {
                Err(FunctionError::InvalidInput)
            }
        })
    }
}

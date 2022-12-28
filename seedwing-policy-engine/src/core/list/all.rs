use crate::core::{Function, FunctionError};
use crate::runtime::Bindings;
use crate::value::Value;
use std::future::Future;
use std::pin::Pin;

#[derive(Debug)]
pub struct All;

impl Function for All {
    fn parameters(&self) -> Vec<String> {
        vec!["pattern".into()]
    }

    fn call<'v>(
        &'v self,
        input: &'v Value,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<Value, FunctionError>> + 'v>> {
        Box::pin(async move {
            if let Some(list) = input.try_get_list() {
                let pattern = bindings.get(&"pattern".into()).unwrap();
                for item in list {
                    let result = pattern.evaluate(item.clone(), &Default::default()).await;

                    match result {
                        Ok(Option::None) => return Err(FunctionError::InvalidInput),
                        Err(_) => return Err(FunctionError::InvalidInput),
                        Ok(Option::Some(_)) => continue,
                    }
                }
                Ok(input.clone())
            } else {
                Err(FunctionError::InvalidInput)
            }
        })
    }
}

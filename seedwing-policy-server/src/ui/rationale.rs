use seedwing_policy_engine::value::{RationaleResult, Value};
use std::future::Future;
use std::pin::Pin;

pub struct Rationalizer {
    value: RationaleResult,
}

impl Rationalizer {
    pub fn new(value: RationaleResult) -> Self {
        Self { value }
    }

    pub async fn rationale(&self) -> String {
        let mut html = String::new();
        html.push_str("<pre>");
        match &self.value {
            RationaleResult::None => {
                html.push_str("failed");
            }
            RationaleResult::Same(value) => {
                let locked_value = value.lock().await;
                Self::rationale_inner(&mut html, &*locked_value).await;
            }
            RationaleResult::Transform(value) => {
                let locked_value = value.lock().await;
                Self::rationale_inner(&mut html, &*locked_value).await;
            }
        }

        html.push_str("<pre>");
        html
    }

    pub fn rationale_inner<'h>(
        html: &'h mut String,
        value: &'h Value,
    ) -> Pin<Box<dyn Future<Output = ()> + 'h>> {
        Box::pin(async move {
            for (k, v) in value.get_rationale() {
                match v {
                    RationaleResult::None => {
                        if let Some(description) = k.description() {
                            html.push_str(format!("did not match {}\n", description).as_str());
                        }
                    }
                    RationaleResult::Same(_) => {
                        if let Some(description) = k.description() {
                            html.push_str(format!("matched {}\n", description).as_str());
                        }
                    }
                    RationaleResult::Transform(transform) => {
                        if let Some(description) = k.description() {
                            html.push_str(
                                format!("matched {} producing a value\n", description).as_str(),
                            );
                            Self::rationale_inner(html, &*transform.lock().await).await;
                        }
                    }
                }
            }
            if let Some(list) = value.try_get_list() {
                for item in list {
                    Self::rationale_inner(html, &*item.lock().await).await;
                }
            }
            if let Some(object) = value.try_get_object() {
                for (_, v) in object.iter() {
                    Self::rationale_inner(html, &*v.lock().await).await;
                }
            }
        })
    }
}

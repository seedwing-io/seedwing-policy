use seedwing_policy_engine::value::{Rationale, RationaleResult, Value};
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
        html.push_str("<div>");
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

        html.push_str("<div>");
        html
    }

    pub fn rationale_inner<'h>(
        html: &'h mut String,
        value: &'h Value,
    ) -> Pin<Box<dyn Future<Output = ()> + 'h>> {
        Box::pin(async move {
            let rationale = value.get_rationale();
            if rationale.is_empty() {
                return;
            }
            html.push_str("<div>");
            let value_json = value.as_json().await;
            let value_json = serde_json::to_string_pretty(&value_json).unwrap();
            let value_json = value_json.replace('<', "&lt;");
            let value_json = value_json.replace('>', "&gt;");
            html.push_str("<pre class='input-value'>");
            html.push_str(value_json.as_str());
            html.push_str("</pre>");

            for (k, v) in rationale.iter().rev() {
                match v {
                    RationaleResult::None => {
                        if let Some(description) = k.description() {
                            html.push_str("<div class='entry no-match'>");
                            html.push_str(format!("did not match {}\n", description).as_str());
                            html.push_str("</div>");
                        }
                    }
                    RationaleResult::Same(_) => {
                        if let Some(description) = k.description() {
                            html.push_str("<div class='entry match'>");
                            html.push_str(
                                format!("<b><code>{}</code> matched</b>", description).as_str(),
                            );
                            html.push_str("</div>");
                        }
                    }
                    RationaleResult::Transform(transform) => {
                        if let Some(description) = k.description() {
                            html.push_str("<div class='entry match transform'>");
                            match k {
                                Rationale::Type(_) => {
                                    html.push_str(
                                        format!(
                                            "<b><code>{}</code> produced a value</b>\n",
                                            description
                                        )
                                        .as_str(),
                                    );
                                }
                                Rationale::Field(f) => {
                                    if let Some(name) = f.ty().name() {
                                        html.push_str(
                                            format!("<b>field <code>{}</code> produced a value via {}</b>\n", description, name).as_str(),
                                        );
                                    } else {
                                        html.push_str(
                                            format!(
                                                "<b>field <code>{}</code> produced a value</b>\n",
                                                description
                                            )
                                            .as_str(),
                                        );
                                    }
                                }
                                Rationale::Expr(_) => {}
                            }
                            Self::rationale_inner(html, &*transform.lock().await).await;
                            html.push_str("</div>")
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
            html.push_str("</div>");
        })
    }
}

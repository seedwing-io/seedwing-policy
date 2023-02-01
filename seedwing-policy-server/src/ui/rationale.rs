use seedwing_policy_engine::runtime::rationale::Rationale;
use seedwing_policy_engine::runtime::{EvaluationResult, Output};

pub struct Rationalizer<'r> {
    result: &'r EvaluationResult,
}

impl<'r> Rationalizer<'r> {
    pub fn new(result: &'r EvaluationResult) -> Self {
        Self { result }
    }

    pub fn rationale(&self) -> String {
        let mut html = String::new();
        html.push_str("<div>");
        Self::rationale_inner(&mut html, self.result);

        html.push_str("<div>");
        html
    }

    pub fn rationale_inner(html: &mut String, result: &EvaluationResult) {
        if result.satisfied() {
            html.push_str("<div class='entry satisfied'>");
        } else {
            html.push_str("<div class='entry unsatisfied'>");
        }

        if let Some(input) = result.input() {
            let input_json = input.as_json();
            let input_json = serde_json::to_string_pretty(&input_json).unwrap();
            let input_json = input_json.replace('<', "&lt;");
            let input_json = input_json.replace('>', "&gt;");
            html.push_str("<div class='input'>");
            html.push_str("<pre>");
            html.push_str(input_json.as_str());
            html.push_str("</pre>");
            html.push_str("</div>");

            if let Some(name) = result.ty().name() {
                html.push_str("<div>");
                if result.satisfied() {
                    html.push_str(
                        format!("<div>Type <code>{name}</code> was satisfied</div>").as_str(),
                    );
                } else {
                    html.push_str(
                        format!("<div>Type <code>{name}</code> was not satisfied</div>").as_str(),
                    );
                }

                match result.rationale() {
                    Rationale::Anything => {}
                    Rationale::Nothing => {}
                    Rationale::Chain(_) => {}
                    Rationale::Object(_) => {}
                    Rationale::List(_) => {}
                    Rationale::NotAnObject => {}
                    Rationale::NotAList => {}
                    Rationale::MissingField(_) => {}
                    Rationale::InvalidArgument(_) => {}
                    Rationale::Const(_) => {}
                    Rationale::Primordial(_) => {}
                    Rationale::Expression(_) => {}
                    Rationale::Function(_, rationale, supporting) => {
                        for each in supporting {
                            Self::rationale_inner(html, each);
                        }
                    }
                    Rationale::Refinement(_, _) => {}
                }
                html.push_str("</div>");
            } else if result.satisfied() {
                html.push_str("<div>was satisfied</div>");
            } else {
                html.push_str("<div>was not satisfied</div>");
            }

            Self::supported_by(html, result);

            if let Some(trace) = result.trace() {
                html.push_str(
                    format!(
                        "<div>Evaluation time: {} ns</div>",
                        trace.duration.as_nanos()
                    )
                    .as_str(),
                );
            }
        } else {
            html.push_str("No input provided");
        }

        html.push_str("</div>");
    }

    pub fn supported_by(html: &mut String, result: &EvaluationResult) {
        match result.rationale() {
            Rationale::Anything => {
                html.push_str("<div>anything is satisfied by anything</div>");
            }
            Rationale::Nothing => {}
            Rationale::Object(fields) => {
                html.push_str("<div class='object'>");
                if result.rationale().satisfied() {
                    html.push_str("<div class='reason'>because all fields were satisfied:</div>");
                } else {
                    html.push_str(
                        "<div class='reason'>because not all fields were satisfied:</div>",
                    );
                }
                for (name, result) in fields {
                    if let Rationale::MissingField(_) = result.rationale() {
                        html.push_str("<div class='field unsatisfied'>");
                        html.push_str(format!("field <code>{name}</code> is missing").as_str());
                        html.push_str("</div>");
                    } else {
                        if result.satisfied() {
                            html.push_str("<div class='field satisfied'>");
                        } else {
                            html.push_str("<div class='field unsatisfied'>");
                        }
                        html.push_str("<div class='field-name'>field <code>");
                        html.push_str(name.as_str());
                        html.push_str("</code></div>");
                        Self::rationale_inner(html, result);
                        html.push_str("</div>");
                    }
                }
                html.push_str("</div>");
            }
            Rationale::List(terms) => {
                html.push_str("<div class='list'>");
                if result.rationale().satisfied() {
                    html.push_str("<div class='reason'>because all members were satisfied:</div>");
                } else {
                    html.push_str(
                        "<div class='reason'>because not all members were satisfied:</div>",
                    );
                }
                for element in terms {
                    if result.satisfied() {
                        html.push_str("<div class='element satisfied'>");
                    } else {
                        html.push_str("<div class='element unsatisfied'>");
                    }
                    Self::rationale_inner(html, element);
                    html.push_str("</div>");
                }
                html.push_str("</div>");
            }
            Rationale::Chain(terms) => {
                html.push_str("<div class='chain'>");
                if result.rationale().satisfied() {
                    html.push_str("<div class='reason'>because the chain was satisfied:</div>");
                } else {
                    html.push_str("<div class='reason'>because the chain was not satisfied:</div>");
                }
                for element in terms {
                    if result.satisfied() {
                        html.push_str("<div class='element satisfied'>");
                    } else {
                        html.push_str("<div class='element unsatisfied'>");
                    }
                    Self::rationale_inner(html, element);
                    html.push_str("</div>");
                }
                html.push_str("</div>");
            }
            Rationale::NotAnObject => {
                html.push_str("<div>not an object</div>");
            }
            Rationale::NotAList => {
                html.push_str("<div>not a list</div>");
            }
            Rationale::MissingField(name) => {
                html.push_str(format!("<div>missing field: {name}</div>").as_str());
            }
            Rationale::InvalidArgument(name) => {
                html.push_str(format!("<div>invalid argument: {name}</div>").as_str());
            }
            Rationale::Const(_) => {}
            Rationale::Primordial(_) => {}
            Rationale::Expression(_) => {}
            Rationale::Function(val, rationale, supporting) => {
                if *val {
                    match result.raw_output() {
                        Output::None => {
                            todo!("should not get here")
                        }
                        Output::Identity => {
                            //html.push_str("<div class='function'>");
                            //html.push_str( "function was satisfied");
                            //html.push_str("</div>");
                        }
                        Output::Transform(output) => {
                            html.push_str("<div class='function'>");
                            html.push_str("and produced a value");

                            let output_json = output.as_json();
                            let output_json = serde_json::to_string_pretty(&output_json).unwrap();
                            let output_json = output_json.replace('<', "&lt;");
                            let output_json = output_json.replace('>', "&gt;");
                            html.push_str("<div class='output'>");
                            html.push_str("<pre>");
                            html.push_str(output_json.as_str());
                            html.push_str("</pre>");
                            html.push_str("</div>");

                            html.push_str("</div>");
                        }
                    }
                    if !supporting.is_empty() {
                        for e in supporting {
                            Self::rationale_inner(html, e);
                        }
                    }
                }
            }
            Rationale::Refinement(primary, refinement) => {
                Self::rationale_inner(html, primary);
                if let Some(refinement) = refinement {
                    Self::rationale_inner(html, refinement);
                }
            }
        }
    }
}

use seedwing_policy_engine::lang::lir::{InnerType, ObjectType, Type, ValueType};
use seedwing_policy_engine::lang::{lir, PrimordialType, TypeName};
use std::sync::Arc;

#[allow(dead_code)]
pub struct Htmlifier<'w> {
    root: String,
    world: &'w lir::World,
}

impl<'w> Htmlifier<'w> {
    pub fn new(root: String, world: &'w lir::World) -> Self {
        Self { root, world }
    }

    pub fn html_of(&self, ty: Arc<Type>) -> String {
        let mut html = String::new();
        self.html_of_ty_inner(&mut html, ty);
        html
    }

    fn a(&self, html: &mut String, name: TypeName) {
        let href = name.as_type_str().replace("::", "/");
        let href = format!("{}{}", self.root, href);
        html.push_str(format!("<a href='{}'>{}</a>", href, name).as_str());
    }

    fn html_of_ty(&self, html: &mut String, ty: Arc<Type>) {
        if let Some(name) = ty.name() {
            self.a(html, name);
        } else {
            self.html_of_ty_inner(html, ty);
        }
    }

    fn html_of_ty_inner(&self, html: &mut String, ty: Arc<Type>) {
        match ty.inner() {
            InnerType::Anything => {
                html.push_str("<span>");
                html.push_str("anything");
                html.push_str("</span>");
            }
            InnerType::Primordial(primordial) => match primordial {
                PrimordialType::Integer => {
                    html.push_str("<span>");
                    html.push_str("integer");
                    html.push_str("</span>");
                }
                PrimordialType::Decimal => {
                    html.push_str("<span>");
                    html.push_str("decimal");
                    html.push_str("</span>");
                }
                PrimordialType::Boolean => {
                    html.push_str("<span>");
                    html.push_str("boolean");
                    html.push_str("</span>");
                }
                PrimordialType::String => {
                    html.push_str("<span>");
                    html.push_str("string");
                    html.push_str("</span>");
                }
                PrimordialType::Function(_type_name, _) => {
                    html.push_str("<span>");
                    html.push_str("built-in function");
                    html.push_str("</span>");
                }
            },
            InnerType::Bound(ty, bindings) => {
                html.push_str("<span>");
                self.html_of_ty(html, ty.clone());
                html.push_str("&lt;");
                for (i, (name, bound)) in bindings.iter().enumerate() {
                    html.push_str(name.as_str());
                    html.push('=');
                    self.html_of_ty(html, bound.clone());
                    if i + 1 < bindings.len() {
                        html.push_str(", ");
                    }
                }
                html.push_str("&gt;");
                html.push_str("</span>");
            }
            InnerType::Argument(arg) => {
                html.push_str(arg.as_str());
            }
            InnerType::Const(val) => match val {
                ValueType::Null => {
                    html.push_str("null");
                }
                ValueType::String(val) => {
                    html.push('"');
                    html.push_str(val.as_str());
                    html.push('"');
                }
                ValueType::Integer(val) => html.push_str(format!("{}", val).as_str()),
                ValueType::Decimal(val) => html.push_str(format!("{}", val).as_str()),
                ValueType::Boolean(val) => html.push_str(format!("{}", val).as_str()),
                ValueType::Object(_val) => {
                    todo!()
                }
                ValueType::List(_val) => {
                    todo!()
                }
                ValueType::Octets(_val) => {
                    todo!()
                }
            },
            InnerType::Object(object) => {
                self.html_of_object(html, object);
            }
            InnerType::Expr(_) => {
                todo!()
            }
            InnerType::Join(lhs, rhs) => {
                html.push_str("<span>");
                self.html_of_ty(html, lhs.clone());
                html.push_str(" || ");
                self.html_of_ty(html, rhs.clone());
                html.push_str("</span>");
            }
            InnerType::Meet(_, _) => {
                todo!()
            }
            InnerType::Refinement(primary, refinement) => {
                html.push_str("<span>");
                self.html_of_ty(html, primary.clone());
                html.push('(');
                self.html_of_ty(html, refinement.clone());
                html.push(')');
                html.push_str("</span>");
            }
            InnerType::List(_) => {
                todo!()
            }
            InnerType::Nothing => {
                html.push_str("<span>");
                html.push_str("nothing");
                html.push_str("</span>");
            }
        }

        //"howdy".into()
    }

    fn html_of_object(&self, html: &mut String, object: &ObjectType) {
        html.push_str("<span>");
        html.push('{');
        for f in object.fields() {
            html.push_str("<div>");
            html.push_str(f.name().as_str());
            html.push_str(": ");
            self.html_of_ty(html, f.ty());
            html.push_str("</div>");
        }
        html.push('}');
        html.push_str("</span>");
    }
}

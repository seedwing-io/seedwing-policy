use seedwing_policy_engine::lang::lir::{Expr, InnerType, ObjectType, Type, ValueType};
use seedwing_policy_engine::lang::{PrimordialType, SyntacticSugar};
use seedwing_policy_engine::runtime;
use seedwing_policy_engine::runtime::{TypeName, World};
use std::sync::Arc;

#[allow(dead_code)]
pub struct Htmlifier<'w> {
    root: String,
    world: &'w runtime::World,
}

impl<'w> Htmlifier<'w> {
    pub fn new(root: String, world: &'w runtime::World) -> Self {
        Self { root, world }
    }

    pub fn html_of(&self, ty: Arc<Type>, world: &World) -> String {
        let mut html = String::new();
        self.html_of_ty_inner(&mut html, ty, world);
        html
    }

    fn a(&self, html: &mut String, name: TypeName) {
        let href = name.as_type_str().replace("::", "/");
        let href = format!("{}{}", self.root, href);
        html.push_str(format!("<a href='{}'>{}</a>", href, name).as_str());
    }

    fn html_of_ty(&self, html: &mut String, ty: Arc<Type>, world: &World) {
        if let Some(name) = ty.name() {
            self.a(html, name);
        } else {
            self.html_of_ty_inner(html, ty, world);
        }
    }

    fn html_of_ty_inner(&self, html: &mut String, ty: Arc<Type>, world: &World) {
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
                PrimordialType::Function(sugar, _type_name, _) => {
                    html.push_str("<span>");
                    html.push_str("built-in function");
                    html.push_str("</span>");
                }
            },
            InnerType::Bound(primary, bindings) => {
                html.push_str("<span>");
                self.html_of_ty(html, primary.clone(), world);
                if !&primary.parameters().is_empty() {
                    html.push_str("&lt;");
                    for (i, param) in primary.parameters().iter().enumerate() {
                        let bound = bindings.get(param);
                        if let Some(bound) = bound {
                            self.html_of_ty(html, bound.clone(), world);
                            if i + 1 < bindings.len() {
                                html.push_str(", ");
                            }
                        } else {
                            html.push_str("missing");
                        }
                    }
                    html.push_str("&gt;");
                }
                html.push_str("</span>");
            }
            InnerType::Ref(sugar, slot, bindings) => match sugar {
                SyntacticSugar::None => {
                    let ty = world.get_by_slot(*slot);
                    if let Some(ty) = ty {
                        html.push_str("<span>");
                        let has_params = !ty.parameters().is_empty();
                        self.html_of_ty(html, ty, world);
                        if has_params {
                            html.push_str("&lt;");
                            for (i, arg) in bindings.iter().enumerate() {
                                self.html_of_ty(html, arg.clone(), world);
                                if i + 1 < bindings.len() {
                                    html.push_str(", ");
                                }
                            }
                            html.push_str("&gt;");
                        }
                        html.push_str("</span>");
                    }
                }
                SyntacticSugar::And => {
                    let terms = &bindings[0];
                    if let InnerType::List(terms) = terms.inner() {
                        html.push_str("<span>");
                        for (i, term) in terms.iter().enumerate() {
                            self.html_of_ty(html, term.clone(), world);
                            if i + 1 < terms.len() {
                                html.push_str(" && ");
                            }
                        }
                        html.push_str("</span>");
                    }
                }
                SyntacticSugar::Or => {
                    let terms = &bindings[0];
                    if let InnerType::List(terms) = terms.inner() {
                        html.push_str("<span>");
                        for (i, term) in terms.iter().enumerate() {
                            self.html_of_ty(html, term.clone(), world);
                            if i + 1 < terms.len() {
                                html.push_str(" || ");
                            }
                        }
                        html.push_str("</span>");
                    }
                }
                SyntacticSugar::Refine => {
                    let refinement = &bindings[0];
                    html.push_str("<span>");
                    html.push('(');
                    self.html_of_ty(html, refinement.clone(), world);
                    html.push(')');
                    html.push_str("</span>");
                }
                SyntacticSugar::Traverse => {
                    let step = &bindings[0];
                    html.push_str("<span>");
                    html.push('.');
                    //self.html_of_ty(html, step.clone(), world);
                    if let InnerType::Const(ValueType::String(step)) = step.inner() {
                        html.push_str(step.as_str())
                    }
                    html.push_str("</span>");
                }
                SyntacticSugar::Chain => {
                    let terms = &bindings[0];
                    if let InnerType::List(terms) = terms.inner() {
                        html.push_str("<span>");
                        //html.push('.');
                        //self.html_of_ty(html, step.clone(), world);
                        //if let InnerType::Const(ValueType::String(step)) = step.inner() {
                        //html.push_str(step.as_str())
                        //}
                        for term in terms {
                            self.html_of_ty(html, term.clone(), world);
                        }
                        html.push_str("</span>");
                    }
                }
            },
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
                ValueType::List(_val) => {
                    todo!()
                }
                ValueType::Octets(_val) => {
                    todo!()
                }
            },
            InnerType::Object(object) => {
                self.html_of_object(html, object, world);
            }
            InnerType::Expr(expr) => {
                html.push_str("$(");
                self.html_of_expr(html, expr);
                html.push(')');
            }
            InnerType::List(terms) => {
                html.push_str("<span>[ ");
                for (i, term) in terms.iter().enumerate() {
                    self.html_of_ty(html, term.clone(), world);
                    if i + 1 < terms.len() {
                        html.push_str(", ");
                    }
                }
                html.push_str(" ]</span>");
            }
            InnerType::Nothing => {
                html.push_str("<span>");
                html.push_str("nothing");
                html.push_str("</span>");
            }
        }
    }

    fn html_of_object(&self, html: &mut String, object: &ObjectType, world: &World) {
        html.push_str("<span>");
        html.push('{');
        for f in object.fields() {
            html.push_str("<div>");
            html.push_str(f.name().as_str());
            if f.optional() {
                html.push('?');
            }
            html.push_str(": ");
            self.html_of_ty(html, f.ty(), world);
            html.push_str("</div>");
        }
        html.push('}');
        html.push_str("</span>");
    }

    fn html_of_expr(&self, html: &mut String, expr: &Arc<Expr>) {
        match &**expr {
            Expr::SelfLiteral() => html.push_str("self"),
            Expr::Value(val) => match val {
                ValueType::Null => {
                    html.push_str("null");
                }
                ValueType::String(inner) => {
                    html.push_str(format!("\"{}\"", inner).as_str());
                }
                ValueType::Integer(inner) => {
                    html.push_str(format!("{}", inner).as_str());
                }
                ValueType::Decimal(inner) => {
                    html.push_str(format!("{}", inner).as_str());
                }
                ValueType::Boolean(inner) => {
                    html.push_str(format!("{}", inner).as_str());
                }
                ValueType::List(_) => {}
                ValueType::Octets(_) => {}
            },
            Expr::Function(_, _) => {}
            Expr::Add(_, _) => {}
            Expr::Subtract(_, _) => {}
            Expr::Multiply(_, _) => {}
            Expr::Divide(_, _) => {}
            Expr::LessThan(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" &lt; ");
                self.html_of_expr(html, rhs);
            }
            Expr::LessThanEqual(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" &lt;= ");
                self.html_of_expr(html, rhs);
            }
            Expr::GreaterThan(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" &gt; ");
                self.html_of_expr(html, rhs);
            }
            Expr::GreaterThanEqual(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" &gt;= ");
                self.html_of_expr(html, rhs);
            }
            Expr::Equal(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" == ");
                self.html_of_expr(html, rhs);
            }
            Expr::NotEqual(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" != ");
                self.html_of_expr(html, rhs);
            }
            Expr::Not(_) => {}
            Expr::LogicalAnd(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" &amp;&amp; ");
                self.html_of_expr(html, rhs);
            }
            Expr::LogicalOr(lhs, rhs) => {
                self.html_of_expr(html, lhs);
                html.push_str(" || ");
                self.html_of_expr(html, rhs);
            }
        }
    }
}

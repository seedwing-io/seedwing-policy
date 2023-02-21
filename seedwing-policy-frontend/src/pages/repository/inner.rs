use crate::pages::AppRoute;
use itertools::intersperse;
use seedwing_policy_engine::api::{
    Field, InnerTypeInformation, ObjectType, PrimordialType, TypeInformation, TypeOrReference,
    TypeRef,
};
use seedwing_policy_engine::lang::{
    lir::{Expr, ValueType},
    SyntacticSugar,
};
use std::rc::Rc;
use yew::prelude::*;
use yew_nested_router::components::Link;

#[derive(PartialEq, Properties)]
pub struct Props {
    pub r#type: Rc<TypeInformation>,
}

#[function_component(Inner)]
pub fn inner(props: &Props) -> Html {
    html!(
        <>
            <div class="sw-in-inner">
                { render_inner(&props.r#type.inner) }
            </div>
        </>
    )
}

fn render_inner(info: &InnerTypeInformation) -> Html {
    match info {
        InnerTypeInformation::Anything => "anything".into(),
        InnerTypeInformation::Primordial(primordial) => render_primordial(primordial),
        InnerTypeInformation::Argument(arg) => html!(<>{arg}</>),
        InnerTypeInformation::Deref(inner) => html!(<span>{"*"} {render_type(inner)}</span>),
        InnerTypeInformation::Const(val) => render_val(val),
        InnerTypeInformation::Bound(primary, bindings) => {
            render_ty_and_bindings(primary, bindings.bindings.values())
        }
        InnerTypeInformation::Ref(sugar, ty, bindings) => render_ref(sugar, ty, bindings),
        InnerTypeInformation::Object(object) => render_object(object),
        InnerTypeInformation::List(terms) => render_list(terms),
        InnerTypeInformation::Expr(expr) => {
            html!( <> { "$(" } { render_expr(expr) } { ")" } </> )
        }
        InnerTypeInformation::Nothing => "nothing".into(),
    }
}

fn render_expr(expr: &Expr) -> Html {
    match expr {
        Expr::SelfLiteral() => "self".into(),
        Expr::Value(val) => render_val(val),
        Expr::Function(name, expr) => html!( <> {name} {"("} { render_expr(expr) } {")"} </> ),
        Expr::Add(lhs, rhs) => render_expr_2(lhs, " + ", rhs),
        Expr::Subtract(lhs, rhs) => render_expr_2(lhs, " - ", rhs),
        Expr::Multiply(lhs, rhs) => render_expr_2(lhs, " * ", rhs),
        Expr::Divide(lhs, rhs) => render_expr_2(lhs, " / ", rhs),
        Expr::LessThan(lhs, rhs) => render_expr_2(lhs, "<", rhs),
        Expr::LessThanEqual(lhs, rhs) => render_expr_2(lhs, "<=", rhs),
        Expr::GreaterThan(lhs, rhs) => render_expr_2(lhs, ">", rhs),
        Expr::GreaterThanEqual(lhs, rhs) => render_expr_2(lhs, ">=", rhs),
        Expr::Equal(lhs, rhs) => render_expr_2(lhs, "==", rhs),
        Expr::NotEqual(lhs, rhs) => render_expr_2(lhs, "!=", rhs),
        Expr::Not(expr) => html!( <> {"!"} { render_expr(expr) } </> ),
        Expr::LogicalAnd(lhs, rhs) => render_expr_2(lhs, "&&", rhs),
        Expr::LogicalOr(lhs, rhs) => render_expr_2(lhs, "||", rhs),
    }
}

fn render_expr_2(lhs: &Expr, op: &str, rhs: &Expr) -> Html {
    html!( <> { render_expr(lhs) } { " " } { op } { " " } { render_expr(rhs) } </> )
}

fn render_val(val: &ValueType) -> Html {
    html!(
        <span class="sw-in-value"> {
            match val {
                ValueType::Null => "null".into(),
                ValueType::String(val) => {
                    html!(<>
                        {"\""} { val.replace("\\","\\\\").replace("\"", "\\\"") } {"\""}
                    </>)
                }
                ValueType::Integer(val) => val.into(),
                ValueType::Decimal(val) => val.into(),
                ValueType::Boolean(val) => val.into(),
                ValueType::List(val) => html!(<>
                    { for intersperse(val.iter().map(|v| render_val(&v)), html!(", ")) }
                </>),
                ValueType::Octets(val) => html!(<span>{ val.len() } { "octets"}</span>),
            }
        } </span>
    )
}

fn render_list(terms: &Vec<TypeOrReference>) -> Html {
    html!(
        <span>
        { "[" }
        { for terms.iter().map(render_type) }
        { "]" }
        </span>
    )
}

fn render_ty_and_bindings<'a, I>(ty: &TypeOrReference, bindings: I) -> Html
where
    I: Iterator<Item = &'a TypeOrReference>,
{
    let bindings: Vec<_> = bindings.map(|v| render_type(v)).collect();

    html!(
        <>
            {render_type(ty)}
            if !bindings.is_empty() {
                {"<"} { bindings } {">"}
            }
        </>
    )
}

fn render_ref(
    sugar: &SyntacticSugar,
    ty: &TypeOrReference,
    bindings: &Vec<TypeOrReference>,
) -> Html {
    match sugar {
        SyntacticSugar::None => render_ty_and_bindings(ty, bindings.iter()),
        SyntacticSugar::Or => {
            html!(if let Some(terms) = terms(bindings) {
                { for intersperse(terms.iter().map(|t|render_type(t)), html!(" || ")) }
            })
        }
        SyntacticSugar::And => {
            html!(if let Some(terms) = terms(bindings) {
                { for intersperse(terms.iter().map(|t|render_type(t)), html!(" && ")) }
            })
        }
        #[rustfmt::skip]
        SyntacticSugar::Refine => {
            html!(if let Some(refinement) = bindings.first() {
                {"("}
                {render_type(&refinement)}
                {")"}
            })
        }
        #[rustfmt::skip]
        SyntacticSugar::Traverse => {
            html!(if let Some(InnerTypeInformation::Const(ValueType::String(step))) = inner(bindings) {
                {"."} { step }
            })
        }
        SyntacticSugar::Chain => {
            html!(if let Some(InnerTypeInformation::List(terms)) = inner(bindings) {
                { for terms.iter().map(render_type) }
            })
        }
        SyntacticSugar::Not => {
            html!(if let Some(InnerTypeInformation::List(terms)) = inner(bindings) {
                { "!" }{ for terms.iter().map(render_type) }
            })
        }
    }
}

fn inner(bindings: &Vec<TypeOrReference>) -> Option<&InnerTypeInformation> {
    if let Some(TypeOrReference::Type(rc)) = bindings.first() {
        Some(&rc)
    } else {
        None
    }
}

fn terms(bindings: &Vec<TypeOrReference>) -> Option<&Vec<TypeOrReference>> {
    if let Some(InnerTypeInformation::List(terms)) = inner(bindings) {
        Some(terms)
    } else {
        None
    }
}

fn render_object(object: &ObjectType) -> Html {
    let last = object.fields.len() - 1;

    html!(
        <span class="sw-in-object">
            {"{"}
                { for object.fields.iter()
                    .enumerate()
                    .map(|(n, f)| render_field(f, n == last)) }
            {"}"}
        </span>
    )
}

fn render_field(field: &Field, last: bool) -> Html {
    html!(
        <span class="sw-in-field">
            <span class="sw-in-field__name">{ &field.name }</span>
            if field.optional { { "?" } }
            { ": " }
            <span class="sw-in-field__type">
                { render_type(&field.ty ) }
            </span>
            if !last { { "," } }
        </span>
    )
}

fn render_primordial(primordial: &PrimordialType) -> Html {
    match primordial {
        PrimordialType::Integer => "integer".into(),
        PrimordialType::Decimal => "decimal".into(),
        PrimordialType::Boolean => "boolean".into(),
        PrimordialType::String => "string".into(),
        PrimordialType::Function(_sugar, _type) => "built-in function".into(),
    }
}

fn render_type(ty: &TypeOrReference) -> Html {
    match ty {
        TypeOrReference::Type(inner) => render_inner(&inner),
        TypeOrReference::Ref(reference) => render_type_ref(reference),
    }
}

fn render_type_ref(r: &TypeRef) -> Html {
    let name = r
        .package
        .iter()
        .map(|s| s.as_str())
        .chain(vec![r.name.as_str()])
        .collect::<Vec<_>>()
        .join("::");

    html!(
        <span class="sw-in-type-ref">
        <Link<AppRoute> target={AppRoute::Repository {path: {name.clone()}}}>
            { name }
        </Link<AppRoute>>
        </span>
    )
}

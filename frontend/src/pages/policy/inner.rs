use crate::pages::AppRoute;
use itertools::intersperse;
use seedwing_policy_engine::api::{
    Field, InnerPatternInformation, ObjectPattern, PrimordialPattern, PatternInformation, PatternOrReference,
    PatternRef,
};
use seedwing_policy_engine::lang::{
    lir::{Expr, ValuePattern},
    SyntacticSugar,
};
use std::rc::Rc;
use yew::prelude::*;
use yew_nested_router::components::Link;

#[derive(PartialEq, Properties)]
pub struct Props {
    pub r#type: Rc<PatternInformation>,
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

fn render_inner(info: &InnerPatternInformation) -> Html {
    match info {
        InnerPatternInformation::Anything => "anything".into(),
        InnerPatternInformation::Primordial(primordial) => render_primordial(primordial),
        InnerPatternInformation::Argument(arg) => html!(<>{arg}</>),
        InnerPatternInformation::Deref(inner) => html!(<span>{"*"} {render_type(inner)}</span>),
        InnerPatternInformation::Const(val) => render_val(val),
        InnerPatternInformation::Bound(primary, bindings) => {
            render_ty_and_bindings(primary, bindings.bindings.values())
        }
        InnerPatternInformation::Ref(sugar, ty, bindings) => render_ref(sugar, ty, bindings),
        InnerPatternInformation::Object(object) => render_object(object),
        InnerPatternInformation::List(terms) => render_list(terms),
        InnerPatternInformation::Expr(expr) => {
            html!( <> { "$(" } { render_expr(expr) } { ")" } </> )
        }
        InnerPatternInformation::Nothing => "nothing".into(),
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

fn render_val(val: &ValuePattern) -> Html {
    html!(
        <span class="sw-in-value"> {
            match val {
                ValuePattern::Null => "null".into(),
                ValuePattern::String(val) => {
                    html!(<>
                        {"\""} { val.replace("\\","\\\\").replace("\"", "\\\"") } {"\""}
                    </>)
                }
                ValuePattern::Integer(val) => val.into(),
                ValuePattern::Decimal(val) => val.into(),
                ValuePattern::Boolean(val) => val.into(),
                ValuePattern::List(val) => html!(<>
                    { for intersperse(val.iter().map(|v| render_val(&v)), html!(", ")) }
                </>),
                ValuePattern::Octets(val) => html!(<span>{ val.len() } { "octets"}</span>),
            }
        } </span>
    )
}

fn render_list(terms: &Vec<PatternOrReference>) -> Html {
    html!(
        <span>
        { "[" }
        { for terms.iter().map(render_type) }
        { "]" }
        </span>
    )
}

fn render_ty_and_bindings<'a, I>(ty: &PatternOrReference, bindings: I) -> Html
where
    I: Iterator<Item = &'a PatternOrReference>,
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
    ty: &PatternOrReference,
    bindings: &Vec<PatternOrReference>,
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
            html!(if let Some(InnerPatternInformation::Const(ValuePattern::String(step))) = inner(bindings) {
                {"."} { step }
            })
        }
        SyntacticSugar::Chain => {
            html!(if let Some(InnerPatternInformation::List(terms)) = inner(bindings) {
                { for terms.iter().map(render_type) }
            })
        }
        SyntacticSugar::Not => {
            html!(if let Some(InnerPatternInformation::List(terms)) = inner(bindings) {
                { "!" }{ for terms.iter().map(render_type) }
            })
        }
    }
}

fn inner(bindings: &Vec<PatternOrReference>) -> Option<&InnerPatternInformation> {
    if let Some(PatternOrReference::Pattern(rc)) = bindings.first() {
        Some(&rc)
    } else {
        None
    }
}

fn terms(bindings: &Vec<PatternOrReference>) -> Option<&Vec<PatternOrReference>> {
    if let Some(InnerPatternInformation::List(terms)) = inner(bindings) {
        Some(terms)
    } else {
        None
    }
}

fn render_object(object: &ObjectPattern) -> Html {
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

fn render_primordial(primordial: &PrimordialPattern) -> Html {
    match primordial {
        PrimordialPattern::Integer => "integer".into(),
        PrimordialPattern::Decimal => "decimal".into(),
        PrimordialPattern::Boolean => "boolean".into(),
        PrimordialPattern::String => "string".into(),
        PrimordialPattern::Function(_sugar, _type) => "built-in function".into(),
    }
}

fn render_type(ty: &PatternOrReference) -> Html {
    match ty {
        PatternOrReference::Pattern(inner) => render_inner(&inner),
        PatternOrReference::Ref(reference) => render_type_ref(reference),
    }
}

fn render_type_ref(r: &PatternRef) -> Html {
    let name = r
        .package
        .iter()
        .map(|s| s.as_str())
        .chain(vec![r.name.as_str()])
        .collect::<Vec<_>>()
        .join("::");

    html!(
        <span class="sw-in-type-ref">
        <Link<AppRoute> target={AppRoute::Policy {path: {name.clone()}}}>
            { name }
        </Link<AppRoute>>
        </span>
    )
}

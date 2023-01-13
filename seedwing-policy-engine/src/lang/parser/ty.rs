//use crate::lang::expr::{expr, Expr, field_expr, Value};
use crate::lang::hir::{Field, MemberQualifier, ObjectType, Type, TypeDefn};
use crate::lang::lir::ValueType;
use crate::lang::parser::expr::{expr, Expr};
use crate::lang::parser::literal::{
    anything_literal, decimal_literal, integer_literal, string_literal,
};
use crate::lang::parser::{
    op, use_statement, CompilationUnit, Located, Location, ParserError, ParserInput,
    SourceLocation, SourceSpan, Use,
};
use crate::runtime::{PackageName, PackagePath, TypeName};
use crate::value::RuntimeValue;
use chumsky::prelude::*;
use chumsky::Parser;
use std::fmt::{Debug, Display, Formatter};
use std::iter;
use std::iter::once;
use std::ops::Deref;

pub fn path_segment() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone {
    filter(|c: &char| (c.is_alphanumeric()) || *c == '@' || *c == '_' || *c == '-')
        .repeated()
        .collect()
        .padded()
        .map_with_span(Located::new)
}

pub fn simple_type_name() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone
{
    path_segment()
}

pub fn type_name() -> impl Parser<ParserInput, Located<TypeName>, Error = ParserError> + Clone {
    just("::")
        .padded()
        .ignored()
        .or_not()
        .then(
            simple_type_name()
                .separated_by(just("::"))
                .at_least(1)
                .allow_leading(),
        )
        .map_with_span(|(absolute, mut segments), span| {
            let tail = segments.pop().unwrap();

            let package = if segments.is_empty() {
                None
            } else {
                Some(PackagePath::from(
                    segments
                        .iter()
                        .map(|e| Located::new(PackageName::new(e.inner()), e.location()))
                        .collect::<Vec<Located<PackageName>>>(),
                ))
            };

            Located::new(TypeName::new(package, tail.inner()), span)
        })
}

pub fn type_parameters(
) -> impl Parser<ParserInput, Vec<Located<String>>, Error = ParserError> + Clone {
    just("<")
        .padded()
        .ignored()
        .then(
            text::ident()
                .map_with_span(Located::new)
                .separated_by(just(",").padded())
                .allow_trailing(),
        )
        .then(just(">").padded().ignored())
        .map(|((_, names), _)| names)
}

pub fn inner_type_definition(
    params: &Option<Vec<Located<String>>>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("=")
        .padded()
        .ignored()
        .then({
            let visible_parameters: Vec<String> = match params {
                Some(params) => params.iter().map(|e| e.inner()).collect(),
                None => Vec::new(),
            };
            type_expr(visible_parameters)
        })
        .map(|(_, x)| x)
}

pub fn doc_comment_line() -> impl Parser<ParserInput, String, Error = ParserError> + Clone {
    just("///").then(take_until(just("\n"))).map(|v| {
        let (_, (doc, _eol)) = v;
        let mut line = String::new();
        line.extend(doc);
        line
    })
}

pub fn doc_comment() -> impl Parser<ParserInput, String, Error = ParserError> + Clone {
    doc_comment_line().repeated().map(|v| {
        let mut docs = String::new();
        let mut prefix = None;
        for line in v {
            if prefix.is_none() {
                let len_before_trim = line.len();
                let line = line.trim_start();
                let len_after_trim = line.len();
                let prefix_len = len_before_trim - len_after_trim;
                prefix.replace((" ").repeat(prefix_len));
            }

            if let Some(line) = line.strip_prefix(prefix.as_ref().unwrap()) {
                docs.push_str(line.trim_start());
            } else {
                docs.push_str(line.trim_start());
            }

            docs.push('\n');
        }
        let docs = docs.trim().into();
        docs
    })
}

pub fn type_definition() -> impl Parser<ParserInput, Located<TypeDefn>, Error = ParserError> + Clone
{
    doc_comment()
        .or_not()
        .then(
            just("pattern")
                .padded()
                .ignored()
                .then(simple_type_name())
                .then(type_parameters().or_not())
                .then_with(move |((_, ty_name), params)| {
                    inner_type_definition(&params)
                        .or_not()
                        .map(move |ty| (ty_name.clone(), params.clone(), ty))
                })
                .map(|(ty_name, params, ty)| {
                    let ty = ty.unwrap_or({
                        let loc = ty_name.location();
                        Located::new(Type::Nothing, loc)
                    });

                    let loc = ty_name.span().start()..ty.span().end();
                    Located::new(TypeDefn::new(ty_name, ty, params.unwrap_or_default()), loc)
                }),
        )
        .map(|(doc, mut defn)| {
            defn.set_documentation(doc);
            defn
        })
}

pub fn type_expr(
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    recursive(|expr| {
        parenthesized_expr(expr.clone()).or(logical_or(expr, visible_parameters.clone()))
    })
}

pub fn simple_u32() -> impl Parser<ParserInput, Located<u32>, Error = ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .padded()
        .map_with_span(|s: String, span| Located::new(s.parse::<u32>().unwrap(), span))
}

pub fn member_qualifier(
) -> impl Parser<ParserInput, Located<MemberQualifier>, Error = ParserError> + Clone {
    just("any")
        .padded()
        .ignored()
        .map_with_span(|_, span| Located::new(MemberQualifier::Any, span))
        .or(just("none")
            .padded()
            .ignored()
            .map_with_span(|_, span| Located::new(MemberQualifier::None, span)))
        .or(just("all")
            .padded()
            .ignored()
            .map_with_span(|_, span| Located::new(MemberQualifier::All, span)))
        .or(just("n<")
            .padded()
            .ignored()
            .then(simple_u32().padded())
            .then(just(">").padded().ignored())
            .map_with_span(|((_, n), _), span| Located::new(MemberQualifier::N(n), span)))
        .then(just("::").padded().ignored())
        .map(|(qualifier, _)| qualifier)
}

pub fn parenthesized_expr(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("(")
        .padded()
        .ignored()
        .then(expr)
        .then(just(")").padded().ignored())
        .map(|((_left_paren, expr), _right_paren)| expr)
}

pub fn logical_or(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    logical_and(expr.clone(), visible_parameters.clone())
        .then(
            op("||")
                .then(logical_and(expr, visible_parameters))
                .repeated(),
        )
        .map_with_span(|(first, rest), span| {
            if rest.is_empty() {
                first
            } else {
                Located::new(
                    Type::Join(
                        once(first)
                            .chain(rest.iter().map(|e| e.1.clone()))
                            .collect(),
                    ),
                    span,
                )
            }
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    ty(expr.clone(), visible_parameters)
        .then(op("&&").then(expr).repeated())
        .map_with_span(|(first, rest), span| {
            if rest.is_empty() {
                first
            } else {
                Located::new(
                    Type::Meet(
                        once(first)
                            .chain(rest.iter().map(|e| e.1.clone()))
                            .collect(),
                    ),
                    span,
                )
            }
        })
}

pub fn const_type() -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    decimal_literal()
        .or(integer_literal())
        .or(string_literal())
        .map(|v| {
            let location = v.location();
            Located::new(Type::Const(v), location)
        })
}

pub fn expr_ty() -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("$(")
        .padded()
        .ignored()
        .then(expr())
        .then(just(")").padded().ignored())
        .map_with_span(|((_, expr), y), span| Located::new(Type::Expr(expr), span))
}

pub fn refinement(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("(")
        .padded()
        .ignored()
        .then(expr.or_not())
        .then(just(")").padded().ignored())
        .map_with_span(|((_, ty), _), span| {
            if let Some(ty) = ty {
                let loc = ty.location();
                Located::new(ty.inner(), loc)
            } else {
                Located::new(Type::Anything, span)
            }
        })
}

pub fn qualified_list(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    member_qualifier()
        .then(expr)
        .map_with_span(|(qualifier, ty), span| {
            match &*qualifier {
                MemberQualifier::All => Located::new(
                    Type::Ref(
                        Located::new(String::from("list::All").into(), span.clone()),
                        vec![ty],
                    ),
                    span,
                ),
                MemberQualifier::Any => Located::new(
                    Type::Ref(
                        Located::new(String::from("list::Any").into(), span.clone()),
                        vec![ty],
                    ),
                    span,
                ),
                MemberQualifier::None => Located::new(
                    Type::Ref(
                        Located::new(String::from("list::None").into(), span.clone()),
                        vec![ty],
                    ),
                    span,
                ),
                MemberQualifier::N(count) => {
                    let count_loc = count.location();
                    let count = Located::new(
                        Type::Const(Located::new(
                            ValueType::Integer(count.inner() as _),
                            count_loc,
                        )),
                        span.clone(),
                    );
                    Located::new(
                        Type::Ref(
                            Located::new(String::from("list::Some").into(), span.clone()),
                            vec![count, ty],
                        ),
                        span,
                    )
                }
            }
            //Located::new(Type::MemberQualifier(qualifier, Box::new(ty)), span)
        })
}

pub fn list_ty(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    qualified_list(expr.clone()).or(list_literal(expr))
}

pub fn list_literal(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("[")
        .padded()
        .ignored()
        .then(
            expr.separated_by(just(",").padded().ignored())
                .allow_trailing(),
        )
        .then(just("]").padded().ignored())
        .map_with_span(|((_, ty), _), span| Located::new(Type::List(ty), span))
}

pub fn ty(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    expr_ty()
        //.or( parser_function())
        .or(anything_literal())
        .or(list_ty(expr.clone()))
        .or(const_type())
        .or(object_type(expr.clone()))
        .or(type_ref(expr.clone(), visible_parameters))
        .then(refinement(expr).or_not())
        .map_with_span(|(ty, refinement), span| {
            if let Some(refinement) = refinement {
                Located::new(Type::Refinement(Box::new(ty), Box::new(refinement)), span)
            } else {
                ty
            }
        })
}

pub fn type_arguments(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Vec<Located<Type>>, Error = ParserError> + Clone {
    just("<")
        .padded()
        .ignored()
        .then(expr.separated_by(just(",").padded().ignored()))
        .then(just(">").padded().ignored())
        .map(|((_, arguments), _)| arguments)
}

pub fn type_ref(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    type_name()
        .then(type_arguments(expr).or_not())
        .validate(move |(name, arguments), span, emit| {
            if visible_parameters.contains(&name.name())
                && !arguments.clone().unwrap_or_default().is_empty()
            {
                emit(ParserError::custom(
                    span,
                    "Arguments may not be passed to parameters",
                ));
            }
            (name, arguments, visible_parameters.clone())
        })
        .map_with_span(|(name, arguments, visible_parameters), span| {
            let loc = name.location();
            let arguments = arguments.unwrap_or_default();
            if visible_parameters.contains(&name.name()) {
                Located::new(Type::Parameter(Located::new(name.name(), span)), loc)
            } else {
                Located::new(
                    Type::Ref(Located::new(name.inner(), loc.clone()), arguments),
                    loc,
                )
            }
        })
}

pub fn object_type(
    ty: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("{")
        .padded()
        .map_with_span(|_, span| span)
        .then(
            field_definition(ty)
                .separated_by(just(",").padded().ignored())
                .allow_trailing(),
        )
        .then(just("}").padded().map_with_span(|_, span| span))
        .map(|((start, fields), end)| {
            let loc = start.start()..end.end();
            let mut ty = ObjectType::new();
            for f in fields {
                ty.add_field(f);
            }

            Located::new(Type::Object(ty), loc)
        })
}

pub fn field_name() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone {
    text::ident().map_with_span(Located::new)
}

pub fn field_definition(
    ty: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Field>, Error = ParserError> + Clone {
    field_name()
        .then(just(":").labelled("colon").padded().ignored())
        .then(ty)
        .map(|((name, _), ty)| {
            let loc = name.span().start()..ty.span().end();
            Located::new(Field::new(name, ty), loc)
        })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::parser::compilation_unit;

    #[test]
    fn parse_ty_name() {
        let name = type_name().parse("bob").unwrap().inner();

        assert_eq!(name.name(), "bob");
    }

    #[test]
    fn parse_ty_defn() {
        let ty = type_definition().parse("pattern bob").unwrap().inner();

        assert_eq!(&*ty.name().inner(), "bob");
    }

    /*
    #[test]
    fn parse_ty_ref() {
        let ty_ref = type_ref().parse("bob").unwrap().into_inner();

        println!("{:?}", ty_ref);

        assert!(
            matches!(
                ty_ref,
                Type::Ref(ty_name)
            if ty_name.type_name().name() == "bob")
        );
    }
     */

    #[test]
    fn parse_simple_obj_ty() {
        let ty = type_expr(Default::default())
            .then_ignore(end())
            .parse(
                r#"
            {
                foo: 81,
                bar: 4.2,
            }
        "#,
            )
            .unwrap()
            .inner();

        println!("{:?}", ty);

        assert!(matches!(ty, Type::Object(_)));

        if let Type::Object(ty) = ty {
            assert!(matches!(
                ty.fields().iter().find(|e| e.name().inner() == "foo"),
                Some(_)
            ));
            assert!(matches!(
                ty.fields().iter().find(|e| e.name().inner() == "bar"),
                Some(_)
            ));
        }
    }

    #[test]
    fn parse_nested_obj_ty() {
        let ty = type_expr(Default::default())
            .then_ignore(end())
            .parse(
                r#"
            {
                foo: 23,
                bar: {
                  quux: 14,
                },
                taco: int,
            }
        "#,
            )
            .unwrap()
            .inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_function_transform() {
        let ty = type_expr(Default::default())
            .then_ignore(end())
            .parse(
                r#"
            {
                name: string && Length( $(self + 1 > 13) ),
            }
        "#,
            )
            .unwrap()
            .inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_collections() {
        let ty = type_expr(Default::default())
            .then_ignore(end())
            .parse(
                r#"
            {
                name: [int && $(self == 2)]
            }
        "#,
            )
            .unwrap()
            .inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_compilation_unit() {
        let unit = compilation_unit("my_file.dog")
            .parse(
                r#"
            use foo::bar::bar
            use x::y::z as osi-approved-license

            pattern signed = SHA256()

            pattern bob = {
                foo: int,
                bar: {
                  quux: int
                },
                taco: int,
            }

            pattern jim = int && taco

            pattern unsigned-int = int && $( self >= 0 )

            pattern lily

        "#,
            )
            .unwrap();

        println!("{:?}", unit);
    }
}

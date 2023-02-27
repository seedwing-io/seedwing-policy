//use crate::lang::expr::{expr, Expr, field_expr, Value};
use crate::lang::hir::{Field, MemberQualifier, ObjectPattern, Pattern, PatternDefn};

use crate::lang::parser::expr::expr;
use crate::lang::parser::literal::{
    boolean_literal, decimal_literal, integer_literal, string_literal,
};
use crate::lang::parser::{op, Located, ParserError, ParserInput};
use crate::lang::SyntacticSugar;
use crate::runtime::{PackageName, PackagePath, PatternName};

use chumsky::chain::Chain;
use chumsky::prelude::*;
use chumsky::text::Character;
use chumsky::Parser;

use std::iter::once;

pub fn path_segment() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone {
    filter(|c: &char| (c.is_alphanumeric()) || *c == '@' || *c == '_' || *c == '-')
        .repeated()
        .at_least(1)
        .collect()
        .padded()
        .map_with_span(Located::new)
}

pub fn simple_type_name() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone
{
    path_segment()
}

pub fn type_name() -> impl Parser<ParserInput, Located<PatternName>, Error = ParserError> + Clone {
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
        .map_with_span(|(_absolute, mut segments), span| {
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

            Located::new(PatternName::new(package, tail.inner()), span)
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
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
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
                docs.push_str(line);
            } else {
                docs.push_str(line.trim_start());
            }

            docs.push('\n');
        }
        let docs = docs.trim().into();
        docs
    })
}

pub fn type_definition() -> impl Parser<ParserInput, Located<PatternDefn>, Error = ParserError> + Clone
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
                        Located::new(Pattern::Nothing, loc)
                    });

                    let loc = ty_name.span().start()..ty.span().end();
                    Located::new(PatternDefn::new(ty_name, ty, params.unwrap_or_default()), loc)
                }),
        )
        .map(|(doc, mut defn)| {
            defn.set_documentation(doc);
            defn
        })
}

pub fn type_expr(
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    recursive(|expr| logical_or(expr, visible_parameters.clone()))
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
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    just("(")
        .padded()
        .ignored()
        .then(expr)
        .then(just(")").padded().ignored())
        .map(|((_left_paren, expr), _right_paren)| expr)
}

pub fn logical_or(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
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
                    Pattern::Join(
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
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    ty(expr.clone(), visible_parameters)
        .then(op("&&").then(expr).repeated())
        .map_with_span(|(first, rest), span| {
            if rest.is_empty() {
                first
            } else {
                Located::new(
                    Pattern::Meet(
                        once(first)
                            .chain(rest.iter().map(|e| e.1.clone()))
                            .collect(),
                    ),
                    span,
                )
            }
        })
}

pub fn const_type() -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    decimal_literal()
        .or(integer_literal())
        .or(boolean_literal())
        .or(string_literal())
        .map(|v| {
            let location = v.location();
            Located::new(Pattern::Const(v), location)
        })
}

pub fn expr_ty() -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    just("$(")
        .padded()
        .ignored()
        .then(expr())
        .then(just(")").padded().ignored())
        .map_with_span(|((_, expr), _y), span| Located::new(Pattern::Expr(expr), span))
}

pub enum Postfix {
    Refinement(Option<Located<Pattern>>),
    Traversal(Located<String>),
}

pub fn postfix(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Postfix, Error = ParserError> + Clone {
    refinement(expr.clone()).or(traversal(expr))
}

pub fn refinement(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Postfix, Error = ParserError> + Clone {
    just("(")
        .padded()
        .ignored()
        .then(expr.clone().or_not())
        .then(just(")").padded().ignored())
        .map(move |((_, refinement), _)| Postfix::Refinement(refinement))
        .or(just("|")
            .padded()
            .ignored()
            .then(expr)
            .map(|(_, refinement)| Postfix::Refinement(Some(refinement))))
}

pub fn traversal(
    _expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Postfix, Error = ParserError> + Clone {
    just(".")
        .padded()
        .ignored()
        .then(field_name())
        .map_with_span(move |(_, step), _span| Postfix::Traversal(step))
}

pub fn list_ty(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    list_literal(expr)
}

pub fn list_literal(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    just("[")
        .padded()
        .ignored()
        .then(
            expr.separated_by(just(",").padded().ignored())
                .allow_trailing(),
        )
        .then(just("]").padded().ignored())
        .map_with_span(|((_, ty), _), span| Located::new(Pattern::List(ty), span))
}

pub fn ty(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    just("!")
        .padded()
        .or_not()
        .then(just("*").padded().repeated())
        .then(
            parenthesized_expr(expr.clone())
                .or(expr_ty())
                .or(list_ty(expr.clone()))
                .or(const_type())
                .or(type_ref(expr.clone(), visible_parameters))
                .or(object_type(expr.clone()))
                .then(postfix(expr).repeated()),
        )
        .map_with_span(|((not, deref), (primary, postfix)), span| {
            let mut core = if postfix.is_empty() {
                primary
            } else {
                let mut terms = Vec::new();
                terms.push(primary);

                for each in postfix {
                    match each {
                        Postfix::Refinement(refinement) => {
                            if let Some(refinement) = refinement {
                                terms.push(Located::new(
                                    Pattern::Refinement(Box::new(refinement.clone())),
                                    refinement.location(),
                                ));
                            }
                        }
                        Postfix::Traversal(step) => {
                            terms.push(Located::new(Pattern::Traverse(step.clone()), step.location()));
                        }
                    }
                }

                Located::new(Pattern::Chain(terms), span)
            };

            for _ in deref {
                let loc = core.location();
                core = Located::new(Pattern::Deref(Box::new(core)), loc);
            }

            if not.is_some() {
                Located::new(Pattern::Not(Box::new(core.clone())), core.location())
            } else {
                core
            }
        })
}

pub fn type_arguments(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Vec<Located<Pattern>>, Error = ParserError> + Clone {
    just("<")
        .padded()
        .ignored()
        .then(expr.separated_by(just(",").padded().ignored()))
        .then(just(">").padded().ignored())
        .map(|((_, arguments), _)| arguments)
}

pub fn type_ref(
    expr: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
    type_name()
        .then(type_arguments(expr).or_not())
        .validate(move |(name, arguments), span, emit| {
            if visible_parameters.contains(&name.name().to_string())
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
            let name_s = name.name().to_string();
            if visible_parameters.contains(&name_s) {
                Located::new(Pattern::Parameter(Located::new(name_s, span)), loc)
            } else {
                let type_str = name.as_type_str();
                Located::new(
                    match type_str.as_str() {
                        "anything" | "self" => Pattern::Anything,
                        "nothing" => Pattern::Nothing,
                        _ => Pattern::Ref(
                            SyntacticSugar::None,
                            Located::new(name.inner(), loc.clone()),
                            arguments,
                        ),
                    },
                    loc,
                )
            }
        })
}

pub fn object_type(
    ty: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone {
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
            let mut ty = ObjectPattern::new();
            for f in fields {
                ty.add_field(f);
            }

            Located::new(Pattern::Object(ty), loc)
        })
}

pub fn field_name() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone {
    filter(|c: &char| c.to_char().is_ascii_alphabetic() || c.to_char() == '_' || c.to_char() == '@')
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| {
                c.to_char().is_ascii_alphanumeric() || c.to_char() == '_' || c.to_char() == '-'
            })
            .repeated(),
        )
        .collect()
        .map_with_span(Located::new)
}

pub fn field_definition(
    ty: impl Parser<ParserInput, Located<Pattern>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Field>, Error = ParserError> + Clone {
    field_name()
        .then(just("?").or_not())
        .then(just(":").labelled("colon").padded().ignored())
        .then(ty)
        .map(|(((name, optional), _), ty)| {
            let loc = name.span().start()..ty.span().end();
            Located::new(Field::new(name, ty, optional.is_some()), loc)
        })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::lang::parser::compilation_unit;
    use crate::runtime::sources::Ephemeral;

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

    #[test]
    fn parse_ty_defn_with_traversal() {
        let _ty = type_definition()
            .parse("pattern bob = person.first_name(\"bob\").last_name")
            .unwrap()
            .inner();
    }
    /*
    #[test]
    fn parse_ty_ref() {
        let ty_ref = type_ref().parse("bob").unwrap().into_inner();

        println!("{:?}", ty_ref);

        assert!(
            matches!(
                ty_ref,
                Pattern::Ref(ty_name)
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

        assert!(matches!(ty, Pattern::Object(_)));

        if let Pattern::Object(ty) = ty {
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

        println!("{ty:?}");
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

        println!("{ty:?}");
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

        println!("{ty:?}");
    }

    #[test]
    fn list_trailing_comma() {
        let ty = type_expr(Default::default())
            .then_ignore(end())
            .parse(
                r#"
                [ integer, integer, integer, ]
        "#,
            )
            .unwrap()
            .inner();

        println!("{ty:?}");
        assert!(matches!(ty, Pattern::List(_)));

        if let Pattern::List(inner) = ty {
            assert_eq!(inner.len(), 3);
        }
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

        println!("{unit:?}");
    }

    #[test]
    fn parse_identifiers_including_specials() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern self-foo = integer
            pattern foo = self-foo
            pattern anything = self-foo
            pattern bar = self
        "#,
        );

        let mut builder = Builder::new();

        builder.build(src.iter()).unwrap();
    }

    #[test]
    fn parse_postfix_and_logicals() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern cheese = {
               gouda: {
                 name: anything
               }
            }

            pattern sandwich = {
              ham: {
                name: anything
              }
            }

            pattern mix = cheese.gouda.name("bob") && sandwich.ham.name("terry")
        "#,
        );

        let mut builder = Builder::new();

        builder.build(src.iter()).unwrap();
    }

    #[test]
    fn parse_parentheses() {
        let src = Ephemeral::new(
            "test",
            r#"
            pattern ulf = (true || false) && false || ( false && true )
        "#,
        );

        let mut builder = Builder::new();

        builder.build(src.iter()).unwrap();
    }
}

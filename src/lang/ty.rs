//use crate::lang::expr::{expr, Expr, field_expr, Value};
use crate::lang::{ComparisonOp, DerivationOp, Located, Location, ParserError, ParserInput, Span};
use chumsky::prelude::*;
use chumsky::Parser;
use std::fmt::{Debug, Formatter};
use crate::lang::expr::{Expr, expr};
//use crate::lang::expr::Expr;
//use crate::lang::ty::Type::Meet;

#[derive(Debug)]
pub struct CompilationUnit {
    types: Vec<Located<TypeDefn>>,
}

impl CompilationUnit {
    pub fn new() -> Self {
        Self {
            types: Default::default(),
        }
    }

    pub fn add(&mut self, ty: Located<TypeDefn>) {
        self.types.push(ty)
    }
}

#[derive(Clone, Debug)]
pub struct TypeName(String);

#[derive(Clone, Debug)]
pub struct FunctionName(String);

impl TypeName {
    pub fn new(name: String) -> Self {
        Self(name)
    }

    pub fn name(&self) -> &str {
        self.0.as_str()
    }
}


#[derive(Clone, Debug)]
pub struct TypeDefn {
    name: Located<TypeName>,
    ty: Located<Type>,
}

impl TypeDefn {
    pub fn new(name: Located<TypeName>, ty: Located<Type>) -> Self {
        Self { name, ty }
    }
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Primordial(PrimordialType),
    Ref(Located<TypeName>),
    Const(Located<Value>),
    Object(ObjectType),
    Expr(Located<Expr>),
    Join(Box<Located<Type>>, Box<Located<Type>>),
    Meet(Box<Located<Type>>, Box<Located<Type>>),
    Functional(Located<FunctionName>, Box<Located<Type>>),
    List(Box<Located<Type>>),
    Nothing,
}

#[derive(Debug, Clone)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "Anything"),
            Type::Ref(r) => write!(f, "{:?}", r),
            Type::Primordial(p) => write!(f, "{:?}", p),
            Type::Const(value) => write!(f, "{:?}", value),
            Type::Join(l, r) => write!(f, "Join({:?}, {:?})", l, r),
            Type::Meet(l, r) => write!(f, "Meet({:?}, {:?})", l, r),
            Type::Nothing => write!(f, "Nothing"),
            Type::Object(obj) => write!(f, "{:?}", obj),
            Type::Functional(fn_name, ty) => write!(f, "{:?}({:?})", fn_name, ty),
            Type::List(ty) => write!(f, "[{:?}]", ty),
            Type::Expr(expr) => write!(f, "#({:?})", expr)
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Integer(i64),
    Decimal(f64),
    String(String),
    Boolean(bool),
}


#[derive(Clone, Debug)]
pub struct ObjectType {
    fields: Vec<Located<Field>>,
}

impl ObjectType {
    pub fn new() -> Self {
        Self {
            fields: vec![]
        }
    }

    pub fn add_field(&mut self, field: Located<Field>) -> &Self {
        self.fields.push(field);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    name: Located<String>,
    ty: Located<Type>,
}

impl Field {
    pub fn new(name: Located<String>, ty: Located<Type>) -> Self {
        Self {
            name,
            ty,
        }
    }
}

fn op(op: &str) -> impl Parser<ParserInput, &str, Error=ParserError> + Clone {
    just(op).padded()
}

pub fn type_name() -> impl Parser<ParserInput, Located<TypeName>, Error=ParserError> + Clone {
    filter(|c: &char| {
        (!c.is_uppercase() && c.is_alphanumeric())
            || *c == '_' || *c == '-'
    })
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| {
                c.is_alphanumeric() || *c == '_' || *c == '-'
            }).repeated(),
        )
        .collect()
        .padded()
        .map_with_span(|v, span|
            Located::new(TypeName(v), span)
        )
}

pub fn type_definition() -> impl Parser<ParserInput, Located<TypeDefn>, Error=ParserError> + Clone {
    just("type")
        .padded()
        .ignored()
        .then(
            type_name()
        )
        .then(
            just("=")
                .padded()
                .ignored()
                .then(
                    type_expr()
                )
                .or_not()
        )
        .map(|((_, ty_name), ty)| {
            let ty = ty.unwrap_or({
                let loc = ty_name.location();
                ((), Located::new(Type::Nothing, loc.clone()))
            }).1;

            let loc = ty_name.span().start()..ty.span().end();
            Located::new(
                TypeDefn::new(ty_name, ty),
                loc)
        })
}

pub fn type_expr() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    recursive(|expr| {
        parenthesized_expr(expr.clone())
            .or(
                logical_or(expr)
            )
    })
}

pub fn parenthesized_expr(
    expr: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just("(")
        .padded()
        .ignored()
        .then(expr)
        .then(just(")").padded().ignored())
        .map(|((_left_paren, expr), _right_paren)|
            expr
        )
}

pub fn logical_or(
    expr: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    logical_and(expr.clone())
        .then(op("||").then(expr.clone()).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let location = lhs.span().start()..rhs.span().end();
            Located::new(
                Type::Join(
                    Box::new(lhs),
                    Box::new(rhs)),
                location)
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    ty(expr.clone())
        .then(op("&&").then(expr.clone()).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let location = lhs.span().start()..rhs.span().end();
            Located::new(
                Type::Meet(
                    Box::new(lhs),
                    Box::new(rhs)),
                location)
        })
}


pub fn integer_literal() -> impl Parser<ParserInput, Located<Value>, Error=ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .padded()
        .map_with_span(|s: String, span| Located::new(Value::Integer(s.parse().unwrap()), span))
}

pub fn decimal_literal() -> impl Parser<ParserInput, Located<Value>, Error=ParserError> + Clone {
    text::int(10)
        .then(just('.').then(text::int(10)))
        .padded()
        .map_with_span(
            |(integral, (_dot, decimal)): (String, (char, String)), span| {
                Located::new(
                    Value::Decimal(format!("{}.{}", integral, decimal).parse().unwrap()),
                    span,
                )
            },
        )
}

pub fn string_literal() -> impl Parser<ParserInput, Located<Value>, Error=ParserError> + Clone {
    just('"')
        .ignored()
        .then(
            filter(|c: &char| *c != '"')
                .repeated()
                .collect()
        )
        .then(
            just('"')
                .ignored()
        )
        .padded()
        .map_with_span(|((_, x), _), span: Span| {
            Located::new(
                Value::String(
                    x
                ),
                span.clone(),
            )
        })
}


pub fn const_type() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    decimal_literal()
        .or(
            integer_literal()
        )
        .or(
            string_literal()
        )
        .map(|v| {
            let location = v.location();
            Located::new(
                Type::Const(v),
                location,
            )
        })
}

pub fn expr_ty() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just("$(")
        .padded()
        .ignored()
        .then(expr())
        .then(
            just(")")
                .padded()
                .ignored())
        .map_with_span(|((_, expr), y), span| {
            Located::new(
                Type::Expr(expr),
                span,
            )
        })
}

pub fn function_name() -> impl Parser<ParserInput, Located<FunctionName>, Error=ParserError> + Clone {
    filter(|c: &char| {
        c.is_uppercase() && c.is_alphanumeric()
    })
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| {
                c.is_alphanumeric() || *c == '_' || *c == '-'
            }).repeated(),
        )
        .collect()
        .padded()
        .map_with_span(|v, span|
            Located::new(FunctionName(v), span)
        )
}

pub fn functional_ty(expr: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    function_name()
        .then(
            just("(")
                .padded()
                .ignored()
        )
        .then(expr.clone())
        .then(
            just(")")
                .padded()
                .ignored()
        )
        .map(|(((fn_name, _), ty), _)| {
            let location = fn_name.span().start()..ty.span().end();

            Located::new(
                Type::Functional(fn_name, Box::new(ty)),
                location,
            )
        })
}

pub fn list_ty(expr: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just("[")
        .padded()
        .ignored()
        .then(expr)
        .then(
            just("]")
                .padded()
                .ignored()
        )
        .map_with_span(|((_, ty), _), span| {
            Located::new(
                Type::List(Box::new(ty)),
                span,
            )
        })
}

pub fn ty(expr: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    expr_ty()
        .or(
            list_ty(expr.clone())
        )
        .or(
            functional_ty(expr.clone())
        )
        .or(
            const_type()
        )
        .or(
            object_type(expr.clone())
        )
        .or(
            type_ref()
        )
}

pub fn type_ref() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    type_name()
        .map(|name| {
            let loc = name.location();
            Located::new(
                Type::Ref(
                    Located::new(
                        name.into_inner(),
                        loc.clone())
                ),
                loc,
            )
        })
}

pub fn object_type(ty: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just("{")
        .padded()
        .map_with_span(|_, span| {
            span
        })
        .then(
            field_definition(ty)
                .separated_by(
                    just(",")
                        .padded()
                        .ignored()
                )
                .allow_trailing()
        )
        .then(
            just("}")
                .padded()
                .map_with_span(|_, span| {
                    span
                })
        ).map(|((start, fields), end)| {
        let loc = start.start()..end.end();
        let mut ty = ObjectType::new();
        for f in fields {
            ty.add_field(f);
        }

        Located::new(
            Type::Object(ty),
            loc,
        )
    })
}

pub fn field_name() -> impl Parser<ParserInput, Located<String>, Error=ParserError> + Clone {
    text::ident().map_with_span(|name, span| {
        Located::new(name, span)
    })
}

pub fn field_definition(ty: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Field>, Error=ParserError> + Clone {
    field_name()
        .then(just(":").padded().ignored())
        .then(ty)
        .map(|((name, _), ty)| {
            let loc = name.span().start()..ty.span().end();
            Located::new(
                Field::new(name, ty),
                loc,
            )
        })
}

pub fn compilation_unit() -> impl Parser<ParserInput, CompilationUnit, Error=ParserError> + Clone {
    type_definition().padded().repeated()
        .then_ignore(end())
        .map(|ty| {
            let mut unit = CompilationUnit::new();

            for e in ty {
                unit.add(e)
            }

            unit
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_ty_name() {
        let name = type_name().parse("bob").unwrap().into_inner();

        assert_eq!(name.name(), "bob");
    }

    #[test]
    fn parse_ty_defn() {
        let ty = type_definition().parse("type bob").unwrap().into_inner();

        assert_eq!(ty.name.name(), "bob");
    }

    #[test]
    fn parse_ty_ref() {
        let ty_ref = type_ref().parse("bob").unwrap().into_inner();

        println!("{:?}", ty_ref);

        assert!(
            matches!(
                ty_ref,
                Type::Ref(ty_name)
            if ty_name.name() == "bob")
        );
    }

    #[test]
    fn parse_simple_obj_ty() {
        let ty = type_expr().then_ignore(end()).parse(r#"
            {
                foo: 81,
                bar: 4.2,
            }
        "#).unwrap().into_inner();

        println!("{:?}", ty);

        assert!(
            matches!(
                ty,
                Type::Object(_)
            )
        );

        if let Type::Object(ty) = ty {
            assert!(
                matches!(
                    ty.fields.iter().find(|e| *e.name == "foo"),
                    Some(_)
                )
            );
            assert!(
                matches!(
                    ty.fields.iter().find(|e| *e.name == "bar"),
                    Some(_)
                )
            );
        }
    }

    #[test]
    fn parse_nested_obj_ty() {
        let ty = type_expr().then_ignore(end()).parse(r#"
            {
                foo: 23,
                bar: {
                  quux: 14,
                },
                taco: int,
            }
        "#).unwrap().into_inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_function_transform() {
        let ty = type_expr().then_ignore(end()).parse(r#"
            {
                name: string && Length( $(self + 1 > 13) ),
            }
        "#).unwrap().into_inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_collections() {
        let ty = type_expr().then_ignore(end()).parse(r#"
            {
                name: [int && $(self == 2)]
            }
        "#).unwrap().into_inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_compilation_unit() {
        let unit = compilation_unit().parse(r#"
            type bob = {
                foo: int,
                bar: {
                  quux: int
                },
                taco: int,
            }

            type jim = int && taco

            type unsigned-int = int && $( self >= 0 )

            type lily

        "#).unwrap();

        println!("{:?}", unit);
    }
}

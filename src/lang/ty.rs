use crate::lang::expression::{expr, Expr, field_expr, Value};
use crate::lang::{ComparisonOp, DerivationOp, Located, Location, ParserError, ParserInput};
use chumsky::prelude::*;
use chumsky::Parser;
use chumsky::Span;
use std::fmt::{Debug, Formatter};

#[derive(Clone, Debug)]
pub struct TypeName(String);

impl TypeName {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

#[derive(Clone, Debug)]
pub struct TypeRef {
    name: TypeName,
}

impl TypeRef {
    pub fn new(name: TypeName) -> Self {
        Self { name }
    }
}

#[derive(Clone, Debug)]
pub struct TypeDefn {
    name: TypeName,
    ty: Type,
}

impl TypeDefn {
    pub fn new(name: TypeName, ty: Type) -> Self {
        Self { name, ty }
    }
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Ref(TypeRef),
    Const(Located<Value>),
    Object(ObjectType),
    Constrained(Located<Expr>),
    Join(Box<Located<Type>>, Box<Located<Type>>),
    Meet(Box<Located<Type>>, Box<Located<Type>>),
    Nothing,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "Anything"),
            Type::Ref(r) => write!(f, "{:?}", r),
            Type::Const(value) => write!(f, "{:?}", value),
            Type::Constrained(expr) => write!(f, "{:?}", expr),
            Type::Join(l, r) => write!(f, "Join({:?}, {:?})", l, r),
            Type::Meet(l, r) => write!(f, "Meet({:?}, {:?})", l, r),
            Type::Nothing => write!(f, "Nothing"),
            Type::Object(obj) => write!(f, "{:?}", obj),
        }
    }
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
    name: String,
    expr: Option<Located<Expr>>,
}

impl Field {
    pub fn new(name: String, expr: Located<Expr>) -> Self {
        Self {
            name,
            expr: Some(expr),
        }
    }
}

pub fn ty_name() -> impl Parser<ParserInput, Located<TypeName>, Error=ParserError> + Clone {
    filter(|c: &char| (c.is_ascii_alphabetic() && c.is_uppercase()) || *c == '_')
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_').repeated(),
        )
        .collect()
        .padded()
        .map_with_span(|v, span|
            Located::new(TypeName(v), span)
        )
}

pub fn ty_ref() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    ty_name().map(|name| {
        let hoisted_location = name.location();
        Located::new(Type::Ref(
            TypeRef::new(name.into_inner())
        ), hoisted_location)
    })
}

pub fn super_ty_decl() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just(":").padded().ignored()
        .then(ty())
        .map(|v| {
            v.1
        })
}

pub fn ty_decl() -> impl Parser<ParserInput, Located<TypeDefn>, Error=ParserError> + Clone {
    just("type").padded().map_with_span(|_, span| {
        span
    })
        .then(
            ty_name().padded()
                .then(
                    super_ty_decl().or_not()
                )
                .then(
                    constraint_list()
                )
        )
        .map(|(type_span, ((name, super_ty), defn))| {
            let (defn_name, defn_location) = name.clone().split();
            match (super_ty, defn) {
                (None, None) => {
                    println!("A");
                    Located::new(
                        TypeDefn::new(defn_name, Type::Anything),
                        type_span.start()..defn_location.span().end(),
                    )
                }
                (Some(super_ty), None) => {
                    println!("B");
                    let (super_ty, super_location) = super_ty.split();
                    Located::new(
                        TypeDefn::new(defn_name, super_ty),
                        type_span.start()..super_location.span().end())
                }
                (None, Some(defn)) => {
                    println!("C");
                    let (defn_ty, defn_location) = defn.clone().split();

                    let defn_ty = Type::Constrained(defn);

                    Located::new(
                        TypeDefn::new(defn_name, defn_ty),
                        type_span.start()..defn_location.span().end())
                }
                (Some(super_ty), Some(defn)) => {
                    println!("D");
                    let span = type_span.start()..defn_location.span().end();
                    let defn = Located::new( Type::Constrained(defn.clone()), defn.location());
                    Located::new(
                        TypeDefn::new(defn_name,
                                      Type::Meet(
                                          Box::new(super_ty),
                                          Box::new(defn)),
                        ), span)
                }
            }
        }).then_ignore(end())
}

pub fn constraint_list() -> impl Parser<ParserInput, Option<Located<Expr>>, Error=ParserError> + Clone {
    just("{").padded().ignored()
        .map(|v| {
            println!("saw open curly");
            v
        })
        .then(
            expr().or(field_expr())
                .repeated()
                .separated_by(
                    just(",")
                        .padded()
                        .ignored()
                )
                .allow_trailing()
                .padded()
        )
        .then(
            just("}")
                .padded()
                .ignored()
        )
        .map(|v| {
            println!("close curly");
            v
        })
        .map_with_span(|((_left_curly, exprs), _right_curly), span| {
            let expr = exprs.iter().flatten().cloned().reduce(|accum, each| {
                let span = accum.span().start()..each.span().end();
                Located::new(
                    Expr::LogicalAnd(
                        Box::new(accum),
                        Box::new(each)),
                    span)
            });

            println!("---> {:?}", expr);

            if let Some(expr) = expr {
                Some(expr)
            } else {
                None
            }
        })
}

pub fn constrained_ty() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just("->").padded().ignored()
        .then(
            constraint_list()
                .map_with_span(|v, span| {
                    Located::new(
                        v.map_or(Type::Anything, |v| Type::Constrained(v),
                        ), span)
                })
        ).map(|(_arrow, ty)| {
        ty
    })
}


pub fn ty() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    ty_ref()
        .then(
            constraint_list().or_not()
        ).then_ignore(end())
        .map(|(ty, constraints)| {
            if let Some(Some(constraints)) = constraints {
                let span = ty.span().start()..constraints.span().end();
                let hoisted_location = constraints.location();
                Located::new(
                    Type::Meet(
                        Box::new(ty),
                        Box::new(
                            Located::new(Type::Constrained(constraints), hoisted_location)
                        )),
                    span)
            } else {
                ty
            }
        })
}


pub fn expr_ty() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    expr().then(just(";").padded().ignored())
        .map_with_span(|(expr, _semi), span| {
            //(Type::Expr(expr.0), expr.1)
            expr.evaluate_to_type().unwrap()
        })
}

pub fn object_ty() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    just("{").padded().ignored().map_with_span(|_, span| span)
        .then(field().separated_by(just(",").padded().ignored()).allow_trailing())
        .then(just("}").padded().ignored().map_with_span(|_, span| span))
        .map(|((open_curly, fields), close_curly)| {
            let mut object_ty = ObjectType::new();
            for f in fields {
                object_ty.add_field(f);
            }
            Located::new(
                Type::Object(
                    object_ty
                ),
                open_curly.start()..close_curly.end(),
            )
        })
}

fn field() -> impl Parser<ParserInput, Located<Field>, Error=ParserError> + Clone {
    text::ident().padded().map_with_span(|v, span| (v, span))
        .then(just(":").padded().ignored())
        .then(expr())
        .map(|(((name, span), _), expr)| {
            let expr_location = expr.location();
            Located::new(Field::new(name, expr), span.start()..expr_location.span().end())
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn primordial_type() {
        let ty = ty().parse(r#"
            String
        "#).unwrap();

        let expected = TypeName("String".into());

        assert!(matches!( &*ty, Type::Ref(
            TypeRef{
                name : TypeName(expected)
            }
       ) ));

        println!("{:?}", ty);
    }

    #[test]
    fn primordial_type_no_constraints() {
        let ty = ty().parse(r#"
            Integer -> { }
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn primordial_type_with_constraints() {
        let ty = ty().parse(r#"
            Integer -> { self > 42 }
        "#).unwrap();

        println!("{:?}", ty);
    }


    #[test]
    fn bare_type() {
        let ty = ty_decl().parse(r#"
            type LargerInteger
        "#).unwrap();

        println!("{:?}", ty)
    }

    #[test]
    fn type_alias_decl() {
        let ty = ty_decl().parse(r#"
            type LargerInteger : Integer
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn primordial_type_defn() {
        let ty = ty_decl().parse(r#"
            type LargerInteger : Integer {
                self > 42,
            }
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn simple_object_type_defn() {
        let ty = ty_decl().parse(r#"
            type RandomObject {
                foo: 42,
                baz: self > 82,
            }
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn nested_object_type_defn() {
        let ty = ty_decl().parse(r#"
            type RandomObject {
                foo: 42,
                baz: {
                    quux: "howdy"
                }
            }
        "#).unwrap();

        println!("{:?}", ty);
    }

    /*
    #[test]
    fn const_type() {
        let ty = ty_decl().parse(r#"
            type Version {
                42
            }
        "#).unwrap();

        println!("{:?}", ty);
    }
     */

    /*
    #[test]
    fn object_type_defn() {
        let ty = ty_decl().parse(r#"
            type RandomObject {
                foo: Integer { self > 42 },
                bar: LargerInteger,
                baz: {
                    x: 42,
                    y: String,
                },
                quux: String { self in [ "bob", "jim" ] },
            }
        "#).unwrap();

        println!("{:?}", ty);
    }

     */
}

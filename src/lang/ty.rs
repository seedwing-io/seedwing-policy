use crate::lang::expression::{expr, Expr, field_expr, Value};
use crate::lang::{ComparisonOp, DerivationOp, Located, Location, ParserError, ParserInput};
use chumsky::prelude::*;
use chumsky::Parser;
use chumsky::Span;
use std::fmt::{Debug, Formatter};

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
    Ref(Located<TypeName>),
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
    name: Located<String>,
    ty: Option<Located<Type>>,
}

impl Field {
    pub fn new(name: Located<String>, expr: Located<Type>) -> Self {
        Self {
            name,
            ty: Some(expr),
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

pub fn ty_defn() -> impl Parser<ParserInput, Located<TypeDefn>, Error=ParserError> + Clone {
    just("type")
        .padded()
        .ignored()
        .then(
            ty_name()
        )
        .then(
            ty().or_not()
        )
        .map(|((_, ty_name), ty)| {
            if let Some(ty) = ty {
                let loc = ty_name.span().start()..ty.span().end();
                Located::new(
                    TypeDefn::new(ty_name, ty),
                    loc)
            } else {
                let loc = ty_name.location();
                Located::new(
                    TypeDefn::new(ty_name,
                                  Located::new(Type::Anything, loc.clone()),
                    ),
                    loc,
                )
            }
        })
}

pub fn ty() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    recursive(|ty| {
        ty_constraint(ty.clone())
            .or(
                ty_object(ty.clone())
            ).or(
            ty_ref()
        )
    })
}

pub fn ty_ref() -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    ty_name()
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

pub fn ty_constraint(ty: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
    expr()
        .map(|expr| {
            let loc = expr.location();
            Located::new(
                Type::Constrained(expr),
                loc,
            )
        })
}

pub fn ty_object(ty: impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone) -> impl Parser<ParserInput, Located<Type>, Error=ParserError> + Clone {
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
    ty_defn().padded().repeated()
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
        let name = ty_name().parse("Bob").unwrap().into_inner();

        assert_eq!(name.name(), "Bob");
    }

    #[test]
    fn parse_ty_defn() {
        let ty = ty_defn().parse("type Bob").unwrap().into_inner();

        assert_eq!(ty.name.name(), "Bob");
    }

    #[test]
    fn parse_ty_ref() {
        let ty_ref = ty_ref().parse("Bob").unwrap().into_inner();

        println!("{:?}", ty_ref);

        assert!(
            matches!(
                ty_ref,
                Type::Ref(ty_name)
            if ty_name.name() == "Bob")
        );
    }

    #[test]
    fn parse_simple_obj_ty() {
        let ty = ty().then_ignore(end()).parse(r#"
            {
                foo: self > 23,
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
        let ty = ty().then_ignore(end()).parse(r#"
            {
                foo: self > 23,
                bar: {
                  quux: self < 14,
                },
                taco: Integer,
            }
        "#).unwrap().into_inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_compilation_unit() {
        let unit = compilation_unit().parse(r#"
            type Bob {
                foo: self > 23,
                bar: {
                  quux: self < 14,
                },
                taco: Integer,
            }

            type Jim {
            }

            type Dan

            type Lily {

            }

        "#).unwrap();

        println!("{:?}", unit);
    }
}

/*
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


 */
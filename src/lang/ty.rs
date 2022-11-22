use chumsky::Span;
use chumsky::Parser;
use chumsky::prelude::*;
use crate::lang::expression::{Expr, expr};
use crate::lang::{ParserError, ParserInput, Spanned};

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
        Self {
            name
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeDefn {
    name: TypeName,
    ty: Type,
}

impl TypeDefn {
    pub fn new(name: TypeName, ty: Type) -> Self {
        Self {
            name,
            ty,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Type {
    Anything,
    Ref(TypeRef),
    Constrained(Expr),
    Join(Box<Type>, Box<Type>),
    Meet(Box<Type>, Box<Type>),
    Nothing,
}

#[derive(Clone, Debug)]
pub struct ObjectType {
    fields: Vec<Field>,
}

impl ObjectType {
    pub fn new() -> Self {
        Self {
            fields: vec![]
        }
    }

    pub fn add_field(&mut self, field: Field) -> &Self {
        self.fields.push(field);
        self
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    name: String,
    expr: Option<Expr>,
}

impl Field {
    pub fn new(name: String, expr: Expr) -> Self {
        Self {
            name,
            expr: Some(expr),
        }
    }
}


pub fn ty_name() -> impl Parser<ParserInput, Spanned<TypeName>, Error=ParserError> + Clone {
    filter(|c: &char| (c.is_ascii_alphabetic() && c.is_uppercase()) || *c == '_')
        .map(Some)
        .chain::<char, Vec<_>, _>(
            filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_').repeated(),
        )
        .collect()
        .padded()
        .map_with_span(|v, span| (TypeName(v), span))
}

pub fn ty_ref() -> impl Parser<ParserInput, Spanned<Type>, Error=ParserError> + Clone {
    ty_name().map(|name| {
        (Type::Ref(TypeRef::new(name.0)), name.1)
    })
}

pub fn super_ty_decl() -> impl Parser<ParserInput, Spanned<Type>, Error=ParserError> + Clone {
    just(":").padded().ignored()
        .then(ty())
        .map(|v| {
            v.1
        })
}

pub fn ty_decl() -> impl Parser<ParserInput, Spanned<TypeDefn>, Error=ParserError> + Clone {
    just("type").padded().map_with_span(|_, span| {
        span
    })
        .then(
            ty_name().padded()
                .then(
                    super_ty_decl().or_not()
                )
                .then(
                    constrained_ty().or_not()
                )
        )
        .map(|(type_span, ((name, super_ty), defn))| {
            match (super_ty, defn) {
                (None, None) => {
                    ( TypeDefn::new(name.0, Type::Anything),type_span.start()..name.1.end())
                }
                (Some(super_ty), None) => {
                    ( TypeDefn::new(name.0, super_ty.0),
                        type_span.start()..super_ty.1.end())
                }
                (None, Some(defn)) => {
                    ( TypeDefn::new(name.0, defn.0),
                        type_span.start()..defn.1.end())
                }
                (Some(super_ty), Some(defn)) => {
                    ( TypeDefn::new(name.0,
                                  Type::Meet(Box::new(super_ty.0), Box::new(defn.0)),
                    ),
                        type_span.start()..defn.1.end())
                }
            }
        })
}

pub fn constraints() -> impl Parser<ParserInput, Option<Spanned<Expr>>, Error=ParserError> + Clone {
    just("{").padded().ignored()
        .then(
            expr()
                .repeated()
                .separated_by(
                    just(",")
                        .padded()
                        .ignored()
                )
                .allow_trailing().padded()
        )
        .then(just("}").padded().ignored())
        .map_with_span(|((_left_curly, exprs), _right_curly), span| {
            let expr = exprs.iter().flatten().cloned().reduce(|accum, each| {
                let span = accum.1.start()..each.1.end();
                (Expr::And(Box::new(accum), Box::new(each)), span)
            });

            if let Some(expr) = expr {
                Some(expr)
            } else {
                None
            }
        })
}

pub fn constrained_ty() -> impl Parser<ParserInput, Spanned<Type>, Error=ParserError> + Clone {
    constraints()
        .map_with_span(|v, span| {
            (v.map_or(Type::Anything, |v| Type::Constrained(v.0)), span)
        })
}


pub fn ty() -> impl Parser<ParserInput, Spanned<Type>, Error=ParserError> + Clone {
    ty_ref()
        .then(
            constraints().or_not()
        )
        .map(|(ty, constraints)| {
            if let Some(Some(constraints)) = constraints {
                let span = ty.1.start()..constraints.1.end();
                (Type::Meet(Box::new(ty.0),
                            Box::new(Type::Constrained(constraints.0.clone()))), span)
            } else {
                ty
            }
        })
}


/*
pub fn expr_ty() -> impl Parser<ParserInput, Spanned<Type>, Error=ParserError> + Clone {
    expr().then(just(";").padded().ignored())
        .map(|(expr, _semi)| {
            (Type::Expr(expr.0), expr.1)
        })
}

 */

/*
pub fn object_ty() -> impl Parser<ParserInput, Spanned<Type>, Error=ParserError> + Clone {
    just("{").padded().ignored().map_with_span(|_, span| span)
        .then(field().separated_by(just(",").padded().ignored()).allow_trailing())
        .then(just("}").padded().ignored().map_with_span(|_, span| span))
        .map(|((open_curly, fields), close_curly)| {
            let mut object_ty = ObjectType::new();
            for f in fields {
                object_ty.add_field(f.0);
            }
            (Type::Object(object_ty), open_curly.start()..close_curly.end())
        })
}
 */

fn field() -> impl Parser<ParserInput, Spanned<Field>, Error=ParserError> + Clone {
    text::ident().padded().map_with_span(|v, span| (v, span))
        .then(just(":").padded().ignored())
        .then(expr())
        .map(|(((name, span), _), expr)| {
            (Field::new(name, expr.0), span.start()..expr.1.end())
        })
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn primordial_type() {
        let (ty, span) = ty().parse(r#"
            String
        "#).unwrap();

        let expected = TypeName("String".into());

        assert!(matches!( &ty, Type::Ref(
            TypeRef{
                name : TypeName(expected)
            }
       ) ));

        println!("{:?}", ty);
    }

    #[test]
    fn primordial_type_no_constraints() {
        let (ty, span) = ty().parse(r#"
            Integer -> { }
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn primordial_type_with_constraints() {
        let (ty, span) = ty().parse(r#"
            Integer -> { self > 42 }
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn bare_type() {
        let (ty, span) = ty_decl().parse(r#"
            type LargerInteger
        "#).unwrap();

        println!("{:?}", ty)
    }

    #[test]
    fn type_alias_decl() {
        let (ty, span) = ty_decl().parse(r#"
            type LargerInteger : Integer
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn primordial_type_defn() {
        let (ty, span) = ty_decl().parse(r#"
            type LargerInteger : Integer {
                self > 42,
            }
        "#).unwrap();

        println!("{:?}", ty);
    }

    #[test]
    fn object_type_defn() {
        let (ty, span) = ty_decl().parse(r#"
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
}
use std::collections::HashMap;
//use crate::lang::expr::{expr, Expr, field_expr, Value};
use crate::lang::{CompilationUnit, Located, Location, ParserError, ParserInput, Source, Span, TypePath, Use};
use chumsky::prelude::*;
use chumsky::Parser;
use std::fmt::{Debug, Formatter};
use crate::lang::expr::{Expr, expr};
use crate::value::Value;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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

    pub fn name(&self) -> Located<TypeName> {
        self.name.clone()
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypePath>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<TypeName, Option<Located<TypePath>>>) {
        self.ty.qualify_types(types);
    }
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Ref(Located<TypePath>),
    Const(Located<Value>),
    Object(ObjectType),
    Expr(Located<Expr>),
    Join(Box<Located<Type>>, Box<Located<Type>>),
    Meet(Box<Located<Type>>, Box<Located<Type>>),
    Functional(Located<FunctionName>, Option<Box<Located<Type>>>),
    List(Box<Located<Type>>),
    Nothing,
}

impl Type {
    pub(crate) fn referenced_types(&self) -> Vec<Located<TypePath>> {
        match self {
            Type::Anything => Vec::default(),
            Type::Ref(inner) => vec![
                inner.clone()
            ],
            Type::Const(_) => Vec::default(),
            Type::Object(inner) => inner.referenced_types(),
            Type::Expr(_) => Vec::default(),
            Type::Join(lhs, rhs) => lhs.referenced_types().iter().chain(rhs.referenced_types().iter()).cloned().collect(),
            Type::Meet(lhs, rhs) => lhs.referenced_types().iter().chain(rhs.referenced_types().iter()).cloned().collect(),
            Type::Functional(_, inner) => inner.as_ref().map_or(Vec::default(), |inner| inner.referenced_types()),
            Type::List(inner) => inner.referenced_types(),
            Type::Nothing => Vec::default(),
        }
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<TypeName, Option<Located<TypePath>>>) {
        match self {
            Type::Anything => {}
            Type::Ref(ref mut path) => {
                if path.inner.0.is_empty() {
                    // it's a simple single-word name, needs qualifying, perhaps.
                    if let Some(Some(qualified)) = types.get(&*path.inner.1) {
                        *path = qualified.clone();
                    }
                }
            }
            Type::Const(_) => {}
            Type::Object(inner) => {
                inner.qualify_types(types);
            }
            Type::Expr(_) => {}
            Type::Join(lhs, rhs) => {
                lhs.qualify_types(types);
                rhs.qualify_types(types);
            }
            Type::Meet(lhs, rhs) => {
                lhs.qualify_types(types);
                rhs.qualify_types(types);
            }
            Type::Functional(_, inner) => {
                //inner.qualify_types(types);
                inner.as_mut().map(|inner| {
                    inner.qualify_types(types)
                });
            }
            Type::List(inner) => {
                inner.qualify_types(types);
            }
            Type::Nothing => {}
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "Anything"),
            Type::Ref(r) => write!(f, "{:?}", r),
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

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypePath>> {
        self.fields.iter().flat_map(|e| {
            e.referenced_types()
        }).collect()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<TypeName, Option<Located<TypePath>>>) {
        for field in &mut self.fields {
            field.qualify_types(types);
        }
    }

    pub fn fields(&self) -> &Vec<Located<Field>> {
        &self.fields
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

    pub fn name(&self) -> &Located<String> {
        &self.name
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypePath>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<TypeName, Option<Located<TypePath>>>) {
        self.ty.qualify_types(types)
    }
}

fn op(op: &str) -> impl Parser<ParserInput, &str, Error=ParserError> + Clone {
    just(op).padded()
}

pub fn use_statement() -> impl Parser<ParserInput, Located<Use>, Error=ParserError> + Clone {
    just("use").padded().ignored()
        .then(type_path())
        .then(as_clause().or_not())
        // .then( just(";").padded().ignored() )
        .map_with_span(|(((_, type_path), as_clause)), span| {
            Located::new(
                Use::new(type_path, as_clause),
                span,
            )
        })
}

pub fn as_clause() -> impl Parser<ParserInput, Located<TypeName>, Error=ParserError> + Clone {
    just("as").padded().ignored()
        .then(package_or_type_name())
        .map(|(_, v)| {
            v
        })
}

pub fn type_path() -> impl Parser<ParserInput, Located<TypePath>, Error=ParserError> + Clone {
    package_or_type_name()
        .separated_by(just("::").padded().ignored())
        .at_least(1)
        .map_with_span(|segments, span| {
            Located::new(
                TypePath::new(segments),
                span,
            )
        })
}

pub fn package_or_type_name() -> impl Parser<ParserInput, Located<TypeName>, Error=ParserError> + Clone {
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
            package_or_type_name()
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
        .map_with_span(|s: String, span| Located::new(s.parse::<i64>().unwrap().into(), span))
}

pub fn decimal_literal() -> impl Parser<ParserInput, Located<Value>, Error=ParserError> + Clone {
    text::int(10)
        .then(just('.').then(text::int(10)))
        .padded()
        .map_with_span(
            |(integral, (_dot, decimal)): (String, (char, String)), span| {
                Located::new(
                    format!("{}.{}", integral, decimal).parse::<f64>().unwrap().into(),
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
                .collect::<String>()
        )
        .then(
            just('"')
                .ignored()
        )
        .padded()
        .map_with_span(|((_, x), _), span: Span| {
            Located::new(
                x.into(),
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
        .then(expr.clone().or_not())
        .then(
            just(")")
                .padded()
                .ignored()
        )
        .map_with_span(|((((fn_name, _)), ty), _), span| {
            let fn_type = Type::Functional(
                fn_name,
                //ty.map(|inner| Box::new(inner))
                None,
            );

            Located::new(
                fn_type,
                span,
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
    type_path()
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

pub fn compilation_unit<S: Into<Source> + Clone>(source: S) -> impl Parser<ParserInput, CompilationUnit, Error=ParserError> + Clone {
    use_statement().padded().repeated()
        .then(
            type_definition().padded().repeated()
        )
        .then_ignore(end())
        .map(move |(use_statements, types)| {
            let mut unit = CompilationUnit::new(source.clone().into());

            for e in use_statements {
                unit.add_use(e)
            }

            for e in types {
                unit.add_type(e)
            }

            unit
        })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_ty_name() {
        let name = package_or_type_name().parse("bob").unwrap().into_inner();

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
            if ty_name.type_name().name() == "bob")
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
        let unit = compilation_unit("my_file.dog").parse(r#"
            use foo::bar::bar
            use x::y::z as osi-approved-license

            type signed = SHA256()

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

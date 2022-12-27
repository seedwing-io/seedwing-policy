use std::collections::HashMap;
//use crate::lang::expr::{expr, Expr, field_expr, Value};
use crate::lang::expr::{expr, Expr};
use crate::lang::literal::{anything_literal, decimal_literal, integer_literal, string_literal};
use crate::lang::package::{PackageName, PackagePath};
use crate::lang::{
    op, use_statement, CompilationUnit, Located, Location, ParserError, ParserInput,
    SourceLocation, SourceSpan, Use,
};
use crate::value::Value;
use chumsky::prelude::*;
use chumsky::Parser;
use std::fmt::{Debug, Display, Formatter};
use std::iter::once;
use std::ops::Deref;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TypeName {
    package: Option<PackagePath>,
    name: String,
}

impl Display for TypeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_type_str())
    }
}

impl TypeName {
    pub fn new(package: Option<PackagePath>, name: String) -> Self {
        Self { package, name }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn is_qualified(&self) -> bool {
        self.package.is_some()
    }

    pub fn as_type_str(&self) -> String {
        let mut fq = String::new();
        if let Some(package) = &self.package {
            fq.push_str(&package.as_package_str());
            fq.push_str("::");
        }

        fq.push_str(&self.name);

        fq
    }
}

impl From<String> for TypeName {
    fn from(path: String) -> Self {
        let mut segments = path.split("::").map(|e| e.into()).collect::<Vec<String>>();
        if segments.is_empty() {
            Self::new(None, "".into())
        } else {
            let tail = segments.pop().unwrap();
            if segments.is_empty() {
                Self {
                    package: None,
                    name: tail,
                }
            } else {
                let package = Some(segments.into());
                Self {
                    package,
                    name: tail,
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TypeDefn {
    name: Located<String>,
    ty: Located<Type>,
    parameters: Vec<Located<String>>,
}

impl TypeDefn {
    pub fn new(name: Located<String>, ty: Located<Type>, parameters: Vec<Located<String>>) -> Self {
        Self {
            name,
            ty,
            parameters,
        }
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        self.ty.qualify_types(types);
    }

    pub(crate) fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Ref(Located<TypeName>, Vec<Located<Type>>),
    Parameter(Located<String>),
    Const(Located<Value>),
    Object(ObjectType),
    Expr(Located<Expr>),
    Join(Box<Located<Type>>, Box<Located<Type>>),
    Meet(Box<Located<Type>>, Box<Located<Type>>),
    Refinement(Box<Located<Type>>, Box<Located<Type>>),
    List(Box<Located<Type>>),
    MemberQualifier(Located<MemberQualifier>, Box<Located<Type>>),
    Nothing,
}

#[derive(Debug, Clone)]
pub enum MemberQualifier {
    All,
    Any,
    N(Located<u32>),
}

impl Type {
    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        match self {
            Type::Anything => Vec::default(),
            Type::Ref(inner, arguuments) => once(inner.clone())
                .chain(arguuments.iter().flat_map(|e| e.referenced_types()))
                .collect(),
            Type::Const(_) => Vec::default(),
            Type::Object(inner) => inner.referenced_types(),
            Type::Expr(_) => Vec::default(),
            Type::Join(lhs, rhs) => lhs
                .referenced_types()
                .iter()
                .chain(rhs.referenced_types().iter())
                .cloned()
                .collect(),
            Type::Meet(lhs, rhs) => lhs
                .referenced_types()
                .iter()
                .chain(rhs.referenced_types().iter())
                .cloned()
                .collect(),
            Type::Refinement(primary, refinement) => primary
                .referenced_types()
                .iter()
                .chain(refinement.referenced_types().iter())
                .cloned()
                .collect(),
            Type::List(inner) => inner.referenced_types(),
            Type::Nothing => Vec::default(),
            Type::MemberQualifier(_, inner) => inner.referenced_types(),
            Type::Parameter(_) => Vec::default(),
        }
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        match self {
            Type::Anything => {}
            Type::Ref(ref mut name, arguments) => {
                if !name.is_qualified() {
                    // it's a simple single-word name, needs qualifying, perhaps.
                    if let Some(Some(qualified)) = types.get(&name.name()) {
                        *name = qualified.clone();
                    }
                }
                for arg in arguments {
                    arg.qualify_types(types);
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
            Type::Refinement(primary, refinement) => {
                primary.qualify_types(types);
                refinement.qualify_types(types);
            }
            Type::List(inner) => {
                inner.qualify_types(types);
            }
            Type::MemberQualifier(_, inner) => {
                inner.qualify_types(types);
            }
            Type::Nothing => {}
            Type::Parameter(_) => {}
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "Anything"),
            Type::Ref(r, args) => write!(f, "{:?}<{:?}>", r, args),
            Type::Const(value) => write!(f, "{:?}", value),
            Type::Join(l, r) => write!(f, "Join({:?}, {:?})", l, r),
            Type::Meet(l, r) => write!(f, "Meet({:?}, {:?})", l, r),
            Type::Nothing => write!(f, "Nothing"),
            Type::Object(obj) => write!(f, "{:?}", obj),
            Type::Refinement(fn_name, ty) => write!(f, "{:?}({:?})", fn_name, ty),
            Type::List(ty) => write!(f, "[{:?}]", ty),
            Type::Expr(expr) => write!(f, "#({:?})", expr),
            Type::MemberQualifier(qualifier, ty) => write!(f, "{:?}::{:?}", qualifier, ty),
            Type::Parameter(name) => write!(f, "{:?}", name),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectType {
    fields: Vec<Located<Field>>,
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectType {
    pub fn new() -> Self {
        Self { fields: vec![] }
    }

    pub fn add_field(&mut self, field: Located<Field>) -> &Self {
        self.fields.push(field);
        self
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.fields
            .iter()
            .flat_map(|e| e.referenced_types())
            .collect()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
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
        Self { name, ty }
    }

    pub fn name(&self) -> &Located<String> {
        &self.name
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        self.ty.qualify_types(types)
    }
}

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
                        .map(|e| {
                            Located::new(PackageName::new(e.clone().into_inner()), e.location())
                        })
                        .collect::<Vec<Located<PackageName>>>(),
                ))
            };

            Located::new(
                TypeName {
                    package,
                    name: tail.into_inner(),
                },
                span,
            )
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
                Some(params) => params.iter().cloned().map(|e| e.into_inner()).collect(),
                None => Vec::new(),
            };
            type_expr(visible_parameters)
        })
        .map(|(_, x)| x)
}

pub fn type_definition() -> impl Parser<ParserInput, Located<TypeDefn>, Error = ParserError> + Clone
{
    just("type")
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
    logical_and(expr.clone(), visible_parameters)
        .then(op("||").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let location = lhs.span().start()..rhs.span().end();
            Located::new(Type::Join(Box::new(lhs), Box::new(rhs)), location)
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    ty(expr.clone(), visible_parameters)
        .then(op("&&").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let location = lhs.span().start()..rhs.span().end();
            Located::new(Type::Meet(Box::new(lhs), Box::new(rhs)), location)
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
                Located::new(ty.into_inner(), loc)
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
            Located::new(Type::MemberQualifier(qualifier, Box::new(ty)), span)
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
        .then(expr)
        .then(just("]").padded().ignored())
        .map_with_span(|((_, ty), _), span| Located::new(Type::List(Box::new(ty)), span))
}

pub fn ty(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    expr_ty()
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
    visisble_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    type_name()
        .then(type_arguments(expr).or_not())
        .map_with_span(move |(name, arguments), span| {
            let loc = name.location();
            let arguments = arguments.unwrap_or_default();
            if visisble_parameters.contains(&name.name) {
                if !arguments.is_empty() {
                    todo!("arguments to parameter references not currently allowed")
                }
                Located::new(Type::Parameter(Located::new(name.name.clone(), span)), loc)
            } else {
                Located::new(
                    Type::Ref(Located::new(name.into_inner(), loc.clone()), arguments),
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
    use crate::lang::compilation_unit;

    #[test]
    fn parse_ty_name() {
        let name = type_name().parse("bob").unwrap().into_inner();

        assert_eq!(name.name(), "bob");
    }

    #[test]
    fn parse_ty_defn() {
        let ty = type_definition().parse("type bob").unwrap().into_inner();

        assert_eq!(&*ty.name.into_inner(), "bob");
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
            .into_inner();

        println!("{:?}", ty);

        assert!(matches!(ty, Type::Object(_)));

        if let Type::Object(ty) = ty {
            assert!(matches!(
                ty.fields.iter().find(|e| *e.name == "foo"),
                Some(_)
            ));
            assert!(matches!(
                ty.fields.iter().find(|e| *e.name == "bar"),
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
            .into_inner();

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
            .into_inner();

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
            .into_inner();

        println!("{:?}", ty);
    }

    #[test]
    fn parse_compilation_unit() {
        let unit = compilation_unit("my_file.dog")
            .parse(
                r#"
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

        "#,
            )
            .unwrap();

        println!("{:?}", unit);
    }
}

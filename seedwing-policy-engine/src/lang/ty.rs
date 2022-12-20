use std::collections::HashMap;
//use crate::lang::expr::{expr, Expr, field_expr, Value};
use crate::lang::expr::{expr, Expr};
use crate::lang::{
    CompilationUnit, Located, Location, ParserError, ParserInput, Source, Span, Use,
};
use crate::value::Value;
use chumsky::prelude::*;
use chumsky::Parser;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PackageName(String);

impl Deref for PackageName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PackagePath {
    is_absolute: bool,
    path: Vec<Located<PackageName>>,
}

impl From<Vec<String>> for PackagePath {
    fn from(mut segments: Vec<String>) -> Self {
        let first = segments.get(0).unwrap();
        let is_absolute = first.is_empty();
        if is_absolute {
            segments = segments[1..].to_vec()
        }

        Self {
            is_absolute: true,
            path: segments
                .iter()
                .map(|e| Located::new(PackageName(e.clone()), 0..0))
                .collect(),
        }
    }
}

impl PackagePath {
    pub fn from_parts(segments: Vec<&str>) -> Self {
        Self {
            is_absolute: true,
            path: segments
                .iter()
                .map(|e| Located::new(PackageName(String::from(*e)), 0..0))
                .collect(),
        }
    }

    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    pub fn is_qualified(&self) -> bool {
        self.path.len() > 1
    }

    pub fn type_name(&self, name: String) -> TypeName {
        println!("TN {:?} -> {}", self.path, name);
        TypeName {
            package: Some(self.clone()),
            name,
        }
    }

    pub fn as_package_str(&self) -> String {
        let mut fq = String::new();
        if self.is_absolute {
            fq.push_str("::");
        }

        fq.push_str(
            &self
                .path
                .iter()
                .map(|e| e.inner.0.clone())
                .collect::<Vec<String>>()
                .join("::"),
        );

        fq
    }

    pub fn path(&self) -> &Vec<Located<PackageName>> {
        &self.path
    }
}

impl From<Source> for PackagePath {
    fn from(src: Source) -> Self {
        let segments = src
            .name
            .split('/')
            .map(|segment| Located::new(PackageName(segment.into()), 0..0))
            .collect();

        Self {
            is_absolute: true,
            path: segments,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TypeName {
    package: Option<PackagePath>,
    name: String,
}

impl TypeName {
    pub fn new(name: String) -> Self {
        println!("TN {}", name);
        Self {
            package: None,
            name,
        }
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
        println!("segments {:?}", segments);
        if segments.is_empty() {
            Self::new("".into())
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
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Ref(Located<TypeName>),
    Parameter(Located<String>),
    Const(Located<Value>),
    Object(ObjectType),
    Expr(Located<Expr>),
    Join(Box<Located<Type>>, Box<Located<Type>>),
    Meet(Box<Located<Type>>, Box<Located<Type>>),
    Functional(Located<TypeName>, Option<Box<Located<Type>>>),
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
            Type::Ref(inner) => vec![inner.clone()],
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
            Type::Functional(_, inner) => inner
                .as_ref()
                .map_or(Vec::default(), |inner| inner.referenced_types()),
            Type::List(inner) => inner.referenced_types(),
            Type::Nothing => Vec::default(),
            Type::MemberQualifier(_, inner) => inner.referenced_types(),
            Type::Parameter(_) => Vec::default(),
        }
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        match self {
            Type::Anything => {}
            Type::Ref(ref mut name) => {
                if !name.is_qualified() {
                    // it's a simple single-word name, needs qualifying, perhaps.
                    if let Some(Some(qualified)) = types.get(&name.name()) {
                        println!("replace {:?} with {:?}", name, qualified);
                        *name = qualified.clone();
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
                if let Some(inner) = inner.as_mut() {
                    inner.qualify_types(types)
                }
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
            Type::Ref(r) => write!(f, "{:?}", r),
            Type::Const(value) => write!(f, "{:?}", value),
            Type::Join(l, r) => write!(f, "Join({:?}, {:?})", l, r),
            Type::Meet(l, r) => write!(f, "Meet({:?}, {:?})", l, r),
            Type::Nothing => write!(f, "Nothing"),
            Type::Object(obj) => write!(f, "{:?}", obj),
            Type::Functional(fn_name, ty) => write!(f, "{:?}({:?})", fn_name, ty),
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

fn op(op: &str) -> impl Parser<ParserInput, &str, Error = ParserError> + Clone {
    just(op).padded()
}

pub fn use_statement() -> impl Parser<ParserInput, Located<Use>, Error = ParserError> + Clone {
    just("use")
        .padded()
        .ignored()
        .then(type_name())
        .then(as_clause().or_not())
        // .then( just(";").padded().ignored() )
        .map_with_span(|(((_, type_path), as_clause)), span| {
            Located::new(Use::new(type_path, as_clause), span)
        })
}

pub fn as_clause() -> impl Parser<ParserInput, Located<String>, Error = ParserError> + Clone {
    just("as")
        .padded()
        .ignored()
        .then(simple_type_name())
        .map(|(_, v)| v)
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
                Some(PackagePath {
                    is_absolute: true,
                    path: segments
                        .iter()
                        .map(|e| Located::new(PackageName(e.clone().into_inner()), e.location()))
                        .collect(),
                })
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
                .map_with_span(|name: String, span| Located::new(name, span))
                .separated_by(just(",").padded())
                .allow_trailing(),
        )
        .then(just(">").padded().ignored())
        .map(|((_, names), _)| names)
}

pub fn type_definition() -> impl Parser<ParserInput, Located<TypeDefn>, Error = ParserError> + Clone
{
    just("type")
        .padded()
        .ignored()
        .then(simple_type_name())
        .then(type_parameters().or_not())
        .then(just("=").padded().ignored())
        .then_with(|(((_, ty_name), params), _)| {
            let params = params.unwrap_or(Default::default());
            let visible_parameters = params.iter().cloned().map(|e| e.into_inner()).collect();
            type_expr(visible_parameters)
                .or_not()
                .map(move |ty| (ty_name.clone(), params.clone(), ty))
        })
        .map(|(ty_name, params, ty)| {
            let ty = ty.unwrap_or({
                let loc = ty_name.location();
                Located::new(Type::Nothing, loc)
            });

            let loc = ty_name.span().start()..ty.span().end();
            Located::new(TypeDefn::new(ty_name, ty, params), loc)
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
    logical_and(expr.clone(), visible_parameters.clone())
        .then(op("||").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            println!("lhs {:?}", lhs);
            println!("rhs {:?}", rhs);
            let location = lhs.span().start()..rhs.span().end();
            Located::new(Type::Join(Box::new(lhs), Box::new(rhs)), location)
        })
}

pub fn logical_and(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
    visible_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    ty(expr.clone(), visible_parameters.clone())
        .then(op("&&").then(expr).repeated())
        .foldl(|lhs, (_op, rhs)| {
            let location = lhs.span().start()..rhs.span().end();
            Located::new(Type::Meet(Box::new(lhs), Box::new(rhs)), location)
        })
}

pub fn integer_literal() -> impl Parser<ParserInput, Located<Value>, Error = ParserError> + Clone {
    text::int::<char, ParserError>(10)
        .padded()
        .map_with_span(|s: String, span| Located::new(s.parse::<i64>().unwrap().into(), span))
}

pub fn decimal_literal() -> impl Parser<ParserInput, Located<Value>, Error = ParserError> + Clone {
    text::int(10)
        .then(just('.').then(text::int(10)))
        .padded()
        .map_with_span(
            |(integral, (_dot, decimal)): (String, (char, String)), span| {
                Located::new(
                    format!("{}.{}", integral, decimal)
                        .parse::<f64>()
                        .unwrap()
                        .into(),
                    span,
                )
            },
        )
}

pub fn string_literal() -> impl Parser<ParserInput, Located<Value>, Error = ParserError> + Clone {
    just('"')
        .ignored()
        .then(filter(|c: &char| *c != '"').repeated().collect::<String>())
        .then(just('"').ignored())
        .padded()
        .map_with_span(|((_, x), _), span: Span| Located::new(x.into(), span))
}

pub fn anything_literal() -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    just("anything")
        .padded()
        .ignored()
        .map_with_span(|_, span| Located::new(Type::Anything, span))
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

pub fn functional_ty(
    expr: impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    type_name()
        .then(just("(").padded().ignored())
        .then(expr.or_not())
        .then(just(")").padded().ignored())
        .map_with_span(|((((fn_name, _)), ty), _), span| {
            let fn_type = Type::Functional(fn_name, ty.map(Box::new));

            Located::new(fn_type, span)
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
        .or(functional_ty(expr.clone()))
        .or(const_type())
        .or(object_type(expr))
        .or(type_ref(visible_parameters))
}

pub fn type_ref(
    visisble_parameters: Vec<String>,
) -> impl Parser<ParserInput, Located<Type>, Error = ParserError> + Clone {
    type_name().map_with_span(move |name, span| {
        let loc = name.location();
        if visisble_parameters.contains(&name.name) {
            Located::new(Type::Parameter(Located::new(name.name.clone(), span)), loc)
        } else {
            Located::new(Type::Ref(Located::new(name.into_inner(), loc.clone())), loc)
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
        .then(just(":").padded().ignored())
        .then(ty)
        .map(|((name, _), ty)| {
            let loc = name.span().start()..ty.span().end();
            Located::new(Field::new(name, ty), loc)
        })
}

pub fn compilation_unit<S: Into<Source> + Clone>(
    source: S,
) -> impl Parser<ParserInput, CompilationUnit, Error = ParserError> + Clone {
    use_statement()
        .padded()
        .repeated()
        .then(type_definition().padded().repeated())
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

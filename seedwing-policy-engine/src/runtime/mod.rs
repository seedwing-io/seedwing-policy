pub mod linker;
pub mod sources;

use crate::core::{Function, FunctionError};
use crate::lang::expr::Expr;
use crate::lang::ty::{MemberQualifier, PackagePath, Type, TypeName};
use crate::lang::{
    CompilationUnit, Located, ParserError, ParserInput, PolicyParser, SourceLocation, SourceSpan,
};
use crate::package::Package;
use crate::runtime::cache::SourceCache;
use crate::runtime::linker::Linker;
use crate::value::Value;
use ariadne::Cache;
use async_mutex::Mutex;
use chumsky::{Error, Stream};
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::{ready, Future};
use std::mem;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::ready;

pub mod cache;

pub enum Component {
    Package(PackageHandle),
    Type(Arc<TypeHandle>),
}

#[derive(Clone, Debug)]
pub enum BuildError {
    TypeNotFound(SourceLocation, SourceSpan, String),
    Parser(SourceLocation, ParserError),
}

impl BuildError {
    pub fn source_location(&self) -> SourceLocation {
        match self {
            BuildError::TypeNotFound(loc, _, _) => loc.clone(),
            BuildError::Parser(loc, _) => loc.clone(),
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            BuildError::TypeNotFound(_, span, _) => span.clone(),
            BuildError::Parser(_, err) => err.span(),
        }
    }
}

impl From<(SourceLocation, ParserError)> for BuildError {
    fn from(inner: (SourceLocation, ParserError)) -> Self {
        Self::Parser(inner.0, inner.1)
    }
}

#[derive(Default)]
pub struct Builder {
    units: Vec<CompilationUnit>,
    packages: HashMap<PackagePath, Package>,
    source_cache: SourceCache,
}

impl Builder {
    pub fn new() -> Self {
        let mut builder = Self {
            units: Default::default(),
            packages: Default::default(),
            source_cache: Default::default(),
        };
        builder.add_package(crate::core::sigstore::package());
        builder.add_package(crate::core::x509::package());
        builder.add_package(crate::core::base64::package());

        builder
    }

    pub fn source_cache(&self) -> &SourceCache {
        &self.source_cache
    }

    pub fn build<S, SrcIter>(&mut self, sources: SrcIter) -> Result<(), Vec<BuildError>>
    where
        Self: Sized,
        S: Into<String>,
        SrcIter: Iterator<Item = (SourceLocation, S)>,
    {
        let mut errors = Vec::new();
        for (source, stream) in sources {
            log::info!("loading policies from {}", source);

            let input = stream.into();

            self.source_cache.add(source.clone(), input.clone().into());
            let unit = PolicyParser::default().parse(source.clone(), input);
            match unit {
                Ok(unit) => self.add_compilation_unit(unit),
                Err(err) => {
                    for e in err {
                        errors.push((source.clone(), e).into())
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn add_compilation_unit(&mut self, unit: CompilationUnit) {
        self.units.push(unit)
    }

    pub fn add_package(&mut self, package: Package) {
        self.packages.insert(package.path(), package);
    }

    pub async fn link(&mut self) -> Result<Arc<Runtime>, Vec<BuildError>> {
        let mut core_units = Vec::new();

        let mut errors = Vec::new();

        for pkg in self.packages.values() {
            for (source, stream) in pkg.source_iter() {
                log::info!("loading {}", source);
                let unit = PolicyParser::default().parse(source.to_owned(), stream);
                match unit {
                    Ok(unit) => {
                        core_units.push(unit);
                    }
                    Err(err) => {
                        for e in err {
                            errors.push((source.clone(), e).into())
                        }
                    }
                }
            }
        }

        for unit in core_units {
            self.add_compilation_unit(unit);
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Linker::new(&mut self.units, &mut self.packages)
            .link()
            .await
    }
}

pub type EvaluationResult = Option<Arc<Mutex<Value>>>;

//#[derive(Default, Debug)]
//pub struct EvaluationResult {
//value: Option<Arc<Mutex<Value>>>,
//}

/*
impl EvaluationResult {
    pub fn new() -> Self {
        Self { value: None }
    }

    pub fn set_value(mut self, value: Arc<Mutex<Value>>) -> Self {
        self.value.replace(value);
        self
    }

    pub fn value(&self) -> &Option<Arc<Mutex<Value>>> {
        &self.value
    }

    pub fn matches(&self) -> bool {
        self.value.is_some()
    }
}

 */

#[derive(Debug)]
pub enum RuntimeError {
    Lock,
    NoSuchType,
    Function(FunctionError),
}

pub struct Runtime {
    types: Mutex<HashMap<TypeName, Arc<TypeHandle>>>,
}

pub struct PackageHandle {}

#[derive(Default, Debug)]
pub struct TypeHandle {
    ty: Mutex<Option<Arc<Located<RuntimeType>>>>,
    parameters: Vec<Located<String>>,
}

impl TypeHandle {
    pub fn new() -> Self {
        Self {
            ty: Mutex::new(None),
            parameters: vec![],
        }
    }

    pub fn new_with(ty: Located<RuntimeType>) -> Self {
        Self {
            ty: Mutex::new(Some(Arc::new(ty))),
            parameters: vec![],
        }
    }

    async fn with(mut self, ty: Located<RuntimeType>) -> Self {
        self.define(Arc::new(ty)).await;
        self
    }

    fn with_parameters(mut self, parameters: Vec<Located<String>>) -> Self {
        self.parameters = parameters;
        self
    }

    fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }

    async fn define(&self, ty: Arc<Located<RuntimeType>>) {
        self.ty.lock().await.replace(ty);
    }

    pub async fn ty(&self) -> Arc<Located<RuntimeType>> {
        self.ty.lock().await.as_ref().unwrap().clone()
    }

    async fn evaluate(
        &self,
        value: Arc<Mutex<Value>>,
        bindings: &Bindings,
    ) -> Result<EvaluationResult, RuntimeError> {
        if let Some(ty) = &*self.ty.lock().await {
            ty.evaluate(value, bindings).await
        } else {
            Err(RuntimeError::NoSuchType)
        }
    }
}

impl Runtime {
    pub(crate) fn new() -> Arc<Self> {
        let mut initial_types = HashMap::new();
        initial_types.insert(
            TypeName::new("int".into()),
            Arc::new(TypeHandle::new_with(Located::new(
                RuntimeType::Primordial(PrimordialType::Integer),
                0..0,
            ))),
        );

        initial_types.insert(
            TypeName::new("string".into()),
            Arc::new(TypeHandle::new_with(Located::new(
                RuntimeType::Primordial(PrimordialType::String),
                0..0,
            ))),
        );

        initial_types.insert(
            TypeName::new("boolean".into()),
            Arc::new(TypeHandle::new_with(Located::new(
                RuntimeType::Primordial(PrimordialType::Boolean),
                0..0,
            ))),
        );

        initial_types.insert(
            TypeName::new("decimal".into()),
            Arc::new(TypeHandle::new_with(Located::new(
                RuntimeType::Primordial(PrimordialType::Decimal),
                0..0,
            ))),
        );

        Arc::new(Self {
            types: Mutex::new(initial_types),
        })
    }

    pub async fn get<P: Into<String>>(&self, path: P) -> Option<Component> {
        let path = TypeName::from(path.into());
        if let Some(ty) = self.types.lock().await.get(&path) {
            Some(Component::Type(ty.clone()))
        } else {
            // check to see if there's a package handle we could create.
            None
        }
    }

    pub async fn evaluate<P: Into<String>, V: Into<Value>>(
        &self,
        path: P,
        value: V,
    ) -> Result<EvaluationResult, RuntimeError> {
        let value = Arc::new(Mutex::new(value.into()));
        let path = TypeName::from(path.into());
        let ty = &self.types.lock().await[&path];
        let ty = ty.ty().await;
        let bindings = Bindings::default();
        ty.evaluate(value, &bindings).await
    }

    async fn declare(self: &mut Arc<Self>, path: TypeName, parameters: Vec<Located<String>>) {
        self.types.lock().await.insert(
            path,
            Arc::new(TypeHandle::new().with_parameters(parameters)),
        );
    }

    async fn define(self: &mut Arc<Self>, path: TypeName, ty: &Located<Type>) {
        log::info!("define type {}", path);
        let converted = self.convert(ty).await;
        if let Some(handle) = self.types.lock().await.get_mut(&path) {
            if let Some(inner) = &*converted.ty.lock().await {
                handle.define(inner.clone()).await;
            }
        }
    }

    async fn define_function(self: &mut Arc<Self>, path: TypeName, func: Arc<dyn Function>) {
        log::info!("define function {}", path);
        let runtime_type = Located::new(
            RuntimeType::Primordial(PrimordialType::Function(path.clone(), func.clone())),
            0..0,
        );

        if let Some(handle) = self.types.lock().await.get_mut(&path) {
            handle.define(Arc::new(runtime_type)).await;
        }

        //self.types.lock().await.insert(
        //path,
        //Arc::new(runtime_type),
        //);
    }

    fn convert<'c>(
        self: &'c Arc<Self>,
        ty: &'c Located<Type>,
    ) -> Pin<Box<dyn Future<Output = Arc<TypeHandle>> + 'c>> {
        match &**ty {
            Type::Anything => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(RuntimeType::Anything, ty.location()))
                        .await,
                )
            }),
            Type::Ref(inner, arguments) => Box::pin(async move {
                let primary_type = self.types.lock().await[&(inner.clone().into_inner())].clone();

                if arguments.is_empty() {
                    primary_type
                } else {
                    let parameter_names = primary_type.parameters();

                    if parameter_names.len() != arguments.len() {
                        todo!("argument mismatch")
                    }

                    let mut bindings = Bindings::new();

                    for (name, arg) in parameter_names.iter().zip(arguments.iter()) {
                        bindings.bind(name.clone().into_inner(), self.convert(arg).await)
                    }

                    Arc::new(
                        TypeHandle::new()
                            .with(Located::new(
                                RuntimeType::Bound(primary_type, bindings),
                                ty.location(),
                            ))
                            .await,
                    )
                }
            }),
            Type::Parameter(name) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Argument(name.clone()),
                            name.location(),
                        ))
                        .await,
                )
            }),
            Type::Const(inner) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Const(inner.clone()),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::Object(inner) => Box::pin(async move {
                let mut fields = Vec::new();

                for f in inner.fields().iter() {
                    fields.push(Arc::new(Located::new(
                        RuntimeField {
                            name: f.name().clone(),
                            ty: self.convert(f.ty()).await,
                        },
                        ty.location(),
                    )));
                }

                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Object(RuntimeObjectType { fields }),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::Expr(inner) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Expr(Arc::new(inner.clone())),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::Join(lhs, rhs) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Join(
                                self.convert(&**lhs).await,
                                self.convert(&**rhs).await,
                            ),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::Meet(lhs, rhs) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Meet(
                                self.convert(&**lhs).await,
                                self.convert(&**rhs).await,
                            ),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::Refinement(primary, refinement) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::Refinement(
                                self.convert(&**primary).await,
                                self.convert(&**refinement).await,
                            ),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::List(inner) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::List(self.convert(inner).await),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            Type::MemberQualifier(qualifier, ty) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(
                            RuntimeType::MemberQualifier(qualifier.clone(), self.convert(ty).await),
                            qualifier.location(),
                        ))
                        .await,
                )
            }),
            Type::Nothing => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(RuntimeType::Nothing, ty.location()))
                        .await,
                )
            }),
        }
    }
}

pub enum RuntimeType {
    Anything,
    Primordial(PrimordialType),
    Bound(Arc<TypeHandle>, Bindings),
    Argument(Located<String>),
    Const(Located<Value>),
    Object(RuntimeObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Arc<TypeHandle>, Arc<TypeHandle>),
    Meet(Arc<TypeHandle>, Arc<TypeHandle>),
    Refinement(Arc<TypeHandle>, Arc<TypeHandle>),
    List(Arc<TypeHandle>),
    MemberQualifier(Located<MemberQualifier>, Arc<TypeHandle>),
    Nothing,
}

impl Debug for RuntimeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeType::Anything => write!(f, "anything"),
            RuntimeType::Primordial(inner) => write!(f, "{:?}", inner),
            RuntimeType::Const(inner) => write!(f, "{:?}", inner),
            RuntimeType::Object(inner) => write!(f, "{:?}", inner),
            RuntimeType::Expr(inner) => write!(f, "$({:?})", inner),
            RuntimeType::Join(lhs, rhs) => write!(f, "({:?} || {:?})", lhs, rhs),
            RuntimeType::Meet(lhs, rhs) => write!(f, "({:?} && {:?})", lhs, rhs),
            RuntimeType::Refinement(primary, refinement) => {
                write!(f, "{:?}({:?})", primary, refinement)
            }
            RuntimeType::List(inner) => write!(f, "[{:?}]", inner),
            RuntimeType::MemberQualifier(qualifier, ty) => write!(f, "{:?}::{:?}", qualifier, ty),
            RuntimeType::Argument(name) => write!(f, "{:?}", name),
            RuntimeType::Bound(primary, bindings) => write!(f, "{:?}<{:?}>", primary, bindings),
            RuntimeType::Nothing => write!(f, "nothing"),
        }
    }
}

#[derive(Default, Debug)]
pub struct Bindings {
    bindings: HashMap<String, Arc<TypeHandle>>,
}

impl Bindings {
    pub fn new() -> Self {
        Self {
            bindings: Default::default(),
        }
    }

    pub fn bind(&mut self, name: String, ty: Arc<TypeHandle>) {
        self.bindings.insert(name, ty);
    }

    pub fn get(&self, name: &String) -> Option<Arc<TypeHandle>> {
        self.bindings.get(name).cloned()
    }
}

impl Located<RuntimeType> {
    pub fn to_html(&self) -> Pin<Box<dyn Future<Output = String> + '_>> {
        match &**self {
            RuntimeType::Anything => Box::pin(async move { "<b>anything</b>".into() }),
            RuntimeType::Primordial(primordial) => Box::pin(async move {
                match primordial {
                    PrimordialType::Integer => "<b>integer</b>".into(),
                    PrimordialType::Decimal => "<b>decimal</b>".into(),
                    PrimordialType::Boolean => "<b>boolean</b>".into(),
                    PrimordialType::String => "<b>string</b>".into(),
                    PrimordialType::Function(name, _) => {
                        format!("<b>{}(...)</b>", name)
                    }
                }
            }),
            RuntimeType::Bound(_, _) => Box::pin(async move { "bound".into() }),
            RuntimeType::Argument(_) => Box::pin(async move { "argument".into() }),
            RuntimeType::Const(_) => Box::pin(async move { "const".into() }),
            RuntimeType::Object(inner) => Box::pin(async move { inner.to_html().await }),
            RuntimeType::Expr(_) => Box::pin(async move { "expr".into() }),
            RuntimeType::Join(lhs, rhs) => Box::pin(async move {
                format!(
                    "{} || {}",
                    lhs.ty().await.to_html().await,
                    rhs.ty().await.to_html().await
                )
            }),
            RuntimeType::Meet(lhs, rhs) => Box::pin(async move {
                format!(
                    "{} && {}",
                    lhs.ty().await.to_html().await,
                    rhs.ty().await.to_html().await
                )
            }),
            RuntimeType::Refinement(_, _) => Box::pin(async move { "refinement".into() }),
            RuntimeType::List(_) => Box::pin(async move { "list".into() }),
            RuntimeType::MemberQualifier(_, _) => {
                Box::pin(async move { "qualified-member".into() })
            }
            RuntimeType::Nothing => Box::pin(async move { "<b>nothing</b>".into() }),
        }
    }

    pub fn evaluate<'v>(
        self: &'v Arc<Self>,
        value: Arc<Mutex<Value>>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>> {
        match &***self {
            RuntimeType::Anything => Box::pin(ready(Ok(Some(value)))),
            RuntimeType::Argument(name) => Box::pin(async move {
                if let Some(bound) = bindings.get(&name.clone().into_inner()) {
                    let result = bound.evaluate(value.clone(), bindings).await?;
                    let mut locked_value = value.lock().await;
                    if result.is_some() {
                        locked_value.note(self.clone(), true);
                        Ok(Some(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                } else {
                    let mut locked_value = value.lock().await;
                    locked_value.note(self.clone(), false);
                    Ok(None)
                }
            }),
            RuntimeType::Primordial(inner) => match inner {
                PrimordialType::Integer => Box::pin(async move {
                    let mut locked_value = value.lock().await;
                    if locked_value.is_integer() {
                        locked_value.note(self.clone(), true);
                        Ok(Some(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                }),
                PrimordialType::Decimal => Box::pin(async move {
                    let mut locked_value = value.lock().await;
                    if locked_value.is_decimal() {
                        locked_value.note(self.clone(), true);
                        Ok(Some(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                }),
                PrimordialType::Boolean => Box::pin(async move {
                    let mut locked_value = value.lock().await;

                    if locked_value.is_boolean() {
                        locked_value.note(self.clone(), true);
                        Ok(Some(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                }),
                PrimordialType::String => Box::pin(async move {
                    let mut locked_value = value.lock().await;
                    if locked_value.is_string() {
                        locked_value.note(self.clone(), true);
                        Ok(Some(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                }),
                PrimordialType::Function(name, func) => Box::pin(async move {
                    let mut locked_value = value.lock().await;
                    let mut result = func.call(&*locked_value).await;
                    if let Ok(transform) = result {
                        let transform = Arc::new(Mutex::new(transform));
                        locked_value.transform(name.clone(), transform.clone());
                        Ok(Some(transform))
                    } else {
                        Ok(None)
                    }
                }),
            },
            RuntimeType::Const(inner) => Box::pin(async move {
                let mut locked_value = value.lock().await;
                if (**inner).eq(&*locked_value) {
                    locked_value.note(self.clone(), true);
                    Ok(Some(value.clone()))
                } else {
                    locked_value.note(self.clone(), false);
                    Ok(None)
                }
            }),
            RuntimeType::Object(inner) => Box::pin(async move {
                let mut locked_value = value.lock().await;
                if locked_value.is_object() {
                    let mut obj = locked_value.try_get_object();
                    let mut mismatch = vec![];
                    if let Some(obj) = obj {
                        for field in &inner.fields {
                            if let Some(field_value) = obj.get(field.name.clone().into_inner()) {
                                let result = field.ty.evaluate(field_value, bindings).await?;
                                if result.is_none() {
                                    locked_value.note(self.clone(), false);
                                    return Ok(None);
                                }
                            } else {
                                mismatch.push(field);
                                break;
                            }
                        }
                        if !mismatch.is_empty() {
                            for e in mismatch {
                                locked_value.note(e.clone(), false);
                            }
                            locked_value.note(self.clone(), false);
                            Ok(None)
                        } else {
                            locked_value.note(self.clone(), true);
                            Ok(Some(value.clone()))
                        }
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                } else {
                    locked_value.note(self.clone(), false);
                    Ok(None)
                }
            }),
            RuntimeType::Expr(expr) => Box::pin(async move {
                let result = expr.evaluate(value.clone()).await?;
                let mut locked_value = value.lock().await;
                let locked_result = result.lock().await;
                if let Some(true) = locked_result.try_get_boolean() {
                    locked_value.note(self.clone(), true);
                    Ok(Some(value.clone()))
                } else {
                    locked_value.note(self.clone(), false);
                    Ok(None)
                }
            }),
            RuntimeType::Join(lhs, rhs) => Box::pin(async move {
                let lhs_result = lhs.evaluate(value.clone(), bindings).await?;
                let rhs_result = rhs.evaluate(value.clone(), bindings).await?;

                let mut locked_value = value.lock().await;
                if lhs_result.is_some() {
                    locked_value.note(lhs.clone(), true);
                }

                if rhs_result.is_some() {
                    locked_value.note(rhs.clone(), true);
                }

                if rhs_result.is_some() || lhs_result.is_some() {
                    return Ok(Some(value.clone()));
                }

                Ok(None)
            }),
            RuntimeType::Meet(lhs, rhs) => Box::pin(async move {
                let lhs_result = lhs.evaluate(value.clone(), bindings).await?;
                let rhs_result = rhs.evaluate(value.clone(), bindings).await?;

                let mut locked_value = value.lock().await;
                if lhs_result.is_some() {
                    locked_value.note(lhs.clone(), true);
                }

                if rhs_result.is_some() {
                    locked_value.note(rhs.clone(), true);
                }

                if rhs_result.is_some() && lhs_result.is_some() {
                    return Ok(Some(value.clone()));
                }

                Ok(None)
            }),
            RuntimeType::Refinement(primary, refinement) => Box::pin(async move {
                let mut result = primary.evaluate(value.clone(), bindings).await?;
                if let Some(primary_value) = result {
                    let result = refinement.evaluate(primary_value.clone(), bindings).await?;
                    if result.is_some() {
                        Ok(Some(value.clone()))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }),
            RuntimeType::List(_) => todo!(),
            RuntimeType::MemberQualifier(qualifier, ty) => Box::pin(async move {
                let mut locked_value = value.lock().await;
                match &**qualifier {
                    MemberQualifier::All => {
                        if let Some(list) = locked_value.try_get_list() {
                            for e in list {
                                let result = ty.evaluate(e.clone(), bindings).await?;
                                if result.is_none() {
                                    locked_value.note(self.clone(), false);
                                    return Ok(None);
                                }
                            }
                            locked_value.note(self.clone(), true);
                            return Ok(Some(value.clone()));
                        }
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                    MemberQualifier::Any => {
                        if let Some(list) = locked_value.try_get_list() {
                            for e in list {
                                let result = ty.evaluate(e.clone(), bindings).await?;
                                if result.is_some() {
                                    locked_value.note(self.clone(), true);
                                    return Ok(Some(value.clone()));
                                }
                            }
                            locked_value.note(self.clone(), false);
                            return Ok(None);
                        }
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                    MemberQualifier::N(expected_n) => {
                        let expected_n = expected_n.clone().into_inner();
                        let mut n = 0;
                        if let Some(list) = locked_value.try_get_list() {
                            for e in list {
                                let result = ty.evaluate(e.clone(), bindings).await?;
                                if result.is_some() {
                                    n += 1;
                                    if n >= expected_n {
                                        locked_value.note(self.clone(), true);
                                        return Ok(Some(value.clone()));
                                    }
                                }
                            }
                        }
                        locked_value.note(self.clone(), false);
                        Ok(None)
                    }
                }
            }),
            RuntimeType::Bound(primary, bindings) => {
                Box::pin(async move { primary.evaluate(value, bindings).await })
            }
            RuntimeType::Nothing => Box::pin(ready(Ok(None))),
        }
    }
}

#[derive(Debug)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(TypeName, Arc<dyn Function>),
}

#[derive(Debug)]
pub struct RuntimeObjectType {
    fields: Vec<Arc<Located<RuntimeField>>>,
}

impl RuntimeObjectType {
    pub async fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<div>{");
        for f in &self.fields {
            html.push_str("<div style='padding-left: 1em'>");
            html.push_str(
                format!(
                    "{}: {},",
                    f.name.clone().into_inner(),
                    f.ty.ty().await.to_html().await
                )
                .as_str(),
            );
            html.push_str("</div>");
        }
        html.push_str("}</div>");

        html
    }
}

#[derive(Debug)]
pub struct RuntimeField {
    name: Located<String>,
    ty: Arc<TypeHandle>,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::runtime::sources::{Directory, Ephemeral};
    use serde_json::json;
    use std::default::Default;
    use std::env;
    use std::iter::once;

    #[actix_rt::test]
    async fn ephemeral_sources() {
        let src = Ephemeral::new("foo::bar", "type bob");

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.link().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn link_test_data() {
        let src = Directory::new(env::current_dir().unwrap().join("test-data"));

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.link().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn evaluate_function() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
            # Single-line comment, yay
            type signed-thing = {
                digest: sigstore::SHA256(
                    n<1>::{
                        apiVersion: "0.0.1",
                        spec: {
                            signature: {
                                publicKey: {
                                    content: base64::Base64(
                                        x509::PEM( n<1>::{
                                            version: 2,
                                            extensions: n<1>::{
                                                subjectAlternativeName: n<1>::{
                                                    rfc822: "bob@mcwhirter.org",
                                                }
                                            }
                                        } )
                                    )
                                }
                            }
                        }
                    }
                )
            }
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());
        let runtime = builder.link().await.unwrap();

        let value = json!(
            {
                "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
            }
        );

        let result = runtime.evaluate("foo::bar::signed-thing", value).await;

        assert!(matches!(result, Ok(Some(_)),))
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_literals() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        type named<name> = {
            name: name
        }

        type jim = named<"Jim">
        type bob = named<"Bob">

        type folks = jim || bob

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.link().await.unwrap();

        let good_bob = json!(
            {
                "name": "Bob",
                "age": 52,
            }
        );

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 52,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_types() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
                type named<name> = {
                    name: name
                }

                type jim = named<int>
                type bob = named<"Bob">

                type folks = jim || bob

                "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.link().await.unwrap();

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 52,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        type bob = {
            name: "Bob",
            age: $(self > 48),
        }

        type jim = {
            name: "Jim",
            age: $(self > 52),
        }

        type folks = bob || jim

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.link().await.unwrap();

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 49,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Jim",
                            "age": 49,
                        }
                    )
                )
                .await,
            Ok(None)
        ));

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Bob",
                            "age": 42,
                        }
                    )
                )
                .await,
            Ok(None)
        ));

        assert!(matches!(
            runtime
                .evaluate(
                    "foo::bar::folks",
                    json!(
                        {
                            "name": "Jim",
                            "age": 53,
                        }
                    )
                )
                .await,
            Ok(Some(_))
        ));
    }
}

pub mod linker;
pub mod sources;

use crate::core::{Function, FunctionError};
use crate::lang::expr::Expr;
use crate::lang::ty::{MemberQualifier, PackagePath, Type, TypeName};
use crate::lang::{CompilationUnit, Located, ParserError, ParserInput, PolicyParser, Source};
use crate::package::Package;
use crate::runtime::linker::Linker;
use crate::value::{Value as RuntimeValue, Value};
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

#[derive(Debug)]
pub enum BuildError {
    TypeNotFound,
    Parser(ParserError),
}

impl From<ParserError> for BuildError {
    fn from(inner: ParserError) -> Self {
        Self::Parser(inner)
    }
}

#[derive(Default)]
pub struct Builder {
    units: Vec<CompilationUnit>,
    packages: HashMap<PackagePath, Package>,
}

impl Builder {
    pub fn new() -> Self {
        let mut builder = Self {
            units: Default::default(),
            packages: Default::default(),
        };
        builder.add_package(crate::core::sigstore::package());
        builder.add_package(crate::core::x509::package());
        builder.add_package(crate::core::base64::package());

        builder
    }

    pub fn build<'a, Iter, S, SrcIter>(&mut self, sources: SrcIter) -> Result<(), Vec<BuildError>>
        where
            Self: Sized,
            Iter: Iterator<Item=(ParserInput, <ParserError as Error<ParserInput>>::Span)> + 'a,
            S: Into<Stream<'a, ParserInput, <ParserError as Error<ParserInput>>::Span, Iter>>,
            SrcIter: Iterator<Item=(Source, S)>,
    {
        let mut errors = Vec::new();
        for (source, stream) in sources {
            let unit = PolicyParser::default().parse(source, stream);
            match unit {
                Ok(unit) => self.add_compilation_unit(unit),
                Err(err) => {
                    for e in err {
                        errors.push(e.into())
                    }
                }
            }
        }

        let mut core_units = Vec::new();

        for (_, pkg) in &self.packages {
            for (source, stream) in pkg.source_iter() {
                let unit = PolicyParser::default().parse(source, stream);
                match unit {
                    Ok(unit) => {
                        core_units.push(unit);
                    }
                    Err(err) => {
                        for e in err {
                            errors.push(e.into())
                        }
                    }
                }
            }
        }

        for unit in core_units {
            self.add_compilation_unit(unit);
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

    pub async fn link(self) -> Result<Arc<Runtime>, Vec<BuildError>> {
        Linker::new(self.units, self.packages).link().await
    }
}

#[derive(Default, Debug)]
pub struct EvaluationResult {
    value: Option<Arc<Mutex<Value>>>,
}

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

#[derive(Debug)]
pub enum RuntimeError {
    Lock,
    NoSuchType,
    Function(FunctionError),
}

pub struct Runtime {
    types: Mutex<HashMap<TypeName, Arc<TypeHandle>>>,
}

#[derive(Default, Debug)]
pub struct TypeHandle {
    ty: Mutex<Option<Arc<Located<RuntimeType>>>>,
}

impl TypeHandle {
    pub fn new() -> Self {
        Self {
            ty: Mutex::new(None),
        }
    }

    pub fn new_with(ty: Located<RuntimeType>) -> Self {
        Self {
            ty: Mutex::new(Some(Arc::new(ty))),
        }
    }

    async fn with(mut self, ty: Located<RuntimeType>) -> Self {
        self.define(Arc::new(ty)).await;
        self
    }

    async fn define(&self, ty: Arc<Located<RuntimeType>>) {
        self.ty.lock().await.replace(ty);
    }

    async fn ty(&self) -> Arc<Located<RuntimeType>> {
        self.ty.lock().await.as_ref().unwrap().clone()
    }

    async fn evaluate(
        &self,
        value: Arc<Mutex<RuntimeValue>>,
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

        Arc::new(Self {
            types: Mutex::new(initial_types),
        })
    }

    pub async fn evaluate(
        &self,
        path: String,
        value: RuntimeValue,
        bindings: &Bindings,
    ) -> Result<EvaluationResult, RuntimeError> {
        let value = Arc::new(Mutex::new(value));
        let path = TypeName::from(path);
        let ty = &self.types.lock().await[&path];
        let ty = ty.ty().await;
        println!(">>> TYPE");
        println!("{:?}", ty);
        println!("<<< TYPE");
        ty.evaluate(value, bindings).await
    }

    async fn declare(self: &mut Arc<Self>, path: TypeName) {
        self.types
            .lock()
            .await
            .insert(path, Arc::new(TypeHandle::new()));
    }

    async fn define(self: &mut Arc<Self>, path: TypeName, ty: &Located<Type>) {
        let converted = self.convert(ty).await;
        if let Some(handle) = self.types.lock().await.get_mut(&path) {
            if let Some(inner) = &*converted.ty.lock().await {
                handle.define(inner.clone()).await;
            }
        }
    }

    async fn define_function(self: &mut Arc<Self>, path: TypeName, func: Arc<dyn Function>) {
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
    ) -> Pin<Box<dyn Future<Output=Arc<TypeHandle>> + 'c>> {
        match &**ty {
            Type::Anything => Box::pin(async move {
                Arc::new(
                    TypeHandle::new()
                        .with(Located::new(RuntimeType::Anything, ty.location()))
                        .await,
                )
            }),
            Type::Ref(inner) => {
                Box::pin(
                    async move { self.types.lock().await[&(inner.clone().into_inner())].clone() },
                )
            }
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
    //Ref(Arc<Runtime>, Located<TypeName>),
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
            RuntimeType::Refinement(primary, refinement) => write!(f, "{:?}({:?})", primary, refinement),
            RuntimeType::List(inner) => write!(f, "[{:?}]", inner),
            RuntimeType::MemberQualifier(qualifier, ty) => write!(f, "{:?}::{:?}", qualifier, ty),
            RuntimeType::Argument(name) => write!(f, "{:?}", name),
            RuntimeType::Nothing => write!(f, "nothing"),
        }
    }
}

#[derive(Default)]
pub struct Bindings {
    bindings: HashMap<String, Arc<Mutex<RuntimeType>>>,
}

impl Bindings {
    pub fn new() -> Self {
        Self {
            bindings: Default::default(),
        }
    }
}

impl Located<RuntimeType> {
    pub fn evaluate<'v>(
        self: &'v Arc<Self>,
        value: Arc<Mutex<RuntimeValue>>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output=Result<EvaluationResult, RuntimeError>> + 'v>> {
        match &***self {
            RuntimeType::Anything => {
                return Box::pin(ready(Ok(EvaluationResult::new().set_value(value))));
            }
            RuntimeType::Argument(name) => Box::pin(async move { todo!() }),
            RuntimeType::Primordial(inner) => match inner {
                PrimordialType::Integer => {
                    return Box::pin(async move {
                        let mut locked_value = value.lock().await;
                        if locked_value.is_integer() {
                            locked_value.note(self.clone(), true);
                            Ok(EvaluationResult::new().set_value(value.clone()))
                        } else {
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                    });
                }
                PrimordialType::Decimal => {
                    return Box::pin(async move {
                        let mut locked_value = value.lock().await;
                        if locked_value.is_decimal() {
                            locked_value.note(self.clone(), true);
                            Ok(EvaluationResult::new().set_value(value.clone()))
                        } else {
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                    });
                }
                PrimordialType::Boolean => {
                    return Box::pin(async move {
                        let mut locked_value = value.lock().await;

                        if locked_value.is_boolean() {
                            locked_value.note(self.clone(), true);
                            Ok(EvaluationResult::new().set_value(value.clone()))
                        } else {
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                    });
                }
                PrimordialType::String => {
                    return Box::pin(async move {
                        let mut locked_value = value.lock().await;
                        if locked_value.is_string() {
                            locked_value.note(self.clone(), true);
                            Ok(EvaluationResult::new().set_value(value.clone()))
                        } else {
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                    });
                }
                PrimordialType::Function(name, func) => {
                    return Box::pin(async move {
                        let mut locked_value = value.lock().await;
                        let mut result = func.call(&*locked_value).await;
                        if let Ok(transform) = result {
                            let transform = Arc::new(Mutex::new(transform));
                            locked_value.transform(name.clone(), transform.clone());
                            println!("function {:?} succeed", name);
                            Ok(EvaluationResult::new().set_value(transform))
                        } else {
                            println!("core {:?} fail", name);
                            Ok(EvaluationResult::new())
                        }
                    });
                }
            },
            RuntimeType::Const(inner) => {
                return Box::pin(async move {
                    let mut locked_value = value.lock().await;
                    if (**inner).eq(&*locked_value) {
                        locked_value.note(self.clone(), true);
                        Ok(EvaluationResult::new().set_value(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(EvaluationResult::new())
                    }
                });
            }
            RuntimeType::Object(inner) => {
                return Box::pin(async move {
                    let mut locked_value = value.lock().await;
                    if locked_value.is_object() {
                        let mut obj = locked_value.try_get_object();
                        let mut mismatch = vec![];
                        if let Some(obj) = obj {
                            for field in &inner.fields {
                                if let Some(field_value) = obj.get(field.name.clone().into_inner())
                                {
                                    let result = field.ty.evaluate(field_value, bindings).await?;
                                    if result.value().is_none() {
                                        locked_value.note(self.clone(), false);
                                        return Ok(EvaluationResult::new());
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
                                Ok(EvaluationResult::new())
                            } else {
                                locked_value.note(self.clone(), true);
                                Ok(EvaluationResult::new().set_value(value.clone()))
                            }
                        } else {
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(EvaluationResult::new())
                    }
                });
            }
            RuntimeType::Expr(expr) => {
                return Box::pin(async move {
                    let result = expr.evaluate(value.clone()).await?;
                    let mut locked_value = value.lock().await;
                    let locked_result = result.lock().await;
                    if let Some(true) = locked_result.try_get_boolean() {
                        locked_value.note(self.clone(), true);
                        Ok(EvaluationResult::new().set_value(value.clone()))
                    } else {
                        locked_value.note(self.clone(), false);
                        Ok(EvaluationResult::new())
                    }
                });
            }
            RuntimeType::Join(lhs, rhs) => {
                return Box::pin(async move {
                    let lhs_result = lhs.evaluate(value.clone(), bindings).await?;
                    let rhs_result = rhs.evaluate(value.clone(), bindings).await?;

                    let mut locked_value = value.lock().await;
                    if lhs_result.value().is_some() {
                        locked_value.note(lhs.clone(), true);
                    }

                    if rhs_result.value().is_some() {
                        locked_value.note(rhs.clone(), true);
                    }

                    if rhs_result.value().is_some() || lhs_result.value().is_some() {
                        return Ok(EvaluationResult::new().set_value(value.clone()));
                    }

                    Ok(EvaluationResult::new())
                });
            }
            RuntimeType::Meet(lhs, rhs) => {
                return Box::pin(async move {
                    let lhs_result = lhs.evaluate(value.clone(), bindings).await?;
                    let rhs_result = rhs.evaluate(value.clone(), bindings).await?;

                    let mut locked_value = value.lock().await;
                    if lhs_result.value().is_some() {
                        locked_value.note(lhs.clone(), true);
                    }

                    if rhs_result.value().is_some() {
                        locked_value.note(rhs.clone(), true);
                    }

                    if rhs_result.value().is_some() && lhs_result.value().is_some() {
                        return Ok(EvaluationResult::new().set_value(value.clone()));
                    }

                    Ok(EvaluationResult::new())
                });
            }
            RuntimeType::Refinement(primary, refinement) => {
                return Box::pin(async move {
                    let mut result = primary.evaluate(value.clone(), bindings).await?;
                    if let Some(primary_value) = result.value() {
                        let result = refinement.evaluate(primary_value.clone(), bindings).await?;
                        if result.value().is_some() {
                            Ok(EvaluationResult::new().set_value(value.clone()))
                        } else {
                            Ok(EvaluationResult::new())
                        }
                    } else {
                        Ok(EvaluationResult::new())
                    }
                });
            }
            RuntimeType::List(_) => todo!(),
            RuntimeType::MemberQualifier(qualifier, ty) => {
                return Box::pin(async move {
                    println!("MEMBER {:?} {:?}", qualifier, ty);
                    let mut locked_value = value.lock().await;
                    match &**qualifier {
                        MemberQualifier::All => {
                            if let Some(list) = locked_value.try_get_list() {
                                for e in list {
                                    let result = ty.evaluate(e.clone(), bindings).await?;
                                    if !result.matches() {
                                        locked_value.note(self.clone(), false);
                                        return Ok(EvaluationResult::new());
                                    }
                                }
                                locked_value.note(self.clone(), true);
                                return Ok(EvaluationResult::new().set_value(value.clone()));
                            }
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                        MemberQualifier::Any => {
                            if let Some(list) = locked_value.try_get_list() {
                                for e in list {
                                    let result = ty.evaluate(e.clone(), bindings).await?;
                                    if result.matches() {
                                        locked_value.note(self.clone(), true);
                                        return Ok(EvaluationResult::new().set_value(value.clone()));
                                    }
                                }
                                locked_value.note(self.clone(), false);
                                return Ok(EvaluationResult::new());
                            }
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                        MemberQualifier::N(expected_n) => {
                            let expected_n = expected_n.clone().into_inner();
                            let mut n = 0;
                            if let Some(list) = locked_value.try_get_list() {
                                for e in list {
                                    //println!("TEST {}", e.lock().await.display().await);
                                    let result = ty.evaluate(e.clone(), bindings).await?;
                                    if result.matches() {
                                        n += 1;
                                        if n >= expected_n {
                                            locked_value.note(self.clone(), true);
                                            return Ok(
                                                EvaluationResult::new().set_value(value.clone())
                                            );
                                        }
                                    }
                                }
                            }
                            locked_value.note(self.clone(), false);
                            Ok(EvaluationResult::new())
                        }
                    }
                });
            }
            RuntimeType::Nothing => Box::pin(ready(Ok(EvaluationResult::new()))),
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

    #[test]
    fn ephemeral_sources() {
        let src = Ephemeral::new(
            PackagePath::from_parts(vec!["foo", "bar"]),
            "type bob".into(),
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.link();
    }

    #[test]
    fn link_test_data() {
        let src = Directory::new(env::current_dir().unwrap().join("test-data"));

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        println!("build {:?}", result);

        let result = builder.link();

        //println!("link {:?}", result);
    }

    #[actix_rt::test]
    async fn evaluate_function() {
        let src = Ephemeral::new(
            PackagePath::from_parts(vec!["foo", "bar"]),
            r#"
            # is this okay?
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
                                                    #rfc822: "bob@mcwhirter.org",
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
        "#
                .into(),
        );

        /*
               digest: sigstore::SHA256( {
                   apiVersion: "0.0.1",
               } ),

        */
        let mut builder = Builder::new();

        let result = builder.build(src.iter());
        println!("{:?}", result);
        let runtime = builder.link().await.unwrap();

        let value = json!(
            {
                "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
            }
        );

        let mut value = (&value).into();

        let result = runtime
            .evaluate("foo::bar::signed-thing".into(), value, &Default::default())
            .await;

        println!("{:?}", result);
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let src = Ephemeral::new(
            PackagePath::from_parts(vec!["foo", "bar"]),
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

        "#
                .into(),
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

        println!("{:?}", good_bob);

        let mut good_bob = (&good_bob).into();

        let result = runtime
            .evaluate("foo::bar::folks".into(), good_bob, &Default::default())
            .await;
        println!("{:?}", result);

        println!("{:?}", result.unwrap().value());
    }
}

pub mod sources;
pub mod linker;

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::{Future, ready};
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::ready;
use chumsky::{Error, Stream};
use crate::function::{Function, FunctionPackage};
use crate::lang::{CompilationUnit, Located, ParserError, ParserInput, PolicyParser, Source};
use crate::lang::expr::Expr;
use crate::lang::ty::{PackagePath, Type, TypeName};
use crate::value::{Value as RuntimeValue, Value};
use crate::runtime::linker::Linker;

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

pub struct Builder {
    units: Vec<CompilationUnit>,
    packages: HashMap<PackagePath, FunctionPackage>,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            units: Default::default(),
            packages: Default::default(),
        }
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
                Ok(unit) => {
                    self.add_compilation_unit(unit)
                }
                Err(err) => {
                    for e in err {
                        errors.push(
                            e.into()
                        )
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

    pub fn add_function_package(&mut self, path: PackagePath, package: FunctionPackage) {
        self.packages.insert(path, package);
    }

    pub fn link(self) -> Result<Arc<Runtime>, Vec<BuildError>> {
        Linker::new(self.units, self.packages).link()
    }
}

#[derive(Debug)]
pub struct EvaluationResult {
    value: Option<Value>,
}

impl EvaluationResult {
    pub fn new() -> Self {
        Self {
            value: None,
        }
    }

    pub fn set_value(mut self, value: Value) -> Self {
        self.value.replace(value);
        self
    }

    pub fn value(&self) -> &Option<Value> {
        &self.value
    }

    pub fn value_mut(&mut self) -> &mut Option<Value> {
        &mut self.value
    }

    pub fn matches(&self) -> bool {
        self.value.is_some()
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    NoSuchType(String),
}

pub struct Runtime {
    types: Mutex<HashMap<TypeName, Arc<Located<RuntimeType>>>>,
}

impl Runtime {
    pub(crate) fn new() -> Arc<Self> {
        let this = Arc::new(Self {
            types: Mutex::new(Default::default())
        });

        this.types.lock().unwrap().insert(
            TypeName::new("int".into()),
            Arc::new(Located::new(RuntimeType::Primordial(PrimordialType::Integer), 0..0)));

        this
    }

    pub async fn evaluate(&self, path: String, value: &mut RuntimeValue) -> Result<EvaluationResult, RuntimeError> {
        let path = TypeName::from(path);
        let ty = self.types.lock().unwrap()[&path].clone();
        ty.evaluate(value).await
    }

    fn define(self: &mut Arc<Self>, path: TypeName, ty: &Located<Type>) {
        println!("define {:?}", path.as_type_str());
        let converted = self.convert(ty);

        self.types.lock().unwrap().insert(
            path,
            Arc::new(converted),
        );
    }

    fn define_function(self: &mut Arc<Self>, path: TypeName, func: Arc<dyn Function>) {
        println!("define-func {:?}", path.as_type_str());

        let runtime_type = Located::new(RuntimeType::Primordial(
            PrimordialType::Function(
                path.clone(),
                func.clone()
            )
        ), 0..0);

        self.types.lock().unwrap().insert(
            path,
            Arc::new(runtime_type),
        );
    }

    fn convert(self: &Arc<Self>, ty: &Located<Type>) -> Located<RuntimeType> {
        match &**ty {
            Type::Anything => {
                Located::new(RuntimeType::Anything, ty.location())
            }
            Type::Ref(inner) => {
                Located::new(
                    RuntimeType::Ref(self.clone(), inner.clone()),
                    ty.location(),
                )
            }
            Type::Const(inner) => {
                Located::new(
                    RuntimeType::Const(inner.clone()),
                    ty.location(),
                )
            }
            Type::Object(inner) => {
                Located::new(
                    RuntimeType::Object(
                        RuntimeObjectType {
                            fields: inner.fields().iter().map(|f| {
                                Arc::new(Located::new(
                                    RuntimeField {
                                        name: f.name().clone(),
                                        ty: Arc::new(self.convert(f.ty())),
                                    },
                                    ty.location(),
                                ))
                            }).collect()
                        }
                    ),
                    ty.location(),
                )
            }
            Type::Expr(inner) => {
                Located::new(
                    RuntimeType::Expr(Arc::new(inner.clone())),
                    ty.location(),
                )
            }
            Type::Join(lhs, rhs) => {
                Located::new(
                    RuntimeType::Join(
                        Arc::new(self.convert(&**lhs)),
                        Arc::new(self.convert(&**rhs)),
                    ),
                    ty.location(),
                )
            }
            Type::Meet(lhs, rhs) => {
                Located::new(
                    RuntimeType::Meet(
                        Arc::new(self.convert(&**lhs)),
                        Arc::new(self.convert(&**rhs)),
                    ),
                    ty.location(),
                )
            }
            Type::Functional(fn_name, inner) => {
                println!("lang {:?} {:?}", fn_name, inner);
                Located::new(
                    RuntimeType::Functional(
                        self.clone(),
                        fn_name.clone(),
                        inner.as_ref().map(|e| Arc::new(self.convert(&e)))),
                    ty.location(),
                )
            }
            Type::List(inner) => {
                Located::new(
                    RuntimeType::List(Box::new(self.convert(inner))),
                    ty.location(),
                )
            }
            Type::Nothing => Located::new(RuntimeType::Nothing, ty.location())
        }
    }
}

pub enum RuntimeType {
    Anything,
    Primordial(PrimordialType),
    Ref(Arc<Runtime>, Located<TypeName>),
    Const(Located<Value>),
    Object(RuntimeObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Arc<Located<RuntimeType>>, Arc<Located<RuntimeType>>),
    Meet(Arc<Located<RuntimeType>>, Arc<Located<RuntimeType>>),
    Functional(Arc<Runtime>, Located<TypeName>, Option<Arc<Located<RuntimeType>>>),
    List(Box<Located<RuntimeType>>),
    Nothing,
}

impl Debug for RuntimeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeType::Anything => write!(f, "anything"),
            RuntimeType::Primordial(inner) => write!(f, "{:?}", inner),
            RuntimeType::Ref(_, name) => write!(f, "{}", name.as_type_str()),
            RuntimeType::Const(inner) => write!(f, "{:?}", inner),
            RuntimeType::Object(inner) => write!(f, "{:?}", inner),
            RuntimeType::Expr(inner) => write!(f, "$({:?})", inner),
            RuntimeType::Join(lhs, rhs) => write!(f, "({:?} || {:?})", lhs, rhs),
            RuntimeType::Meet(lhs, rhs) => write!(f, "({:?} && {:?})", lhs, rhs),
            RuntimeType::Functional(_, name, ty) => write!(f, "{:?}({:?})", name, ty),
            RuntimeType::List(inner) => write!(f, "[{:?}]", inner),
            RuntimeType::Nothing => write!(f, "nothing"),
        }
    }
}

impl Located<RuntimeType> {
    pub fn evaluate<'v>(self: &'v Arc<Self>, value: &'v mut RuntimeValue) -> Pin<Box<dyn Future<Output=Result<EvaluationResult, RuntimeError>> + 'v>> {
        println!("eval self {:?}", self);
        println!("vs");
        println!("obj {:?}", value);
        println!("");
        match &***self {
            RuntimeType::Anything => {
                return Box::pin(
                    ready(
                        Ok(EvaluationResult::new().set_value(value.clone()))
                    )
                );
            }
            RuntimeType::Primordial(inner) => {
                println!("primordial");
                match inner {
                    PrimordialType::Integer => {
                        if value.is_integer() {
                            println!("prim A");
                            value.note(self.clone(), true);
                            return Box::pin(ready(Ok(EvaluationResult::new().set_value(value.clone()))));
                        } else {
                            println!("prim B");
                            value.note(self.clone(), false);
                            return Box::pin(ready(Ok(EvaluationResult::new())));
                        }
                    }
                    PrimordialType::Decimal => {
                        if value.is_decimal() {
                            println!("prim C");
                            value.note(self.clone(), true);
                            return Box::pin(ready(Ok(EvaluationResult::new().set_value(value.clone()))));
                        } else {
                            println!("prim D");
                            value.note(self.clone(), false);
                            return Box::pin(ready(Ok(EvaluationResult::new())));
                        }
                    }
                    PrimordialType::Boolean => {
                        if value.is_boolean() {
                            println!("prim E");
                            value.note(self.clone(), true);
                            return Box::pin(ready(Ok(EvaluationResult::new().set_value(value.clone()))));
                        } else {
                            println!("prim F");
                            value.note(self.clone(), false);
                            return Box::pin(ready(Ok(EvaluationResult::new())));
                        }
                    }
                    PrimordialType::String => {
                        if value.is_string() {
                            println!("prim G");
                            value.note(self.clone(), true);
                            return Box::pin(ready(Ok(EvaluationResult::new().set_value(value.clone()))));
                        } else {
                            println!("prim H");
                            value.note(self.clone(), false);
                            return Box::pin(ready(Ok(EvaluationResult::new())));
                        }
                    }
                    PrimordialType::Function(name, func) => {
                        return Box::pin(async move {
                            let mut result = func.call(value).await;
                            if let Ok(transform) = result {
                                value.transform( name.clone(), transform.clone() );
                                println!("fn call -> {:?}", transform);
                                return Ok(EvaluationResult::new().set_value(transform.clone()));
                            } else {
                                println!("fn call failed?");
                                return Ok(EvaluationResult::new());
                            }
                        });
                    }
                }
            }
            RuntimeType::Ref(runtime, path) => {
                return Box::pin(
                    async move {
                        let result = runtime.evaluate(path.as_type_str(), value).await;
                        println!("REF RESULT {:?}", result);
                        result
                    }
                );
            }
            RuntimeType::Const(inner) => {
                println!("const");
                if (**inner).eq(value) {
                    value.note(self.clone(), true);
                    println!("eq");
                    return Box::pin(ready(Ok(EvaluationResult::new().set_value(value.clone()))));
                } else {
                    println!("not-eq");
                    value.note(self.clone(), false);
                    return Box::pin(ready(Ok(EvaluationResult::new())));
                }
            }
            RuntimeType::Object(inner) => {
                return Box::pin(async move {
                    if value.is_object() {
                        let mut obj = value.try_get_object();
                        let mut mismatch = vec![];
                        if let Some(obj) = obj {
                            for field in &inner.fields {
                                println!("check field {:?}", field);
                                if let Some(field_value) = obj.get(field.name.clone().into_inner()) {
                                    let result = field.ty.evaluate(field_value).await?;
                                    println!("field result {:?}", result);
                                    if result.value().is_none() {
                                        value.note(self.clone(), false);
                                        return Ok(EvaluationResult::new());
                                    }
                                } else {
                                    mismatch.push(field);
                                    break;
                                }
                            }
                            if !mismatch.is_empty() {
                                println!("mismatch obj");
                                for e in mismatch {
                                    value.note(e.clone(), false);
                                }
                                value.note(self.clone(), false);
                                return Ok(EvaluationResult::new());
                            } else {
                                println!("match obj");
                                value.note(self.clone(), true);
                                return Ok(EvaluationResult::new().set_value(value.clone()));
                            }
                        } else {
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new());
                        }
                    } else {
                        value.note(self.clone(), false);
                        return Ok(EvaluationResult::new());
                    }
                });
            }
            RuntimeType::Expr(expr) => {
                return Box::pin(
                    async move {
                        let result = expr.evaluate(value)?;
                        if let Some(true) = result.try_get_boolean() {
                            value.note(self.clone(), true);
                            return Ok(EvaluationResult::new().set_value(value.clone()));
                        } else {
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new());
                        }
                    });
            }
            RuntimeType::Join(lhs, rhs) => {
                return Box::pin(async move {
                    let lhs_result = lhs.evaluate(value).await?;
                    let rhs_result = rhs.evaluate(value).await?;

                    if lhs_result.value().is_some() {
                        value.note(lhs.clone(), true);
                    }

                    if rhs_result.value().is_some() {
                        value.note(rhs.clone(), true);
                    }

                    if rhs_result.value().is_some() || lhs_result.value().is_some() {
                        return Ok(EvaluationResult::new().set_value(value.clone()));
                    }

                    return Ok(EvaluationResult::new());
                });
            }
            RuntimeType::Meet(lhs, rhs) => {
                return Box::pin(async move {
                    let lhs_result = lhs.evaluate(value).await?;
                    let rhs_result = rhs.evaluate(value).await?;

                    if lhs_result.value().is_some() {
                        value.note(lhs.clone(), true);
                    }

                    if rhs_result.value().is_some() {
                        value.note(rhs.clone(), true);
                    }

                    if rhs_result.value().is_some() && lhs_result.value().is_some() {
                        return Ok(EvaluationResult::new().set_value(value.clone()));
                    }

                    return Ok(EvaluationResult::new());
                });
            }
            RuntimeType::Functional(runtime, path, ty) => {
                return Box::pin(
                    async move {
                        let mut result = runtime.evaluate(path.as_type_str(), value).await?;
                        println!("functional call result: {:?}", result);
                        if let Some(fn_value) = &mut result.value_mut().as_mut() {
                            if let Some(ty) = ty {
                                println!("inner ty check {:?}", ty);
                                let result = ty.evaluate(fn_value).await?;
                                if result.value().is_some() {
                                    println!("ITC A");
                                    Ok(EvaluationResult::new().set_value(value.clone()))
                                } else {
                                    println!("ITC B");
                                    Ok(EvaluationResult::new())
                                }
                            } else {
                                println!("no inner ty check");
                                Ok(EvaluationResult::new().set_value(value.clone()))
                            }
                        } else {
                            println!("failed fncall");
                            return Ok(EvaluationResult::new());
                        }
                    }
                );
            }
            RuntimeType::List(_) => {}
            RuntimeType::Nothing => {}
        }

        println!("shit");
        Box::pin(ready(Ok(EvaluationResult::new())))
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
    ty: Arc<Located<RuntimeType>>,
}

#[cfg(test)]
mod test {
    use std::env;
    use std::iter::once;
    use serde_json::json;
    use super::*;
    use crate::runtime::sources::{Directory, Ephemeral};

    #[test]
    fn ephemeral_sources() {
        let src = Ephemeral::new(PackagePath::from_parts(vec!["foo", "bar"]), "type bob".into());

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        println!("build {:?}", result);

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
        let src = Ephemeral::new(PackagePath::from_parts(vec!["foo", "bar"]), r#"
            type signed-thing = {
                digest: sigstore::SHA256( {
                    apiVersion: "0.0.1",
                } ),
            }
        "#.into());

        let mut builder = Builder::new();
        builder.add_function_package(PackagePath::from_parts(vec!["sigstore"]), crate::function::sigstore::package());
        let result = builder.build(src.iter());
        println!("{:?}", result);
        let runtime = builder.link().unwrap();

        let value = json!(
            {
                "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
            }
        );

        let mut value = (&value).into();

        let result = runtime.evaluate("foo::bar::signed-thing".into(), &mut value).await;

        println!("{:?}", result);
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let src = Ephemeral::new(PackagePath::from_parts(vec!["foo", "bar"]), r#"
        type bob = {
            name: "Bob",
            age: $(self > 48),
        }

        type jim = {
            name: "Jim",
            age: $(self > 52),
        }

        type folks = bob || jim

        "#.into());

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.link().unwrap();

        let good_bob = json!(
            {
                "name": "Bob",
                "age": 52,
            }
        );

        println!("{:?}", good_bob);

        let mut good_bob = (&good_bob).into();

        let result = runtime.evaluate("foo::bar::folks".into(), &mut good_bob).await;
        println!("{:?}", result);

        println!("{:?}", good_bob);
    }
}
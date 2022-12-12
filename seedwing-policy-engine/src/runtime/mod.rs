pub mod sources;
pub mod linker;

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::sync::{Arc, Mutex};
use chumsky::{Error, Stream};
use crate::function::FunctionPackage;
use crate::lang::{CompilationUnit, Located, ParserError, ParserInput, PolicyParser, Source, TypePath};
use crate::lang::expr::Expr;
use crate::lang::ty::{FunctionName, Type, TypeName};
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
    packages: HashMap<String, FunctionPackage>,
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

    pub fn add_function_package(&mut self, path: String, package: FunctionPackage) {
        self.packages.insert( path, package );
    }

    pub fn link(self) -> Result<Arc<Runtime>, Vec<BuildError>> {
        Linker::new(self.units, self.packages).link()
    }
}

#[derive(Debug)]
pub struct EvaluationResult {
    matches: bool,
}

impl EvaluationResult {
    pub fn new() -> Self {
        Self {
            matches: false
        }
    }

    pub fn set_matches(mut self, matches: bool) -> Self {
        self.matches = matches;
        self
    }

    pub fn matches(&self) -> bool {
        self.matches
    }
}

#[derive(Debug)]
pub enum RuntimeError {
    NoSuchType(String),
}

pub struct Runtime {
    types: Mutex<HashMap<String, Arc<Located<RuntimeType>>>>,
}

impl Runtime {
    pub(crate) fn new() -> Arc<Self> {
        let this = Arc::new(Self {
            types: Mutex::new(Default::default())
        });

        this.types.lock().unwrap().insert(
            "int".into(),
            Arc::new(Located::new(RuntimeType::Primordial(PrimordialType::Integer), 0..0)));

        this
    }

    pub fn evaluate(&self, path: String, value: &mut RuntimeValue) -> Result<EvaluationResult, RuntimeError> {
        let ty = self.types.lock().unwrap()[&path].clone();

        ty.evaluate(value)
    }

    fn define(self: &mut Arc<Self>, path: TypePath, ty: &Located<Type>) {
        println!("define {:?}", path.as_path_str());
        let converted = self.convert(ty);

        self.types.lock().unwrap().insert(
            path.as_path_str(),
            Arc::new(converted),
            //self.convert(&Located::new(Type::Nothing, 0..0))
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
                Located::new(
                    RuntimeType::Functional(
                        fn_name.clone(),
                        //Box::new(self.convert(&*inner))),
                    inner.as_ref().map(|e| Arc::new( self.convert(&e) ))),
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
    Ref(Arc<Runtime>, Located<TypePath>),
    Const(Located<Value>),
    Object(RuntimeObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Arc<Located<RuntimeType>>, Arc<Located<RuntimeType>>),
    Meet(Arc<Located<RuntimeType>>, Arc<Located<RuntimeType>>),
    Functional(Located<FunctionName>, Option<Arc<Located<RuntimeType>>>),
    List(Box<Located<RuntimeType>>),
    Nothing,
}

impl Debug for RuntimeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeType::Anything => write!(f, "anything"),
            RuntimeType::Primordial(inner) => write!(f, "{:?}", inner),
            RuntimeType::Ref(_, name) => write!(f, "{}", name.as_path_str()),
            RuntimeType::Const(inner) => write!(f, "{:?}", inner),
            RuntimeType::Object(inner) => write!(f, "{:?}", inner),
            RuntimeType::Expr(inner) => write!(f, "$({:?})", inner),
            RuntimeType::Join(lhs, rhs) => write!(f, "({:?} || {:?})", lhs, rhs),
            RuntimeType::Meet(lhs, rhs) => write!(f, "({:?} && {:?})", lhs, rhs),
            RuntimeType::Functional(name, ty) => todo!(),
            RuntimeType::List(inner) => write!(f, "[{:?}]", inner),
            RuntimeType::Nothing => write!(f, "nothing"),
        }
    }
}

impl Located<RuntimeType> {
    pub fn evaluate(self: &Arc<Self>, value: &mut RuntimeValue) -> Result<EvaluationResult, RuntimeError> {
        println!("eval self {:?}", self);
        match &***self {
            RuntimeType::Anything => {
                return Ok(EvaluationResult::new().set_matches(true));
            }
            RuntimeType::Primordial(inner) => {
                println!("primordial");
                match inner {
                    PrimordialType::Integer => {
                        if value.is_integer() {
                            println!("prim A");
                            value.note(self.clone(), true);
                            return Ok(EvaluationResult::new().set_matches(true));
                        } else {
                            println!("prim B");
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new().set_matches(false));
                        }
                    }
                    PrimordialType::Decimal => {
                        if value.is_decimal() {
                            println!("prim C");
                            value.note(self.clone(), true);
                            return Ok(EvaluationResult::new().set_matches(true));
                        } else {
                            println!("prim D");
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new().set_matches(false));
                        }
                    }
                    PrimordialType::Boolean => {
                        if value.is_boolean() {
                            println!("prim E");
                            value.note(self.clone(), true);
                            return Ok(EvaluationResult::new().set_matches(true));
                        } else {
                            println!("prim F");
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new().set_matches(false));
                        }
                    }
                    PrimordialType::String => {
                        if value.is_string() {
                            println!("prim G");
                            value.note(self.clone(), true);
                            return Ok(EvaluationResult::new().set_matches(true));
                        } else {
                            println!("prim H");
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new().set_matches(false));
                        }
                    }
                }
            }
            RuntimeType::Ref(runtime, path) => {
                return runtime.evaluate(path.as_path_str(), value);
            }
            RuntimeType::Const(inner) => {

                if (**inner).eq( value ) {
                    value.note(self.clone(), true);
                    return Ok(EvaluationResult::new().set_matches(true));
                } else {
                    value.note(self.clone(), false);
                    return Ok(EvaluationResult::new().set_matches(false));
                }
            }
            RuntimeType::Object(inner) => {
                if value.is_object() {
                    let mut obj = value.try_get_object();
                    let mut mismatch = vec![];
                    if let Some(obj) = obj {
                        for field in &inner.fields {
                            println!("check field {:?}", field);
                            if let Some(field_value) = obj.get(field.name.clone().into_inner()) {
                                let result = field.ty.evaluate(field_value)?;
                                println!("field result {:?}", result);
                                if ! result.matches {
                                    value.note(self.clone(), false);
                                    return Ok(EvaluationResult::new().set_matches(false));
                                }
                            } else {
                                mismatch.push(field);
                                break;
                            }
                        }
                        if ! mismatch.is_empty() {
                            println!("mismatch obj");
                            for e in mismatch {
                                value.note( e.clone(), false );
                            }
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new().set_matches(false));
                        } else {
                            println!("match obj");
                            value.note(self.clone(), true);
                            return Ok(EvaluationResult::new().set_matches(true));
                        }
                    } else {
                        value.note(self.clone(), false);
                        return Ok(EvaluationResult::new().set_matches(false));
                    }
                } else {
                    value.note(self.clone(), false);
                    return Ok(EvaluationResult::new().set_matches(false));
                }
            }
            RuntimeType::Expr(expr) => {
                let result = expr.evaluate(value)?;
                if let Some(true) = result.try_get_boolean() {
                    value.note(self.clone(), true);
                    return Ok(EvaluationResult::new().set_matches(true));
                } else {
                    value.note(self.clone(), false);
                    return Ok(EvaluationResult::new().set_matches(false));
                }

            }
            RuntimeType::Join(lhs, rhs) => {
                let lhs_result = lhs.evaluate(value)?;
                let rhs_result = rhs.evaluate(value)?;

                if lhs_result.matches {
                    value.note( lhs.clone(), true );
                }

                if rhs_result.matches {
                    value.note( rhs.clone(), true );
                }

                if rhs_result.matches || lhs_result.matches {
                    return Ok(EvaluationResult::new().set_matches(true));
                }

                return Ok(EvaluationResult::new().set_matches(false));
            }
            RuntimeType::Meet(lhs, rhs) => {
                let lhs_result = lhs.evaluate(value)?;
                let rhs_result = rhs.evaluate(value)?;

                if lhs_result.matches {
                    value.note( lhs.clone(), true );
                }

                if rhs_result.matches {
                    value.note( rhs.clone(), true );
                }

                if rhs_result.matches && lhs_result.matches {
                    return Ok(EvaluationResult::new().set_matches(true));
                }

                return Ok(EvaluationResult::new().set_matches(false));
            }
            RuntimeType::Functional(_, _) => {}
            RuntimeType::List(_) => {}
            RuntimeType::Nothing => {}
        }

        println!("shit");
        Ok(EvaluationResult::new().set_matches(false))
    }
}

#[derive(Debug)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
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
        let src = Ephemeral::new("foo::bar".into(), "type bob".into());

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

    #[test]
    fn evaluate_matches() {
        let src = Ephemeral::new("foo::bar".into(), r#"
        type bob = {
            name: "Bob",
            age: $(self > 48),
        }

        type jim = {
            name: "Jim",
            age: $(self > 52),
        }

        type folks = bob || jim

        type signed = sigstore::SHA256

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

        let result = runtime.evaluate("foo::bar::folks".into(), &mut good_bob);
        println!("{:?}", result);

        println!("{:?}", good_bob);
    }
}
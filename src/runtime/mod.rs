mod sources;
mod linker;

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::mem;
use std::rc::Rc;
use chumsky::{Error, Stream};
use crate::lang::{CompilationUnit, Located, ParserError, ParserInput, PolicyParser, Source, TypePath};
use crate::lang::expr::Expr;
use crate::lang::ty::{FunctionName, Type, TypeName, Value};
use crate::value::Value as RuntimeValue;
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
}

impl Builder {
    fn new() -> Self {
        Self {
            units: Default::default(),
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
                    self.add(unit)
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

    fn add(&mut self, unit: CompilationUnit) {
        self.units.push(unit)
    }

    pub fn link(self) -> Result<Rc<Runtime>, Vec<BuildError>> {
        Linker::new(self.units).link()
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
}

#[derive(Debug)]
pub enum RuntimeError {
    NoSuchType(String),
}

pub struct Runtime {
    types: RefCell<HashMap<String, Rc<Located<RuntimeType>>>>,
}

impl Runtime {
    pub(crate) fn new() -> Rc<Self> {
        let this = Rc::new(Self {
            types: RefCell::new(Default::default())
        });

        this.types.borrow_mut().insert(
            "int".into(),
            Rc::new(Located::new(RuntimeType::Primordial(PrimordialType::Integer), 0..0)));

        this
    }

    pub fn evaluate(&self, path: String, value: &mut RuntimeValue) -> Result<EvaluationResult, RuntimeError> {
        let ty = self.types.borrow()[&path].clone();

        ty.evaluate(value)
    }

    fn define(self: &mut Rc<Self>, path: TypePath, ty: &Located<Type>) {
        println!("define {:?}", path.as_path_str());
        let converted = self.convert(ty);

        self.types.borrow_mut().insert(
            path.as_path_str(),
            Rc::new(converted),
            //self.convert(&Located::new(Type::Nothing, 0..0))
        );
    }

    fn convert(self: &Rc<Self>, ty: &Located<Type>) -> Located<RuntimeType> {
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
                                Located::new(
                                    RuntimeField {
                                        name: f.name().clone(),
                                        ty: Rc::new(self.convert(f.ty())),
                                    },
                                    ty.location(),
                                )
                            }).collect()
                        }
                    ),
                    ty.location(),
                )
            }
            Type::Expr(inner) => {
                Located::new(
                    RuntimeType::Expr(inner.clone()),
                    ty.location(),
                )
            }
            Type::Join(lhs, rhs) => {
                Located::new(
                    RuntimeType::Join(
                        Rc::new(self.convert(&**lhs)),
                        Rc::new(self.convert(&**rhs)),
                    ),
                    ty.location(),
                )
            }
            Type::Meet(lhs, rhs) => {
                Located::new(
                    RuntimeType::Meet(
                        Box::new(self.convert(&**lhs)),
                        Box::new(self.convert(&**rhs)),
                    ),
                    ty.location(),
                )
            }
            Type::Functional(fn_name, inner) => {
                Located::new(
                    RuntimeType::Functional(
                        fn_name.clone(),
                        Box::new(self.convert(&*inner))),
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
    Ref(Rc<Runtime>, Located<TypePath>),
    Const(Located<Value>),
    Object(RuntimeObjectType),
    Expr(Located<Expr>),
    Join(Rc<Located<RuntimeType>>, Rc<Located<RuntimeType>>),
    Meet(Box<Located<RuntimeType>>, Box<Located<RuntimeType>>),
    Functional(Located<FunctionName>, Box<Located<RuntimeType>>),
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
    pub fn evaluate(self: &Rc<Self>, value: &mut RuntimeValue) -> Result<EvaluationResult, RuntimeError> {
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
                match &**inner {
                    Value::Integer(inner) => {}
                    Value::Decimal(inner) => {}
                    Value::String(inner) => {
                        if let Some(str_value) = value.try_get_string() {
                            if str_value == *inner {
                                value.note(self.clone(), true);
                                return Ok(EvaluationResult::new().set_matches(true));
                            } else {
                                value.note(self.clone(), false);
                                return Ok(EvaluationResult::new().set_matches(false));
                            }
                        } else {
                            value.note(self.clone(), false);
                            return Ok(EvaluationResult::new().set_matches(false));
                        }
                    }
                    Value::Boolean(inner) => {}
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
                            //for e in mismatch {
                                //value.note( e.ty.clone(), false );
                            //}
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
            RuntimeType::Expr(_) => {}
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
            RuntimeType::Meet(_, _) => {}
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
    fields: Vec<Located<RuntimeField>>,
}

#[derive(Debug)]
pub struct RuntimeField {
    name: Located<String>,
    ty: Rc<Located<RuntimeType>>,
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
            name: "Bob" || "Jim",
            age: int,
        }
        "#.into());

        let mut builder = Builder::new();

        let result = builder.build(src.iter());
        let runtime = builder.link().unwrap();

        let good_bob = json!(
            {
                "name": "Bob",
            }
        );

        println!("{:?}", good_bob);

        let mut good_bob = (&good_bob).into();

        let result = runtime.evaluate("foo::bar::bob".into(), &mut good_bob);
        println!("{:?}", result);

        println!("{:?}", good_bob);
    }
}
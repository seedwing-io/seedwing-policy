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
use crate::lang::ty::{FunctionName, Type, Value};
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

pub struct Runtime {
    types: RefCell<HashMap<String, Located<RuntimeType>>>,
}

impl Runtime {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new(Self {
            types: RefCell::new(Default::default())
        })
    }

    fn define(self: &mut Rc<Self>, path: TypePath, ty: &Located<Type>) {
        println!("define {:?}", path.as_path_str());
        let converted = self.convert(ty);

        self.types.borrow_mut().insert(
            path.as_path_str(),
            converted
            //self.convert(&Located::new(Type::Nothing, 0..0))
        );
    }

    fn convert(self: &Rc<Self>, ty: &Located<Type>) -> Located<RuntimeType> {
        match &**ty {
            Type::Anything => {
                Located::new(RuntimeType::Anything, ty.location())
            },
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
                                        ty: self.convert(f.ty()),
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
                        Box::new(self.convert(&**lhs)),
                        Box::new(self.convert(&**rhs)),
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
    Ref(Rc<Runtime>, Located<TypePath>),
    Const(Located<Value>),
    Object(RuntimeObjectType),
    Expr(Located<Expr>),
    Join(Box<Located<RuntimeType>>, Box<Located<RuntimeType>>),
    Meet(Box<Located<RuntimeType>>, Box<Located<RuntimeType>>),
    Functional(Located<FunctionName>, Box<Located<RuntimeType>>),
    List(Box<Located<RuntimeType>>),
    Nothing,
}

pub struct RuntimeObjectType {
    fields: Vec<Located<RuntimeField>>,
}

pub struct RuntimeField {
    name: Located<String>,
    ty: Located<RuntimeType>,
}

#[cfg(test)]
mod test {
    use std::env;
    use std::iter::once;
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
}
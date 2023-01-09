use crate::core::Function;
use crate::lang::hir::MemberQualifier;
use crate::lang::lir;
use crate::lang::lir::ValueType;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::PrimordialType;
use crate::lang::TypeName;
use crate::lang::{hir, mir};
use crate::runtime::{BuildError, RuntimeError};
use crate::value::RuntimeValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct TypeHandle {
    name: Option<TypeName>,
    documentation: Option<String>,
    ty: RefCell<Option<Arc<Located<Type>>>>,
    parameters: Vec<Located<String>>,
}

impl TypeHandle {
    pub fn new(name: Option<TypeName>) -> Self {
        Self {
            name,
            documentation: None,
            ty: RefCell::new(None),
            parameters: vec![],
        }
    }

    pub fn documentation(&self) -> Option<String> {
        self.documentation.clone()
    }

    pub fn new_with(name: Option<TypeName>, ty: Located<mir::Type>) -> Self {
        Self {
            name,
            documentation: None,
            ty: RefCell::new(Some(Arc::new(ty))),
            parameters: vec![],
        }
    }

    pub fn with_documentation(mut self, documentation: Option<String>) -> Self {
        self.documentation = documentation;
        self
    }

    pub async fn with(mut self, ty: Located<mir::Type>) -> Self {
        self.define(Arc::new(ty)).await;
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<Located<String>>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }

    pub async fn define(&self, ty: Arc<Located<mir::Type>>) {
        self.ty.borrow_mut().replace(ty);
    }

    pub async fn define_from(&self, ty: Arc<TypeHandle>) {
        let inbound = ty.ty.borrow_mut().as_ref().cloned().unwrap();
        self.ty.borrow_mut().replace(inbound);
    }

    pub async fn ty(&self) -> Arc<Located<mir::Type>> {
        self.ty.borrow_mut().as_ref().unwrap().clone()
    }

    pub fn name(&self) -> Option<TypeName> {
        self.name.clone()
    }

    /*
    pub async fn evaluate(
        &self,
        value: Arc<Mutex<Value>>,
        bindings: &Bindings,
    ) -> Result<EvaluationResult, RuntimeError> {
        if let Some(ty) = &*self.ty.lock().await {
            ty.evaluate(value, bindings).await
        } else {
            Err(RuntimeError::InvalidState)
        }
    }
     */
}

pub enum Type {
    Anything,
    Primordial(PrimordialType),
    Bound(Arc<TypeHandle>, Bindings),
    Argument(Located<String>),
    Const(Located<ValueType>),
    Object(ObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Vec<Arc<TypeHandle>>),
    Meet(Vec<Arc<TypeHandle>>),
    Refinement(Arc<TypeHandle>, Arc<TypeHandle>),
    List(Arc<TypeHandle>),
    Nothing,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "anything"),
            Type::Primordial(inner) => write!(f, "{:?}", inner),
            Type::Const(inner) => write!(f, "{:?}", inner),
            Type::Object(inner) => write!(f, "{:?}", inner),
            Type::Expr(inner) => write!(f, "$({:?})", inner),
            Type::Join(terms) => write!(f, "||({:?})", terms),
            Type::Meet(terms) => write!(f, "&&({:?})", terms),
            Type::Refinement(primary, refinement) => {
                write!(f, "{:?}({:?})", primary, refinement)
            }
            Type::List(inner) => write!(f, "[{:?}]", inner),
            Type::Argument(name) => write!(f, "{:?}", name),
            Type::Bound(primary, bindings) => write!(f, "{:?}<{:?}>", primary, bindings),
            Type::Nothing => write!(f, "nothing"),
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

    pub fn get<S: Into<String>>(&self, name: S) -> Option<Arc<TypeHandle>> {
        self.bindings.get(&name.into()).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<TypeHandle>)> {
        self.bindings.iter()
    }
}

#[derive(Debug)]
pub struct Field {
    name: Located<String>,
    ty: Arc<TypeHandle>,
}

impl Field {
    pub fn new(name: Located<String>, ty: Arc<TypeHandle>) -> Self {
        Self { name, ty }
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<TypeHandle> {
        self.ty.clone()
    }
}

#[derive(Debug)]
pub struct ObjectType {
    fields: Vec<Arc<Located<Field>>>,
}

impl ObjectType {
    pub fn new(fields: Vec<Arc<Located<Field>>>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Arc<Located<Field>>> {
        &self.fields
    }
}

macro_rules! primordial_type {
    ($obj: expr, $name: literal, $primordial: expr) => {
        let name = TypeName::new(None, $name.into());
        $obj.insert(
            name.clone(),
            Arc::new(TypeHandle::new_with(
                Some(name),
                Located::new(mir::Type::Primordial($primordial), 0..0),
            )),
        );
    };
}

#[derive(Debug)]
pub struct World {
    types: HashMap<TypeName, Arc<TypeHandle>>,
}

impl World {
    pub(crate) fn new() -> Self {
        let mut initial_types = HashMap::new();

        primordial_type!(initial_types, "integer", PrimordialType::Integer);
        primordial_type!(initial_types, "string", PrimordialType::String);
        primordial_type!(initial_types, "boolean", PrimordialType::Boolean);
        primordial_type!(initial_types, "decimal", PrimordialType::Decimal);

        Self {
            types: initial_types,
        }
    }

    pub(crate) fn known_world(&self) -> Vec<TypeName> {
        self.types.keys().cloned().collect()
    }

    pub(crate) async fn declare(
        &mut self,
        path: TypeName,
        documentation: Option<String>,
        parameters: Vec<Located<String>>,
    ) {
        self.types.insert(
            path.clone(),
            Arc::new(
                TypeHandle::new(Some(path))
                    .with_parameters(parameters)
                    .with_documentation(documentation),
            ),
        );
    }

    pub(crate) async fn define(&mut self, path: TypeName, ty: &Located<hir::Type>) {
        log::info!("define type {}", path);
        let converted = self.convert(ty).await;
        if let Some(handle) = self.types.get_mut(&path) {
            handle.define_from(converted).await;
        }
    }

    pub(crate) async fn define_function(&mut self, path: TypeName, func: Arc<dyn Function>) {
        log::info!("define function {}", path);
        let runtime_type = Located::new(
            mir::Type::Primordial(PrimordialType::Function(path.clone(), func.clone())),
            0..0,
        );

        if let Some(handle) = self.types.get_mut(&path) {
            handle.define(Arc::new(runtime_type)).await;
        }
    }

    fn convert<'c>(
        &'c self,
        ty: &'c Located<hir::Type>,
    ) -> Pin<Box<dyn Future<Output = Arc<TypeHandle>> + 'c>> {
        match &**ty {
            hir::Type::Anything => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(mir::Type::Anything, ty.location()))
                        .await,
                )
            }),
            hir::Type::Ref(inner, arguments) => Box::pin(async move {
                let primary_type = self.types[&(inner.inner())].clone();

                if arguments.is_empty() {
                    primary_type
                } else {
                    let parameter_names = primary_type.parameters();

                    if parameter_names.len() != arguments.len() {
                        todo!("argument mismatch")
                    }

                    let mut bindings = Bindings::new();

                    for (name, arg) in parameter_names.iter().zip(arguments.iter()) {
                        bindings.bind(name.inner(), self.convert(arg).await)
                    }

                    Arc::new(
                        TypeHandle::new(None)
                            .with(Located::new(
                                mir::Type::Bound(primary_type, bindings),
                                ty.location(),
                            ))
                            .await,
                    )
                }
            }),
            hir::Type::Parameter(name) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(
                            mir::Type::Argument(name.clone()),
                            name.location(),
                        ))
                        .await,
                )
            }),
            hir::Type::Const(inner) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(mir::Type::Const(inner.clone()), ty.location()))
                        .await,
                )
            }),
            hir::Type::Object(inner) => Box::pin(async move {
                let mut fields = Vec::new();

                for f in inner.fields().iter() {
                    fields.push(Arc::new(Located::new(
                        Field::new(f.name().clone(), self.convert(f.ty()).await),
                        ty.location(),
                    )));
                }

                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(
                            mir::Type::Object(ObjectType::new(fields)),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            hir::Type::Expr(inner) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(
                            mir::Type::Expr(Arc::new(inner.clone())),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            hir::Type::Join(terms) => Box::pin(async move {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e).await)
                }
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(mir::Type::Join(inner), ty.location()))
                        .await,
                )
            }),
            hir::Type::Meet(terms) => Box::pin(async move {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e).await)
                }
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(mir::Type::Meet(inner), ty.location()))
                        .await,
                )
            }),
            hir::Type::Refinement(primary, refinement) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(
                            mir::Type::Refinement(
                                self.convert(&**primary).await,
                                self.convert(&**refinement).await,
                            ),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            hir::Type::List(inner) => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(
                            mir::Type::List(self.convert(inner).await),
                            ty.location(),
                        ))
                        .await,
                )
            }),
            hir::Type::Nothing => Box::pin(async move {
                Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(mir::Type::Nothing, ty.location()))
                        .await,
                )
            }),
        }
    }

    pub async fn lower(mut self) -> Result<lir::World, Vec<BuildError>> {
        let mut world = lir::World::new();

        for (path, handle) in self.types {
            world.add(path, handle).await;
        }

        Ok(world)
    }
}

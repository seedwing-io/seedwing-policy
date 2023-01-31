use crate::core::Function;
use crate::lang::hir::{Expr, MemberQualifier};
use crate::lang::lir::ValueType;
use crate::lang::parser::{Located, SourceLocation};
use crate::lang::PrimordialType;
use crate::lang::{hir, mir};
use crate::lang::{lir, SyntacticSugar};
use crate::runtime;
use crate::runtime::TypeName;
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

    pub fn with(mut self, ty: Located<mir::Type>) -> Self {
        self.define(Arc::new(ty));
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<Located<String>>) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }

    pub fn define(&self, ty: Arc<Located<mir::Type>>) {
        self.ty.borrow_mut().replace(ty);
    }

    pub fn define_from(&self, ty: Arc<TypeHandle>) {
        let inbound = ty.ty.borrow_mut().as_ref().cloned().unwrap();
        self.ty.borrow_mut().replace(inbound);
    }

    pub fn ty(&self) -> Arc<Located<mir::Type>> {
        self.ty.borrow_mut().as_ref().unwrap().clone()
    }

    pub fn name(&self) -> Option<TypeName> {
        self.name.clone()
    }
}

pub enum Type {
    Anything,
    Primordial(PrimordialType),
    Ref(SyntacticSugar, usize, Vec<Arc<TypeHandle>>),
    Deref(Arc<TypeHandle>),
    Argument(String),
    Const(ValueType),
    Object(ObjectType),
    Expr(Arc<Located<Expr>>),
    List(Vec<Arc<TypeHandle>>),
    Nothing,
}

impl Debug for Type {
    #[allow(clippy::uninlined_format_args)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "anything"),
            Type::Primordial(inner) => write!(f, "{:?}", inner),
            Type::Const(inner) => write!(f, "{:?}", inner),
            Type::Object(inner) => write!(f, "{:?}", inner),
            Type::Expr(inner) => write!(f, "$({:?})", inner),
            Type::List(inner) => write!(f, "[{:?}]", inner),
            Type::Argument(name) => write!(f, "{:?}", name),
            Type::Ref(sugar, slot, bindings) => write!(f, "{:?}<{:?}>", slot, bindings),
            Type::Deref(inner) => write!(f, "*{:?}", inner),
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

    pub fn iter(&self) -> impl Iterator<Item=(&String, &Arc<TypeHandle>)> {
        self.bindings.iter()
    }
}

#[derive(Debug)]
pub struct Field {
    name: Located<String>,
    ty: Arc<TypeHandle>,
    optional: bool,
}

impl Field {
    pub fn new(name: Located<String>, ty: Arc<TypeHandle>, optional: bool) -> Self {
        Self { name, ty, optional }
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<TypeHandle> {
        self.ty.clone()
    }

    pub fn optional(&self) -> bool {
        self.optional
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
    type_slots: Vec<Arc<TypeHandle>>,
    types: HashMap<TypeName, usize>,
}

impl World {
    pub(crate) fn new() -> Self {
        //let mut initial_types = HashMap::new();

        //primordial_type!(initial_types, "integer", PrimordialType::Integer);
        //primordial_type!(initial_types, "string", PrimordialType::String);
        //primordial_type!(initial_types, "boolean", PrimordialType::Boolean);
        //primordial_type!(initial_types, "decimal", PrimordialType::Decimal);

        let mut this = Self {
            type_slots: vec![],
            types: Default::default(),
        };

        this.define_primordial("integer", PrimordialType::Integer);
        this.define_primordial("string", PrimordialType::String);
        this.define_primordial("boolean", PrimordialType::Boolean);
        this.define_primordial("decimal", PrimordialType::Decimal);

        this
    }

    fn define_primordial(&mut self, name: &str, ty: PrimordialType) {
        let name = TypeName::new(None, name.into());

        let ty = Arc::new(TypeHandle::new_with(
            Some(name.clone()),
            Located::new(mir::Type::Primordial(ty), 0..0),
        ));

        self.type_slots.push(ty);
        self.types.insert(name, self.type_slots.len() - 1);
    }

    pub(crate) fn known_world(&self) -> Vec<TypeName> {
        self.types.keys().cloned().collect()
    }

    pub(crate) fn declare(
        &mut self,
        path: TypeName,
        documentation: Option<String>,
        parameters: Vec<Located<String>>,
    ) {
        log::info!("declare {}", path);
        if documentation.is_none() {
            log::warn!("{} is not documented", path.as_type_str());
        }

        self.type_slots.push(Arc::new(
            TypeHandle::new(Some(path.clone()))
                .with_parameters(parameters)
                .with_documentation(documentation),
        ));
        self.types.insert(path, self.type_slots.len() - 1);
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn define(
        &mut self,
        path: TypeName,
        ty: &Located<hir::Type>,
    ) -> Result<(), BuildError> {
        log::info!("define type {}", path);
        let converted = self.convert(ty)?;
        if let Some(handle) = self.types.get_mut(&path) {
            //handle.define_from(converted);
            self.type_slots[*handle].define_from(converted);
        }
        Ok(())
    }

    pub(crate) fn define_function(&mut self, path: TypeName, func: Arc<dyn Function>) {
        log::info!("define function {}", path);
        let runtime_type = Located::new(
            mir::Type::Primordial(PrimordialType::Function(
                SyntacticSugar::from(path.clone()),
                path.clone(),
                func,
            )),
            0..0,
        );

        if let Some(handle) = self.types.get_mut(&path) {
            self.type_slots[*handle].define(Arc::new(runtime_type));
        }
    }

    #[allow(clippy::result_large_err)]
    fn convert<'c>(&'c self, ty: &'c Located<hir::Type>) -> Result<Arc<TypeHandle>, BuildError> {
        match &**ty {
            hir::Type::Anything => Ok(Arc::new(
                TypeHandle::new(None).with(Located::new(mir::Type::Anything, ty.location())),
            )),
            hir::Type::Ref(sugar, inner, arguments) => {
                let primary_type_handle = self.types[&(inner.inner())];
                if arguments.is_empty() {
                    Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                        mir::Type::Ref(sugar.clone(), primary_type_handle, Vec::default()),
                        inner.location(),
                    ))))
                } else {
                    let primary_type = &self.type_slots[primary_type_handle];
                    let parameter_names = primary_type.parameters();

                    if parameter_names.len() != arguments.len() {
                        return Err(BuildError::ArgumentMismatch(
                            String::new().into(),
                            arguments[0].location().span(),
                        ));
                    }

                    let mut bindings = Vec::new();

                    for (_name, arg) in parameter_names.iter().zip(arguments.iter()) {
                        bindings.push(self.convert(arg)?)
                    }
                    Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                        mir::Type::Ref(sugar.clone(), primary_type_handle, bindings),
                        inner.location(),
                    ))))
                }
            }
            hir::Type::Parameter(name) => Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                mir::Type::Argument(name.inner()),
                name.location(),
            )))),
            hir::Type::Const(inner) => Ok(Arc::new(
                TypeHandle::new(None)
                    .with(Located::new(mir::Type::Const(inner.inner()), ty.location())),
            )),
            hir::Type::Deref(inner) => {
                Ok(Arc::new(
                    TypeHandle::new(None)
                        .with(Located::new(
                            mir::Type::Deref(self.convert(&*inner)?),
                            ty.location(),
                        ))
                ))
            }
            hir::Type::Object(inner) => {
                let mut fields = Vec::new();

                for f in inner.fields().iter() {
                    fields.push(Arc::new(Located::new(
                        Field::new(f.name().clone(), self.convert(f.ty())?, f.optional()),
                        ty.location(),
                    )));
                }

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Object(ObjectType::new(fields)),
                    ty.location(),
                ))))
            }
            hir::Type::Expr(inner) => Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                mir::Type::Expr(Arc::new(inner.clone())),
                ty.location(),
            )))),
            hir::Type::Not(inner) => {
                let primary_type_handle = self.types[&(String::from("lang::Not").into())];

                let bindings = vec![self.convert(inner)?];

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Ref(SyntacticSugar::Not, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Type::Join(terms) => {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }

                let primary_type_handle = self.types[&(String::from("lang::Or").into())];

                let bindings = vec![Arc::new(
                    TypeHandle::new(None).with(Located::new(Type::List(inner), ty.location())),
                )];

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Ref(SyntacticSugar::Or, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Type::Meet(terms) => {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }

                let primary_type_handle = self.types[&(String::from("lang::And").into())];

                let bindings = vec![Arc::new(
                    TypeHandle::new(None).with(Located::new(Type::List(inner), ty.location())),
                )];

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Ref(SyntacticSugar::And, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Type::Refinement(refinement) => {
                let primary_type_handle = self.types[&(String::from("lang::Refine").into())];

                let bindings = vec![self.convert(refinement)?];

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Ref(SyntacticSugar::Refine, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Type::Traverse(step) => {
                let primary_type_handle = self.types[&(String::from("lang::Traverse").into())];

                let bindings = vec![Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Const(ValueType::String(step.inner())),
                    step.location(),
                )))];

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Ref(SyntacticSugar::Traverse, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Type::Chain(terms) => {
                let primary_type_handle = self.types[&(String::from("lang::Chain").into())];

                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }

                let bindings = vec![Arc::new(
                    TypeHandle::new(None).with(Located::new(Type::List(inner), ty.location())),
                )];

                Ok(Arc::new(TypeHandle::new(None).with(Located::new(
                    mir::Type::Ref(SyntacticSugar::Chain, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Type::List(terms) => {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }
                Ok(Arc::new(
                    TypeHandle::new(None).with(Located::new(mir::Type::List(inner), ty.location())),
                ))
            }
            hir::Type::Nothing => Ok(Arc::new(
                TypeHandle::new(None).with(Located::new(mir::Type::Nothing, ty.location())),
            )),
        }
    }

    pub fn lower(mut self) -> Result<runtime::World, Vec<BuildError>> {
        let mut world = runtime::World::new();

        log::info!("Compiling {} patterns", self.types.len());

        for (slot, ty) in self.type_slots.iter().enumerate() {
            world.add(ty.name.as_ref().unwrap().clone(), ty.clone());
        }

        Ok(world)
    }
}

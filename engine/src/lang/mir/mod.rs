use crate::core::{Example, Function};
use crate::lang::hir::Expr;
use crate::lang::parser::Located;
use crate::lang::SyntacticSugar;
use crate::lang::{hir, mir};
use crate::lang::{PackageMeta, PrimordialPattern};
use crate::lang::{PatternMeta, ValuePattern};
use crate::runtime;
use crate::runtime::config::EvalConfig;
use crate::runtime::PatternName;
use crate::runtime::{BuildError, PackagePath};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct PatternHandle {
    name: Option<PatternName>,
    metadata: PatternMeta,
    examples: Vec<Example>,
    ty: RefCell<Option<Arc<Located<Pattern>>>>,
    parameters: Vec<Located<String>>,
}

impl PatternHandle {
    pub fn new(name: Option<PatternName>) -> Self {
        Self {
            name,
            metadata: Default::default(),
            examples: vec![],
            ty: RefCell::new(None),
            parameters: vec![],
        }
    }

    pub fn metadata(&self) -> &PatternMeta {
        &self.metadata
    }

    pub fn new_with(name: Option<PatternName>, ty: Located<mir::Pattern>) -> Self {
        Self {
            name,
            metadata: Default::default(),
            examples: vec![],
            ty: RefCell::new(Some(Arc::new(ty))),
            parameters: vec![],
        }
    }

    pub fn with_metadata(mut self, metadata: PatternMeta) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_examples(mut self, examples: Vec<Example>) -> Self {
        self.examples = examples;
        self
    }

    pub fn examples(&self) -> Vec<Example> {
        self.examples.clone()
    }

    pub fn with(self, ty: Located<mir::Pattern>) -> Self {
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

    pub fn define(&self, ty: Arc<Located<mir::Pattern>>) {
        self.ty.borrow_mut().replace(ty);
    }

    pub fn define_from(&self, ty: Arc<PatternHandle>) {
        let inbound = ty.ty.borrow_mut().as_ref().cloned().unwrap();
        self.ty.borrow_mut().replace(inbound);
    }

    pub fn ty(&self) -> Arc<Located<mir::Pattern>> {
        self.ty.borrow().as_ref().unwrap().clone()
    }

    pub fn name(&self) -> Option<PatternName> {
        self.name.clone()
    }
}

pub enum Pattern {
    Anything,
    Primordial(PrimordialPattern),
    Ref(SyntacticSugar, usize, Vec<Arc<PatternHandle>>),
    Deref(Arc<PatternHandle>),
    Argument(String),
    Const(ValuePattern),
    Object(ObjectPattern),
    Expr(Arc<Located<Expr>>),
    List(Vec<Arc<PatternHandle>>),
    Nothing,
}

impl Debug for Pattern {
    #[allow(clippy::uninlined_format_args)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::Anything => write!(f, "anything"),
            Pattern::Primordial(inner) => write!(f, "{:?}", inner),
            Pattern::Const(inner) => write!(f, "{:?}", inner),
            Pattern::Object(inner) => write!(f, "{:?}", inner),
            Pattern::Expr(inner) => write!(f, "$({:?})", inner),
            Pattern::List(inner) => write!(f, "[{:?}]", inner),
            Pattern::Argument(name) => write!(f, "{:?}", name),
            Pattern::Ref(_sugar, slot, bindings) => write!(f, "{:?}<{:?}>", slot, bindings),
            Pattern::Deref(inner) => write!(f, "*{:?}", inner),
            Pattern::Nothing => write!(f, "nothing"),
        }
    }
}

#[derive(Debug)]
pub struct Field {
    name: Located<String>,
    ty: Arc<PatternHandle>,
    optional: bool,
}

impl Field {
    pub fn new(name: Located<String>, ty: Arc<PatternHandle>, optional: bool) -> Self {
        Self { name, ty, optional }
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<PatternHandle> {
        self.ty.clone()
    }

    pub fn optional(&self) -> bool {
        self.optional
    }
}

#[derive(Debug)]
pub struct ObjectPattern {
    fields: Vec<Arc<Located<Field>>>,
}

impl ObjectPattern {
    pub fn new(fields: Vec<Arc<Located<Field>>>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Arc<Located<Field>>> {
        &self.fields
    }
}

#[derive(Debug)]
pub struct World {
    config: EvalConfig,
    type_slots: Vec<Arc<PatternHandle>>,
    types: HashMap<PatternName, usize>,
    packages: HashMap<PackagePath, PackageMeta>,
}

impl World {
    pub(crate) fn new(config: EvalConfig) -> Self {
        let mut this = Self {
            config,
            type_slots: vec![],
            types: Default::default(),
            packages: Default::default(),
        };

        this.define_primordial("integer", PrimordialPattern::Integer);
        this.define_primordial("string", PrimordialPattern::String);
        this.define_primordial("boolean", PrimordialPattern::Boolean);
        this.define_primordial("decimal", PrimordialPattern::Decimal);

        this
    }

    fn define_primordial(&mut self, name: &str, ty: PrimordialPattern) {
        let name = PatternName::new(None, name.into());

        let ty = Arc::new(PatternHandle::new_with(
            Some(name.clone()),
            Located::new(mir::Pattern::Primordial(ty), 0..0),
        ));

        self.type_slots.push(ty);
        self.types.insert(name, self.type_slots.len() - 1);
    }

    pub(crate) fn known_world(&self) -> Vec<PatternName> {
        self.types.keys().cloned().collect()
    }

    pub(crate) fn declare(
        &mut self,
        path: PatternName,
        metadata: PatternMeta,
        examples: Vec<Example>,
        parameters: Vec<Located<String>>,
    ) {
        log::debug!("declare {}", path);
        if metadata.documentation.is_none() {
            log::warn!("{} is not documented", path.as_type_str());
        }

        let runtime_type = Arc::new(
            PatternHandle::new(Some(path.clone()))
                .with_parameters(parameters)
                .with_metadata(metadata)
                .with_examples(examples),
        );

        if let Some(handle) = self.types.get_mut(&path) {
            // self.types already contains an entry for this path so update it.
            self.type_slots[*handle] = runtime_type;
        } else {
            self.type_slots.push(runtime_type);
            self.types.insert(path, self.type_slots.len() - 1);
        }
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn define(
        &mut self,
        path: PatternName,
        ty: &Located<hir::Pattern>,
    ) -> Result<(), BuildError> {
        log::debug!("define type {}", path);
        let converted = self.convert(ty)?;
        if let Some(handle) = self.types.get_mut(&path) {
            self.type_slots[*handle].define_from(converted);
        } else {
            log::error!("Attempting to define an undeclared type");
        }
        Ok(())
    }

    pub(crate) fn define_function(&mut self, path: PatternName, func: Arc<dyn Function>) {
        log::debug!("define function {}", path);
        let runtime_type = Located::new(
            mir::Pattern::Primordial(PrimordialPattern::Function(
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

    pub(crate) fn define_package(&mut self, path: PackagePath, meta: PackageMeta) {
        log::debug!("define package: {}", path);
        self.packages.insert(path, meta);
    }

    #[allow(clippy::result_large_err)]
    fn convert<'c>(
        &'c self,
        ty: &'c Located<hir::Pattern>,
    ) -> Result<Arc<PatternHandle>, BuildError> {
        match &**ty {
            hir::Pattern::Anything => Ok(Arc::new(
                PatternHandle::new(None).with(Located::new(mir::Pattern::Anything, ty.location())),
            )),
            hir::Pattern::Ref(sugar, inner, arguments) => {
                let primary_type_handle = self.types[&(inner.inner())];
                if arguments.is_empty() {
                    Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(sugar.clone(), primary_type_handle, Vec::default()),
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
                    Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(sugar.clone(), primary_type_handle, bindings),
                        inner.location(),
                    ))))
                }
            }
            hir::Pattern::Parameter(name) => Ok(Arc::new(PatternHandle::new(None).with(
                Located::new(mir::Pattern::Argument(name.inner()), name.location()),
            ))),
            hir::Pattern::Const(inner) => Ok(Arc::new(PatternHandle::new(None).with(
                Located::new(mir::Pattern::Const(inner.inner()), ty.location()),
            ))),
            hir::Pattern::Deref(inner) => Ok(Arc::new(PatternHandle::new(None).with(
                Located::new(mir::Pattern::Deref(self.convert(inner)?), ty.location()),
            ))),
            hir::Pattern::Object(inner) => {
                let mut fields = Vec::new();

                for f in inner.fields().iter() {
                    fields.push(Arc::new(Located::new(
                        Field::new(f.name().clone(), self.convert(f.ty())?, f.optional()),
                        ty.location(),
                    )));
                }

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Object(ObjectPattern::new(fields)),
                    ty.location(),
                ))))
            }
            hir::Pattern::Expr(inner) => Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                mir::Pattern::Expr(Arc::new(inner.clone())),
                ty.location(),
            )))),
            hir::Pattern::Not(inner) => {
                let primary_type_handle = self.types[&(String::from("lang::not").into())];

                let bindings = vec![self.convert(inner)?];

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Ref(SyntacticSugar::Not, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Pattern::Join(terms) => {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }

                let primary_type_handle = self.types[&(String::from("lang::or").into())];

                let bindings = vec![Arc::new(
                    PatternHandle::new(None)
                        .with(Located::new(Pattern::List(inner), ty.location())),
                )];

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Ref(SyntacticSugar::Or, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Pattern::Meet(terms) => {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }

                let primary_type_handle = self.types[&(String::from("lang::and").into())];

                let bindings = vec![Arc::new(
                    PatternHandle::new(None)
                        .with(Located::new(Pattern::List(inner), ty.location())),
                )];

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Ref(SyntacticSugar::And, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Pattern::Refinement(refinement) => {
                let primary_type_handle = self.types[&(String::from("lang::refine").into())];

                let bindings = vec![self.convert(refinement)?];

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Ref(SyntacticSugar::Refine, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Pattern::Traverse(step) => {
                let primary_type_handle = self.types[&(String::from("lang::traverse").into())];

                let bindings = vec![Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Const(ValuePattern::String(step.inner())),
                    step.location(),
                )))];

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Ref(SyntacticSugar::Traverse, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Pattern::Chain(terms) => {
                let primary_type_handle = self.types[&(String::from("lang::chain").into())];

                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }

                let bindings = vec![Arc::new(
                    PatternHandle::new(None)
                        .with(Located::new(Pattern::List(inner), ty.location())),
                )];

                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Ref(SyntacticSugar::Chain, primary_type_handle, bindings),
                    ty.location(),
                ))))
            }
            hir::Pattern::List(terms) => {
                let mut inner = Vec::new();
                for e in terms {
                    inner.push(self.convert(e)?)
                }
                Ok(Arc::new(PatternHandle::new(None).with(Located::new(
                    mir::Pattern::List(inner),
                    ty.location(),
                ))))
            }
            hir::Pattern::Nothing => Ok(Arc::new(
                PatternHandle::new(None).with(Located::new(mir::Pattern::Nothing, ty.location())),
            )),
        }
    }

    pub fn lower(self) -> Result<runtime::World, Vec<BuildError>> {
        let mut world = runtime::World::new(self.config.clone());

        log::info!("Compiling {} patterns", self.types.len());

        for (_slot, ty) in self.type_slots.iter().enumerate() {
            world.add(ty.name.as_ref().unwrap().clone(), ty.clone());
        }

        world.add_packages(self.packages);

        Ok(world)
    }
}

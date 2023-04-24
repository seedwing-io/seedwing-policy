use crate::core::{Example, Function};
use crate::lang::hir::Expr;
use crate::lang::parser::Located;
use crate::lang::{hir, mir};
use crate::lang::{lir, SyntacticSugar};
use crate::lang::{PackageMeta, PrimordialPattern};
use crate::lang::{PatternMeta, ValuePattern};
use crate::runtime;
use crate::runtime::config::ConfigContext;
use crate::runtime::metadata::{PackageMetadata, SubpackageMetadata, ToMetadata, WorldLike};
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

    pub fn set_metadata(&mut self, metadata: PatternMeta) {
        self.metadata = metadata;
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
    config: ConfigContext,
    type_slots: Vec<Arc<PatternHandle>>,
    types: HashMap<PatternName, usize>,
    packages: HashMap<PackagePath, PackageMeta>,
}

impl World {
    pub(crate) fn new(config: ConfigContext) -> Self {
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
            log::info!("{} is not documented", path.as_type_str());
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
        let converted = Arc::new(self.convert(ty)?);
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
    fn convert<'c>(&'c self, ty: &'c Located<hir::Pattern>) -> Result<PatternHandle, BuildError> {
        let handle =
            match &**ty {
                hir::Pattern::Anything => PatternHandle::new(None)
                    .with(Located::new(mir::Pattern::Anything, ty.location())),
                hir::Pattern::Ref(sugar, inner, arguments) => {
                    let primary_type_handle = self.types[&(inner.inner())];
                    if arguments.is_empty() {
                        PatternHandle::new(None).with(Located::new(
                            mir::Pattern::Ref(sugar.clone(), primary_type_handle, Vec::default()),
                            inner.location(),
                        ))
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
                            bindings.push(Arc::new(self.convert(arg)?))
                        }
                        PatternHandle::new(None).with(Located::new(
                            mir::Pattern::Ref(sugar.clone(), primary_type_handle, bindings),
                            inner.location(),
                        ))
                    }
                }
                hir::Pattern::Parameter(name) => PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Argument(name.inner()),
                    name.location(),
                )),
                hir::Pattern::Const(inner) => PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Const(inner.inner()),
                    ty.location(),
                )),
                hir::Pattern::Deref(inner) => PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Deref(Arc::new(self.convert(inner)?)),
                    ty.location(),
                )),
                hir::Pattern::Object(inner) => {
                    let mut fields = Vec::new();

                    for f in inner.fields().iter() {
                        let mut field_ty = self.convert(f.ty())?;
                        field_ty.set_metadata(f.metadata().clone().try_into()?);

                        fields.push(Arc::new(Located::new(
                            Field::new(f.name().clone(), Arc::new(field_ty), f.optional()),
                            ty.location(),
                        )));
                    }

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Object(ObjectPattern::new(fields)),
                        ty.location(),
                    ))
                }
                hir::Pattern::Expr(inner) => PatternHandle::new(None).with(Located::new(
                    mir::Pattern::Expr(Arc::new(inner.clone())),
                    ty.location(),
                )),
                hir::Pattern::Not(inner) => {
                    let primary_type_handle = self.types[&(String::from("lang::not").into())];

                    let bindings = vec![Arc::new(self.convert(inner)?)];

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(SyntacticSugar::Not, primary_type_handle, bindings),
                        ty.location(),
                    ))
                }
                hir::Pattern::Join(terms) => {
                    let mut inner = Vec::new();
                    for e in terms {
                        inner.push(Arc::new(self.convert(e)?))
                    }

                    let primary_type_handle = self.types[&(String::from("lang::or").into())];

                    let bindings = vec![Arc::new(
                        PatternHandle::new(None)
                            .with(Located::new(Pattern::List(inner), ty.location())),
                    )];

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(SyntacticSugar::Or, primary_type_handle, bindings),
                        ty.location(),
                    ))
                }
                hir::Pattern::Meet(terms) => {
                    let mut inner = Vec::new();
                    for e in terms {
                        inner.push(Arc::new(self.convert(e)?));
                    }

                    let primary_type_handle = self.types[&(String::from("lang::and").into())];

                    let bindings = vec![Arc::new(
                        PatternHandle::new(None)
                            .with(Located::new(Pattern::List(inner), ty.location())),
                    )];

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(SyntacticSugar::And, primary_type_handle, bindings),
                        ty.location(),
                    ))
                }
                hir::Pattern::Refinement(refinement) => {
                    let primary_type_handle = self.types[&(String::from("lang::refine").into())];

                    let bindings = vec![Arc::new(self.convert(refinement)?)];

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(SyntacticSugar::Refine, primary_type_handle, bindings),
                        ty.location(),
                    ))
                }
                hir::Pattern::Traverse(step) => {
                    let primary_type_handle = self.types[&(String::from("lang::traverse").into())];

                    let bindings = vec![Arc::new(PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Const(ValuePattern::String(step.inner().into())),
                        step.location(),
                    )))];

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(SyntacticSugar::Traverse, primary_type_handle, bindings),
                        ty.location(),
                    ))
                }
                hir::Pattern::Chain(terms) => {
                    let primary_type_handle = self.types[&(String::from("lang::chain").into())];

                    let mut inner = Vec::new();
                    for e in terms {
                        inner.push(Arc::new(self.convert(e)?))
                    }

                    let bindings = vec![Arc::new(
                        PatternHandle::new(None)
                            .with(Located::new(Pattern::List(inner), ty.location())),
                    )];

                    PatternHandle::new(None).with(Located::new(
                        mir::Pattern::Ref(SyntacticSugar::Chain, primary_type_handle, bindings),
                        ty.location(),
                    ))
                }
                hir::Pattern::List(terms) => {
                    let mut inner = Vec::new();
                    for e in terms {
                        inner.push(Arc::new(self.convert(e)?));
                    }
                    PatternHandle::new(None)
                        .with(Located::new(mir::Pattern::List(inner), ty.location()))
                }
                hir::Pattern::Nothing => PatternHandle::new(None)
                    .with(Located::new(mir::Pattern::Nothing, ty.location())),
            };

        Ok(handle)
    }

    pub fn lower(self) -> Result<runtime::World, Vec<BuildError>> {
        Lowerer::new(self).lower()
    }
}

struct Lowerer {
    world: World,

    types: HashMap<PatternName, usize>,
    type_slots: Vec<Arc<lir::Pattern>>,
    packages: HashMap<PackagePath, PackageMetadata>,
}

struct SlotAccessor<'a>(&'a [Arc<lir::Pattern>]);

impl<'a> WorldLike for SlotAccessor<'a> {
    fn get_by_slot(&self, slot: usize) -> Option<Arc<runtime::Pattern>> {
        self.0.get(slot).cloned()
    }
}

impl Lowerer {
    fn new(world: World) -> Self {
        Self {
            world,
            types: Default::default(),
            type_slots: Default::default(),
            packages: Default::default(),
        }
    }

    fn lower(mut self) -> Result<runtime::World, Vec<BuildError>> {
        log::info!("Compiling {} patterns", self.world.types.len());

        self.add_types();
        self.build_packages();
        self.apply_packages();
        self.sort_world();

        // done

        Ok(runtime::World::new(
            self.world.config,
            self.types,
            self.type_slots,
            self.packages,
        ))
    }

    fn add_types(&mut self) {
        for (slot, handle) in self.world.type_slots.iter().enumerate() {
            let path = handle.name.as_ref().unwrap().clone();

            let name = handle.name();
            let parameters = handle.parameters().iter().map(|e| e.inner()).collect();
            let converted = lir::convert(
                name,
                handle.metadata().clone(),
                handle.examples(),
                parameters,
                &handle.ty(),
            );
            self.type_slots.push(converted.clone());
            self.types.insert(path, slot);
        }
    }

    /// Build the package hierarchy from the known types
    fn build_packages(&mut self) {
        // insert the root
        self.packages.insert(
            PackagePath::root(),
            PackageMetadata::new(PackagePath::root()),
        );

        for (slot, handle) in self.world.type_slots.iter().enumerate() {
            // get the name

            let path = match handle.name.as_ref().and_then(|n| n.package.as_ref()) {
                Some(path) => path.clone(),
                None => {
                    continue;
                }
            };

            // create from leaf to root

            {
                // current path
                let mut path = path.clone();
                // carry over child when creating the parent
                let mut child: Option<PackagePath> = None;

                // while we don't have the "current" path
                while !self.packages.contains_key(&path) {
                    // create a new instance
                    let mut meta = PackageMetadata::new(path.clone());
                    if let Some(child) = child {
                        // fill with the parent information
                        meta.packages.push(SubpackageMetadata {
                            // we can unwrap, as there always is a name, if we had a parent
                            name: child.name().unwrap(),
                            documentation: Default::default(),
                        });
                    }
                    // and insert it
                    self.packages.insert(path.clone(), meta);

                    // eval parent information
                    path = match path.parent() {
                        Some(parent) => {
                            child = Some(path);
                            parent
                        }
                        None => {
                            child = None;
                            // we reached to root, ensure that we registered there too
                            // we use the root is present, so we can use .unwrap()
                            let root = self.packages.get_mut(&PackagePath::root()).unwrap();
                            if let Some(name) = path.name() {
                                root.packages.push(SubpackageMetadata::new(name));
                            }

                            // then bail out
                            break;
                        }
                    }
                    // and continue with the parent level (if there is one)
                }

                if let Some(child) = child {
                    // we did insert something, and did have a parent, but the parent existed
                    // however, we didn't add ourselves to the (existing) parent, so do that now
                    let parent_pkg = self.packages.get_mut(&child.parent().unwrap()).unwrap();

                    // we can unwrap, as there always is a name, if we had a parent
                    parent_pkg
                        .packages
                        .push(SubpackageMetadata::new(child.name().unwrap()));
                }
            }

            // add the type

            if let Some(pkg) = self.packages.get_mut(&path) {
                // we can call to_meta here, as we have a full set of types
                if let Ok(meta) = self.type_slots[slot].to_meta(&SlotAccessor(&self.type_slots)) {
                    pkg.patterns.push(meta);
                }
            }
        }
    }

    fn apply_packages(&mut self) {
        for (path, meta) in &self.world.packages {
            log::debug!("Apply package metadata: {path}: {meta:?}");

            if let Some(pkg) = self.packages.get_mut(path) {
                pkg.apply_meta(meta);
            } else {
                log::warn!("Found package metadata but no package: {path}");
            }

            if let Some((parent_pkg, child_name)) = path
                .split_name()
                .and_then(|(parent, child)| self.packages.get_mut(&parent).map(|p| (p, child)))
            {
                // find the child in the sub-packages list (we could do better here)
                if let Some(child) = parent_pkg
                    .packages
                    .iter_mut()
                    .find(|s| s.name == child_name.0)
                {
                    child.apply_meta(meta);
                }
            }
        }
    }

    /// The world should be sorted, providing a stable order of entries, even over restarts.
    fn sort_world(&mut self) {
        for pkg in self.packages.values_mut() {
            pkg.sort();
        }
    }
}

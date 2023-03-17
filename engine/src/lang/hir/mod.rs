use crate::core::Example;
use crate::data::DataSource;
use crate::lang::lir::ValuePattern;
use crate::lang::parser::{CompilationUnit, Located, Location, PolicyParser, SourceLocation};
use crate::lang::{lir, mir, SyntacticSugar};
use crate::package::Package;
use crate::runtime::cache::SourceCache;
use crate::runtime::config::{ConfigValue, EvalConfig};
use crate::runtime::{AbsolutePackagePath, BuildError, PackagePath, PatternName};
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::iter::once;
use std::sync::Arc;

mod meta;

pub use meta::*;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum Expr {
    SelfLiteral(#[serde(skip)] Location),
    /* self */
    Value(Located<ValuePattern>),
    Add(Box<Located<Expr>>, Box<Located<Expr>>),
    Subtract(Box<Located<Expr>>, Box<Located<Expr>>),
    Multiply(Box<Located<Expr>>, Box<Located<Expr>>),
    Divide(Box<Located<Expr>>, Box<Located<Expr>>),
    LessThan(Box<Located<Expr>>, Box<Located<Expr>>),
    LessThanEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    GreaterThan(Box<Located<Expr>>, Box<Located<Expr>>),
    GreaterThanEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    Equal(Box<Located<Expr>>, Box<Located<Expr>>),
    NotEqual(Box<Located<Expr>>, Box<Located<Expr>>),
    LogicalAnd(Box<Located<Expr>>, Box<Located<Expr>>),
    LogicalOr(Box<Located<Expr>>, Box<Located<Expr>>),
}

impl Expr {
    pub fn lower(&self) -> lir::Expr {
        match self {
            Expr::SelfLiteral(_) => lir::Expr::SelfLiteral(),
            Expr::Value(val) => lir::Expr::Value(val.inner()),
            Expr::Add(lhs, rhs) => lir::Expr::Add(Arc::new(lhs.lower()), Arc::new(rhs.lower())),
            Expr::Subtract(lhs, rhs) => {
                lir::Expr::Subtract(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::Multiply(lhs, rhs) => {
                lir::Expr::Multiply(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::Divide(lhs, rhs) => {
                lir::Expr::Divide(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::LessThan(lhs, rhs) => {
                lir::Expr::LessThan(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::LessThanEqual(lhs, rhs) => {
                lir::Expr::LessThanEqual(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::GreaterThan(lhs, rhs) => {
                lir::Expr::GreaterThan(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::GreaterThanEqual(lhs, rhs) => {
                lir::Expr::GreaterThanEqual(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::Equal(lhs, rhs) => lir::Expr::Equal(Arc::new(lhs.lower()), Arc::new(rhs.lower())),
            Expr::NotEqual(lhs, rhs) => {
                lir::Expr::NotEqual(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::LogicalAnd(lhs, rhs) => {
                lir::Expr::LogicalAnd(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
            Expr::LogicalOr(lhs, rhs) => {
                lir::Expr::LogicalOr(Arc::new(lhs.lower()), Arc::new(rhs.lower()))
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatternDefn {
    name: Located<String>,
    ty: Located<Pattern>,
    parameters: Vec<Located<String>>,
    metadata: Metadata,
    examples: Vec<Example>,
}

impl PatternDefn {
    pub fn new(
        name: Located<String>,
        ty: Located<Pattern>,
        parameters: Vec<Located<String>>,
    ) -> Self {
        Self {
            name,
            ty,
            parameters,
            metadata: Default::default(),
            examples: vec![],
        }
    }

    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.metadata = metadata;
    }

    // triggers "unused" when running `cargo check` for the frontend
    #[allow(unused)]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    // currently this is unused, but we should find a way to provide examples
    #[allow(unused)]
    pub fn set_examples(&mut self, examples: Vec<Example>) {
        self.examples = examples;
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> &Located<Pattern> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<PatternName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<PatternName>>>) {
        self.ty.qualify_types(types);
    }

    pub(crate) fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }
}

#[derive(Clone, PartialEq)]
pub enum Pattern {
    Anything,
    Ref(SyntacticSugar, Located<PatternName>, Vec<Located<Pattern>>),
    Deref(Box<Located<Pattern>>),
    Parameter(Located<String>),
    Const(Located<ValuePattern>),
    Object(ObjectPattern),
    Expr(Located<Expr>),
    Join(Vec<Located<Pattern>>),
    Meet(Vec<Located<Pattern>>),
    List(Vec<Located<Pattern>>),
    // postfix
    Chain(Vec<Located<Pattern>>),
    Traverse(Located<String>),
    Refinement(Box<Located<Pattern>>),
    Not(Box<Located<Pattern>>),
    Nothing,
}

impl Pattern {
    pub(crate) fn referenced_types(&self) -> Vec<Located<PatternName>> {
        match self {
            Pattern::Anything => Vec::default(),
            Pattern::Ref(_, inner, arguuments) => once(inner.clone())
                .chain(arguuments.iter().flat_map(|e| e.referenced_types()))
                .collect(),
            Pattern::Const(_) => Vec::default(),
            Pattern::Object(inner) => inner.referenced_types(),
            Pattern::Expr(_) => Vec::default(),
            Pattern::Join(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Pattern::Meet(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Pattern::Refinement(refinement) => refinement.referenced_types(),
            Pattern::List(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Pattern::Chain(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Pattern::Not(inner) => inner.referenced_types(),
            Pattern::Deref(inner) => inner.referenced_types(),
            Pattern::Traverse(_) => Vec::default(),
            Pattern::Nothing => Vec::default(),
            Pattern::Parameter(_) => Vec::default(),
        }
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<PatternName>>>) {
        match self {
            Pattern::Anything => {}
            Pattern::Ref(_, ref mut name, arguments) => {
                if !name.is_qualified() {
                    // it's a simple single-word name, needs qualifying, perhaps.
                    if let Some(Some(qualified)) = types.get(name.name()) {
                        *name = qualified.clone();
                    }
                }
                for arg in arguments {
                    arg.qualify_types(types);
                }
            }
            Pattern::Const(_) => {}
            Pattern::Object(inner) => {
                inner.qualify_types(types);
            }
            Pattern::Expr(_) => {}
            Pattern::Join(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Pattern::Meet(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Pattern::Refinement(refinement) => {
                refinement.qualify_types(types);
            }
            Pattern::List(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Pattern::Chain(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Pattern::Not(inner) => inner.qualify_types(types),
            Pattern::Deref(inner) => inner.qualify_types(types),
            Pattern::Traverse(_) => {}
            Pattern::Nothing => {}
            Pattern::Parameter(_) => {}
        }
    }
}

impl Debug for Pattern {
    #[allow(clippy::uninlined_format_args)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::Anything => write!(f, "Anything"),
            Pattern::Ref(_, r, args) => write!(f, "{:?}<{:?}>", r, args),
            Pattern::Const(value) => write!(f, "{:?}", value),
            Pattern::Join(terms) => write!(f, "Join({:?})", terms),
            Pattern::Meet(terms) => write!(f, "Meet({:?})", terms),
            Pattern::Nothing => write!(f, "Nothing"),
            Pattern::Object(obj) => write!(f, "{:?}", obj),
            Pattern::Refinement(ty) => write!(f, "({:?})", ty),
            Pattern::List(ty) => write!(f, "[{:?}]", ty),
            Pattern::Expr(expr) => write!(f, "$({:?})", expr),
            Pattern::Chain(terms) => write!(f, "{:?}", terms),
            Pattern::Traverse(step) => write!(f, ".{}", step.inner()),
            Pattern::Not(inner) => write!(f, "!{:?}", inner),
            Pattern::Deref(inner) => write!(f, "*{:?}", inner),
            Pattern::Parameter(name) => write!(f, "{:?}", name),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectPattern {
    fields: Vec<Located<Field>>,
}

impl Default for ObjectPattern {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectPattern {
    pub fn new() -> Self {
        Self { fields: vec![] }
    }

    pub fn add_field(&mut self, field: Located<Field>) -> &Self {
        self.fields.push(field);
        self
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<PatternName>> {
        self.fields
            .iter()
            .flat_map(|e| e.referenced_types())
            .collect()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<PatternName>>>) {
        for field in &mut self.fields {
            field.qualify_types(types);
        }
    }

    pub fn fields(&self) -> &Vec<Located<Field>> {
        &self.fields
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    name: Located<String>,
    ty: Located<Pattern>,
    optional: bool,
    metadata: Metadata,
}

impl Field {
    pub fn new(name: Located<String>, ty: Located<Pattern>, optional: bool) -> Self {
        Self {
            name,
            ty,
            optional,
            metadata: Default::default(),
        }
    }

    pub fn name(&self) -> &Located<String> {
        &self.name
    }

    pub fn ty(&self) -> &Located<Pattern> {
        &self.ty
    }

    pub fn optional(&self) -> bool {
        self.optional
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<PatternName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<PatternName>>>) {
        self.ty.qualify_types(types)
    }

    pub fn set_metadata(&mut self, metadata: Metadata) {
        self.metadata = metadata;
    }
}

pub struct World {
    units: Vec<CompilationUnit>,
    packages: Vec<Package>,
    source_cache: SourceCache,
    data_sources: Vec<Arc<dyn DataSource>>,
    config: EvalConfig,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for World {
    fn clone(&self) -> Self {
        let mut h = World::new();
        h.packages = self.packages.clone();
        h.data_sources = self.data_sources.clone();
        h.config = self.config.clone();
        h
    }
}

impl World {
    pub fn new() -> Self {
        Self::new_with_config(EvalConfig::default())
    }

    pub fn new_with_config(config: EvalConfig) -> Self {
        let mut world = Self {
            units: Default::default(),
            packages: Default::default(),
            source_cache: Default::default(),
            data_sources: Vec::default(),
            config,
        };
        world.add_package(crate::core::lang::package());
        world.add_package(crate::core::config::package());
        world.add_package(crate::core::list::package());
        world.add_package(crate::core::string::package());
        world.add_package(crate::core::base64::package());
        world.add_package(crate::core::json::package());
        #[cfg(feature = "sigstore")]
        world.add_package(crate::core::sigstore::package());
        world.add_package(crate::core::x509::package());
        world.add_package(crate::core::cyclonedx::package());
        world.add_package(crate::core::jsf::package());
        world.add_package(crate::core::spdx::package());
        world.add_package(crate::core::iso::package());
        world.add_package(crate::core::kafka::package());
        world.add_package(crate::core::pem::package());
        world.add_package(crate::core::net::package());
        world.add_package(crate::core::openvex::package());
        world.add_package(crate::core::osv::package());
        world.add_package(crate::core::uri::package());
        world.add_package(crate::core::timestamp::package());
        world.add_package(crate::core::csaf::package());
        world.add_package(crate::core::rhsa::package());
        world.add_package(crate::core::slsa::package());
        world.add_package(crate::core::intoto::package());

        #[cfg(feature = "debug")]
        world.add_package(crate::core::debug::package());

        world.add_package(crate::core::maven::package());
        world.add_package(crate::core::external::package());
        world.add_package(crate::core::guac::package());

        #[cfg(feature = "showcase")]
        world.add_package(crate::core::showcase::package());

        world
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

    pub fn data<D: DataSource + 'static>(&mut self, src: D) {
        self.data_sources.push(Arc::new(src))
    }

    pub fn config(&mut self, key: String, val: ConfigValue) {
        self.config.insert(key, val);
    }

    fn add_compilation_unit(&mut self, unit: CompilationUnit) {
        self.units.push(unit)
    }

    pub fn add_package(&mut self, package: Package) {
        self.packages.push(package);
    }

    pub fn lower(&mut self) -> Result<mir::World, Vec<BuildError>> {
        self.add_package(crate::core::data::package(self.data_sources.clone()));

        let mut core_units = Vec::new();

        let mut errors = Vec::new();

        for pkg in &self.packages {
            for (source, stream) in pkg.source_iter() {
                log::info!("loading {}", source);
                self.source_cache.add(source.clone(), stream.clone().into());
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

        Lowerer::new(&mut self.units, &mut self.packages, self.config.clone()).lower()
    }
}

struct Lowerer<'b> {
    units: &'b mut Vec<CompilationUnit>,
    packages: &'b mut Vec<Package>,
    config: EvalConfig,
}

impl<'b> Lowerer<'b> {
    pub fn new(
        units: &'b mut Vec<CompilationUnit>,
        packages: &'b mut Vec<Package>,
        config: EvalConfig,
    ) -> Self {
        Self {
            units,
            packages,
            config,
        }
    }

    pub fn lower(self) -> Result<mir::World, Vec<BuildError>> {
        // First, perform internal per-unit linkage and type qualification
        let mut world = mir::World::new(self.config);
        let mut errors = Vec::new();

        for unit in self.units.iter_mut() {
            let unit_path = PackagePath::from(unit.source());

            let mut visible_types = unit
                .uses()
                .iter()
                .map(|e| (e.as_name().inner(), Some(e.type_name())))
                .chain(unit.types().iter().map(|e| {
                    (
                        e.name().inner(),
                        Some(Located::new(
                            unit_path.type_name(e.name().inner()),
                            e.location(),
                        )),
                    )
                }))
                .collect::<HashMap<String, Option<Located<PatternName>>>>();

            //visible_types.insert("int".into(), None);
            for primordial in world.known_world() {
                visible_types.insert(primordial.name().to_string(), None);
            }

            for defn in unit.types() {
                let referenced_types = defn.referenced_types();

                for ty in &referenced_types {
                    if !ty.is_qualified() && !visible_types.contains_key(ty.name()) {
                        errors.push(BuildError::PatternNotFound(
                            unit.source().clone(),
                            ty.location().span(),
                            ty.clone().as_type_str(),
                        ))
                    }
                }
            }

            for defn in unit.types_mut() {
                defn.qualify_types(&visible_types)
            }
        }

        // next, perform inter-unit linking.

        let mut known_world = world.known_world();

        for package in self.packages.iter() {
            let package_path = package.path();

            known_world.extend_from_slice(
                &package
                    .function_names()
                    .iter()
                    .map(|e| package_path.type_name(e.clone()))
                    .collect::<Vec<PatternName>>(),
            );
        }

        for unit in self.units.iter() {
            let unit_path = PackagePath::from(unit.source());

            let unit_types = unit
                .types()
                .iter()
                .map(|e| unit_path.type_name(e.name().inner()))
                .collect::<Vec<PatternName>>();

            known_world.extend_from_slice(&unit_types);
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for unit in self.units.iter() {
            for defn in unit.types() {
                // these should be fully-qualified now
                let referenced = defn.referenced_types();

                for each in referenced {
                    if !known_world.contains(&each.clone().inner()) {
                        errors.push(BuildError::PatternNotFound(
                            unit.source().clone(),
                            each.location().span(),
                            each.clone().as_type_str(),
                        ))
                    }
                }
            }
        }

        for unit in self.units.iter() {
            let unit_path = PackagePath::from(unit.source());

            for ty in unit.types() {
                let name = unit_path.type_name(ty.name().inner());
                let metadata = match ty.metadata.clone().try_into() {
                    Ok(metadata) => metadata,
                    Err(err) => {
                        errors.push(err);
                        continue;
                    }
                };
                world.declare(name, metadata, ty.examples.clone(), ty.parameters());
            }
        }

        for package in self.packages.iter() {
            let path = package.path();
            for (fn_name, func) in package.functions() {
                let path = path.type_name(fn_name);
                world.declare(
                    path,
                    func.metadata().clone(),
                    func.examples(),
                    func.parameters()
                        .iter()
                        .cloned()
                        .map(|p| Located::new(p, 0..0))
                        .collect(),
                );
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for package in self.packages.iter() {
            let path = package.path();
            for (fn_name, func) in package.functions() {
                let path = path.type_name(fn_name);
                world.define_function(path, func);
            }
        }

        for unit in self.units.iter() {
            let unit_path = PackagePath::from(unit.source());

            for (path, ty) in unit.types().iter().map(|e| {
                (
                    Located::new(unit_path.type_name(e.name().inner()), e.location()),
                    e.ty(),
                )
            }) {
                world.define(path.inner(), ty).map_err(|e| vec![e])?;
            }
        }

        for pkg in self.packages {
            let path = pkg.path();
            if path.is_absolute() {
                let path = AbsolutePackagePath(path.segments());
                world.define_package(path, pkg.metadata().clone());
            }
        }

        if errors.is_empty() {
            Ok(world)
        } else {
            Err(errors)
        }
    }
}

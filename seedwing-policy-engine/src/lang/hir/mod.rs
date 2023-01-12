use crate::core::Function;
use crate::lang::lir::ValueType;
use crate::lang::mir;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::{CompilationUnit, Located, PolicyParser, SourceLocation};
use crate::package::Package;
use crate::runtime::cache::SourceCache;
use crate::runtime::{BuildError, PackagePath, RuntimeError, TypeName};
use crate::value::RuntimeValue;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::iter::once;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TypeDefn {
    name: Located<String>,
    ty: Located<Type>,
    parameters: Vec<Located<String>>,
    documentation: Option<String>,
}

impl TypeDefn {
    pub fn new(name: Located<String>, ty: Located<Type>, parameters: Vec<Located<String>>) -> Self {
        Self {
            name,
            ty,
            parameters,
            documentation: None,
        }
    }

    pub fn set_documentation(&mut self, doc: Option<String>) {
        self.documentation = doc
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        self.ty.qualify_types(types);
    }

    pub(crate) fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Ref(Located<TypeName>, Vec<Located<Type>>),
    Parameter(Located<String>),
    Const(Located<ValueType>),
    Object(ObjectType),
    Expr(Located<Expr>),
    Join(Vec<Located<Type>>),
    Meet(Vec<Located<Type>>),
    Refinement(Box<Located<Type>>, Box<Located<Type>>),
    List(Vec<Located<Type>>),
    Nothing,
}

#[derive(Debug, Clone)]
pub enum MemberQualifier {
    All,
    Any,
    None,
    N(Located<u32>),
}

impl Type {
    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        match self {
            Type::Anything => Vec::default(),
            Type::Ref(inner, arguuments) => once(inner.clone())
                .chain(arguuments.iter().flat_map(|e| e.referenced_types()))
                .collect(),
            Type::Const(_) => Vec::default(),
            Type::Object(inner) => inner.referenced_types(),
            Type::Expr(_) => Vec::default(),
            Type::Join(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Type::Meet(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Type::Refinement(primary, refinement) => primary
                .referenced_types()
                .iter()
                .chain(refinement.referenced_types().iter())
                .cloned()
                .collect(),
            Type::List(terms) => terms.iter().flat_map(|e| e.referenced_types()).collect(),
            Type::Nothing => Vec::default(),
            Type::Parameter(_) => Vec::default(),
        }
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        match self {
            Type::Anything => {}
            Type::Ref(ref mut name, arguments) => {
                if !name.is_qualified() {
                    // it's a simple single-word name, needs qualifying, perhaps.
                    if let Some(Some(qualified)) = types.get(&name.name()) {
                        *name = qualified.clone();
                    }
                }
                for arg in arguments {
                    arg.qualify_types(types);
                }
            }
            Type::Const(_) => {}
            Type::Object(inner) => {
                inner.qualify_types(types);
            }
            Type::Expr(_) => {}
            Type::Join(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Type::Meet(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Type::Refinement(primary, refinement) => {
                primary.qualify_types(types);
                refinement.qualify_types(types);
            }
            Type::List(terms) => {
                terms.iter_mut().for_each(|e| e.qualify_types(types));
            }
            Type::Nothing => {}
            Type::Parameter(_) => {}
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "Anything"),
            Type::Ref(r, args) => write!(f, "{:?}<{:?}>", r, args),
            Type::Const(value) => write!(f, "{:?}", value),
            Type::Join(terms) => write!(f, "Join({:?})", terms),
            Type::Meet(terms) => write!(f, "Meet({:?})", terms),
            Type::Nothing => write!(f, "Nothing"),
            Type::Object(obj) => write!(f, "{:?}", obj),
            Type::Refinement(fn_name, ty) => write!(f, "{:?}({:?})", fn_name, ty),
            Type::List(ty) => write!(f, "[{:?}]", ty),
            Type::Expr(expr) => write!(f, "#({:?})", expr),
            Type::Parameter(name) => write!(f, "{:?}", name),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectType {
    fields: Vec<Located<Field>>,
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectType {
    pub fn new() -> Self {
        Self { fields: vec![] }
    }

    pub fn add_field(&mut self, field: Located<Field>) -> &Self {
        self.fields.push(field);
        self
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.fields
            .iter()
            .flat_map(|e| e.referenced_types())
            .collect()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        for field in &mut self.fields {
            field.qualify_types(types);
        }
    }

    pub fn fields(&self) -> &Vec<Located<Field>> {
        &self.fields
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    name: Located<String>,
    ty: Located<Type>,
}

impl Field {
    pub fn new(name: Located<String>, ty: Located<Type>) -> Self {
        Self { name, ty }
    }

    pub fn name(&self) -> &Located<String> {
        &self.name
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        self.ty.qualify_types(types)
    }
}

pub struct World {
    units: Vec<CompilationUnit>,
    packages: HashMap<PackagePath, Package>,
    source_cache: SourceCache,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let mut world = Self {
            units: Default::default(),
            packages: Default::default(),
            source_cache: Default::default(),
        };
        world.add_package(crate::core::list::package());
        world.add_package(crate::core::sigstore::package());
        world.add_package(crate::core::x509::package());
        world.add_package(crate::core::base64::package());
        world.add_package(crate::core::json::package());
        world.add_package(crate::core::pattern::package());

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

    fn add_compilation_unit(&mut self, unit: CompilationUnit) {
        self.units.push(unit)
    }

    pub fn add_package(&mut self, package: Package) {
        self.packages.insert(package.path(), package);
    }

    pub async fn lower(&mut self) -> Result<mir::World, Vec<BuildError>> {
        let mut core_units = Vec::new();

        let mut errors = Vec::new();

        for pkg in self.packages.values() {
            for (source, stream) in pkg.source_iter() {
                log::info!("loading {}", source);
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

        Lowerer::new(&mut self.units, &mut self.packages)
            .lower()
            .await
    }
}

struct Lowerer<'b> {
    units: &'b mut Vec<CompilationUnit>,
    packages: &'b mut HashMap<PackagePath, Package>,
}

impl<'b> Lowerer<'b> {
    pub fn new(
        units: &'b mut Vec<CompilationUnit>,
        packages: &'b mut HashMap<PackagePath, Package>,
    ) -> Self {
        Self { units, packages }
    }

    pub async fn lower(mut self) -> Result<mir::World, Vec<BuildError>> {
        // First, perform internal per-unit linkage and type qualification
        let mut world = mir::World::new();
        let mut errors = Vec::new();

        for mut unit in self.units.iter_mut() {
            let unit_path = PackagePath::from(unit.source());

            let mut visible_types = unit
                .uses()
                .iter()
                .map(|e| (e.as_name().inner(), Some(e.type_name())))
                .chain(unit.types().iter().map(|e| {
                    (
                        e.name().inner(),
                        Some(Located::new(
                            TypeName::new(None, e.name().inner()),
                            e.location(),
                        )),
                    )
                }))
                .collect::<HashMap<String, Option<Located<TypeName>>>>();

            //visible_types.insert("int".into(), None);
            for primordial in world.known_world() {
                visible_types.insert(primordial.name(), None);
            }

            for defn in unit.types() {
                visible_types.insert(
                    defn.name().inner(),
                    Some(Located::new(
                        unit_path.type_name(defn.name().inner()),
                        defn.location(),
                    )),
                );
            }

            for defn in unit.types() {
                let referenced_types = defn.referenced_types();

                for ty in &referenced_types {
                    if !ty.is_qualified() && !visible_types.contains_key(&ty.name()) {
                        errors.push(BuildError::TypeNotFound(
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

        //world.push(TypeName::new(None, "int".into()));

        //world.push("int".into());

        for (path, package) in self.packages.iter() {
            let package_path = path;

            known_world.extend_from_slice(
                &package
                    .function_names()
                    .iter()
                    .map(|e| package_path.type_name(e.clone()))
                    .collect::<Vec<TypeName>>(),
            );
        }

        for unit in self.units.iter() {
            let unit_path = PackagePath::from(unit.source());

            let unit_types = unit
                .types()
                .iter()
                .map(|e| unit_path.type_name(e.name().inner()))
                .collect::<Vec<TypeName>>();

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
                        errors.push(BuildError::TypeNotFound(
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
                world
                    .declare(name, ty.documentation.clone(), ty.parameters())
                    .await;
            }
        }

        for (path, package) in self.packages.iter() {
            for (fn_name, func) in package.functions() {
                let path = path.type_name(fn_name);
                world
                    .declare(
                        path,
                        func.documentation(),
                        func.parameters()
                            .iter()
                            .cloned()
                            .map(|p| Located::new(p, 0..0))
                            .collect(),
                    )
                    .await;
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        for (path, package) in self.packages.iter() {
            for (fn_name, func) in package.functions() {
                let path = path.type_name(fn_name);
                world.define_function(path, func).await;
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
                world.define(path.inner(), ty).await;
            }
        }

        if errors.is_empty() {
            Ok(world)
        } else {
            Err(errors)
        }
    }
}

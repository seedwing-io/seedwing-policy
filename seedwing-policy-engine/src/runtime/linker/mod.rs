use crate::lang::ty::{PackagePath, Type, TypeName};
use crate::lang::{CompilationUnit, Located};
use crate::package::Package;
use crate::runtime::{BuildError, Runtime, RuntimeType};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;

pub struct Linker {
    units: Vec<CompilationUnit>,
    packages: HashMap<PackagePath, Package>,
}

impl Linker {
    pub fn new(units: Vec<CompilationUnit>, packages: HashMap<PackagePath, Package>) -> Self {
        Self { units, packages }
    }

    pub async fn link(mut self) -> Result<Arc<Runtime>, Vec<BuildError>> {
        // First, perform internal per-unit linkage and type qualification
        for mut unit in &mut self.units {
            let unit_path = PackagePath::from(unit.source());

            let mut visible_types = unit
                .uses()
                .iter()
                .map(|e| (e.as_name().into_inner(), Some(e.type_name())))
                .chain(unit.types().iter().map(|e| {
                    (
                        e.name().into_inner(),
                        Some(Located::new(
                            TypeName::new(e.name().into_inner()),
                            e.location(),
                        )),
                    )
                }))
                .collect::<HashMap<String, Option<Located<TypeName>>>>();

            visible_types.insert("int".into(), None);

            for defn in unit.types() {
                visible_types.insert(
                    defn.name().clone().into_inner(),
                    Some(Located::new(
                        unit_path.type_name(defn.name().clone().into_inner()),
                        defn.location(),
                    )),
                );
            }

            for defn in unit.types() {
                println!("defn {:?}", defn);
                let referenced_types = defn.referenced_types();

                for ty in &referenced_types {
                    if !ty.is_qualified() && !visible_types.contains_key(&ty.name()) {
                        todo!("unknown type referenced {:?}", ty)
                    }
                }
            }

            println!("qualify with {:?}", visible_types);

            for defn in unit.types_mut() {
                defn.qualify_types(&visible_types)
            }
        }

        // next, perform inter-unit linking.

        let mut world = Vec::new();

        world.push(TypeName::new("int".into()));

        //world.push("int".into());

        for (path, package) in &self.packages {
            let package_path = path;

            world.extend_from_slice(
                &package
                    .function_names()
                    .iter()
                    .map(|e| package_path.type_name(e.clone()))
                    .collect::<Vec<TypeName>>(),
            );

            println!("{:?}", world);
        }

        for unit in &self.units {
            let unit_path = PackagePath::from(unit.source());
            println!("@@@@ {:?}", unit_path);

            let unit_types = unit
                .types()
                .iter()
                .map(|e| unit_path.type_name(e.name().into_inner()))
                .collect::<Vec<TypeName>>();

            world.extend_from_slice(&unit_types);
        }

        println!("world {:?}", world);
        for unit in &self.units {
            for defn in unit.types() {
                // these should be fully-qualified now
                let referenced = defn.referenced_types();

                for each in referenced {
                    if !world.contains(&each.clone().into_inner()) {
                        println!("{:?}", world);
                        todo!("failed to inter-unit link for {:?}", each)
                    }
                }
            }
        }

        //println!("{:?}", world);

        let mut runtime = Runtime::new();

        for unit in &self.units {
            let unit_path = PackagePath::from(unit.source());

            for (path, _) in unit.types().iter().map(|e| {
                (
                    Located::new(unit_path.type_name(e.name().into_inner()), e.location()),
                    e.ty(),
                )
            }) {
                runtime.declare(path.into_inner()).await;
            }
        }

        for (path, package) in &self.packages {
            for (fn_name, _) in package.functions() {
                let path = path.type_name(fn_name);
                runtime.declare(path).await;
            }
        }

        for unit in &self.units {
            let unit_path = PackagePath::from(unit.source());

            for (path, ty) in unit.types().iter().map(|e| {
                (
                    Located::new(unit_path.type_name(e.name().into_inner()), e.location()),
                    e.ty(),
                )
            }) {
                runtime.define(path.into_inner(), ty).await;
            }
        }

        for (path, package) in &self.packages {
            for (fn_name, func) in package.functions() {
                let path = path.type_name(fn_name);
                runtime.define_function(path, func).await;
            }
        }

        Ok(runtime)
    }
}

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use crate::lang::{CompilationUnit, Located, TypePath};
use crate::lang::ty::TypeName;
use crate::runtime::{BuildError, Runtime};

pub struct Linker {
    units: Vec<CompilationUnit>,
}

impl Linker {
    pub fn new(units: Vec<CompilationUnit>) -> Self {
        Self {
            units
        }
    }

    pub fn link(mut self) -> Result<Rc<Runtime>, Vec<BuildError>> {
        // First, perform internal per-unit linkage and type qualification
        for mut unit in &mut self.units {
            let unit_path = TypePath::from(unit.source());

            let mut visible_types = unit.uses().iter()

                .map(|e| {
                    if let Some(name) = e.as_name() {
                        (name.clone().into_inner(), Some(e.type_path()))
                    } else {
                        (e.type_path().type_name().clone().into_inner(), None)
                    }
                })
                .chain(
                    unit.types().iter()
                        .map(|e| {
                            (
                                e.name().into_inner(),
                                Some(
                                    Located::new(
                                        unit_path.join(e.name()),
                                        e.location(),
                                    )
                                )
                            )
                        })).
                collect::<HashMap<TypeName, Option<Located<TypePath>>>>();

            visible_types.insert(TypeName::new("int".into()), None);

            for defn in unit.types() {
                let referenced_types = defn.referenced_types();

                for ty in &referenced_types {
                    if ty.is_simple() {
                        if !visible_types.contains_key(ty.type_name()) {
                            todo!("unknown type referenced {:?}", ty)
                        }
                    }
                }
            }

            for defn in unit.types_mut() {
                defn.qualify_types(&visible_types)
            }
        }

        // next, perform inter-unit linking.

        let mut world = Vec::new();

        world.push("int".into());

        for unit in &self.units {
            let unit_path = TypePath::from(unit.source());

            let unit_types = unit.types().iter()
                .map(|e| {
                    unit_path.join(e.name()).as_path_str()
                })
                .collect::<Vec<String>>();

            world.extend_from_slice(&unit_types);
        }

        for unit in &self.units {
            for defn in unit.types() {
                // these should be fully-qualified now
                let referenced = defn.referenced_types();

                for each in referenced {
                    if !world.contains(&each.as_path_str()) {
                        println!("{:?}", world);
                        todo!("failed to inter-unit link for {:?}", each.as_path_str());
                    }
                }
            }
        }

        //println!("{:?}", world);

        let mut runtime = Runtime::new();

        for unit in &self.units {
            let unit_path = TypePath::from(unit.source());

            unit.types().iter()
                .map(|e| {
                    (Located::new(
                        unit_path.join(e.name()),
                        e.location(),
                    ), e.ty())
                })
                .for_each(|(path, ty)| {
                    runtime.define(path.into_inner(), ty);
                })
        }

        Ok(runtime)
    }
}
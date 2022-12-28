use crate::core::Function;
use crate::lang::parser::SourceLocation;
use crate::lang::PackagePath;
use crate::runtime::sources::Ephemeral;
use std::collections::HashMap;
use std::sync::Arc;

pub struct PackageSource {
    name: String,
    content: &'static str,
}

pub struct Package {
    path: PackagePath,
    functions: HashMap<String, Arc<dyn Function>>,
    sources: Vec<PackageSource>,
}

impl Package {
    pub fn new(path: PackagePath) -> Self {
        Self {
            path,
            functions: Default::default(),
            sources: Default::default(),
        }
    }

    pub fn path(&self) -> PackagePath {
        self.path.clone()
    }

    pub fn register_function<F: Function + 'static>(&mut self, name: String, func: F) {
        self.functions.insert(name, Arc::new(func));
    }

    pub fn register_source(&mut self, name: String, content: &'static str) {
        self.sources.push(PackageSource { name, content })
    }

    pub fn source_iter(&self) -> impl Iterator<Item = (SourceLocation, String)> + '_ {
        self.sources.iter().map(|src| {
            let mut source = self.path.as_package_str();
            source.push_str("::");
            source.push_str(src.name.as_str());

            let stream = src.content.into();

            (source.into(), stream)
        })
    }

    pub fn function_names(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    pub fn functions(&self) -> Vec<(String, Arc<dyn Function>)> {
        self.functions
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

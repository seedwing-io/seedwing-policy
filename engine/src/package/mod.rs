use crate::core::Function;
use crate::lang::parser::SourceLocation;
use crate::lang::PackageMeta;
use crate::runtime::PackagePath;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct PackageSource {
    name: String,
    content: &'static str,
}

#[derive(Clone)]
pub struct Package {
    path: PackagePath,
    functions: HashMap<String, Arc<dyn Function>>,
    sources: Vec<PackageSource>,
    metadata: PackageMeta,
}

impl Package {
    pub fn new(path: PackagePath) -> Self {
        Self {
            path,
            functions: Default::default(),
            sources: Default::default(),
            metadata: Default::default(),
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
            if !source.is_empty() && !src.name.as_str().is_empty() {
                // only add :: if both sides are not empty.
                source.push_str("::");
            }
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

    pub fn metadata(&self) -> &PackageMeta {
        &self.metadata
    }

    pub fn with_metadata(self, metadata: PackageMeta) -> Self {
        Self { metadata, ..self }
    }

    pub fn with_documentation(self, documentation: impl Into<Option<String>>) -> Self {
        self.with_metadata(PackageMeta {
            documentation: documentation.into(),
        })
    }
}

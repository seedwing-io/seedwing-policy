use crate::core::Function;
use crate::lang::parser::{Located, SourceLocation};
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::sync::Arc;

pub mod builder;
pub mod hir;
pub mod lir;
pub mod mir;
pub mod parser;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TypeName {
    package: Option<PackagePath>,
    name: String,
}

impl Display for TypeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_type_str())
    }
}

impl TypeName {
    pub fn new(package: Option<PackagePath>, name: String) -> Self {
        Self { package, name }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn is_qualified(&self) -> bool {
        self.package.is_some()
    }

    pub fn as_type_str(&self) -> String {
        let mut fq = String::new();
        if let Some(package) = &self.package {
            fq.push_str(&package.as_package_str());
            fq.push_str("::");
        }

        fq.push_str(&self.name);

        fq
    }
}

impl From<String> for TypeName {
    fn from(path: String) -> Self {
        let mut segments = path.split("::").map(|e| e.into()).collect::<Vec<String>>();
        if segments.is_empty() {
            Self::new(None, "".into())
        } else {
            let tail = segments.pop().unwrap();
            if segments.is_empty() {
                Self {
                    package: None,
                    name: tail,
                }
            } else {
                let package = Some(segments.into());
                Self {
                    package,
                    name: tail,
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PackageName(String);

impl PackageName {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

impl Deref for PackageName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PackagePath {
    is_absolute: bool,
    path: Vec<Located<PackageName>>,
}

impl From<&str> for PackagePath {
    fn from(segments: &str) -> Self {
        let segments: Vec<String> = segments.split("::").map(|e| e.into()).collect();
        segments.into()
    }
}

impl From<String> for PackagePath {
    fn from(segments: String) -> Self {
        let segments: Vec<String> = segments.split("::").map(|e| e.into()).collect();
        segments.into()
    }
}

impl From<Vec<String>> for PackagePath {
    fn from(mut segments: Vec<String>) -> Self {
        let first = segments.get(0).unwrap();
        let is_absolute = first.is_empty();
        if is_absolute {
            segments = segments[1..].to_vec()
        }

        Self {
            is_absolute: true,
            path: segments
                .iter()
                .map(|e| Located::new(PackageName(e.clone()), 0..0))
                .collect(),
        }
    }
}

impl From<Vec<Located<PackageName>>> for PackagePath {
    fn from(mut segments: Vec<Located<PackageName>>) -> Self {
        Self {
            is_absolute: true,
            path: segments,
        }
    }
}

impl PackagePath {
    pub fn from_parts(segments: Vec<&str>) -> Self {
        Self {
            is_absolute: true,
            path: segments
                .iter()
                .map(|e| Located::new(PackageName(String::from(*e)), 0..0))
                .collect(),
        }
    }

    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    pub fn is_qualified(&self) -> bool {
        self.path.len() > 1
    }

    pub fn type_name(&self, name: String) -> TypeName {
        TypeName::new(Some(self.clone()), name)
    }

    pub fn as_package_str(&self) -> String {
        let mut fq = String::new();

        fq.push_str(
            &self
                .path
                .iter()
                .map(|e| e.inner().0)
                .collect::<Vec<String>>()
                .join("::"),
        );

        fq
    }

    pub fn path(&self) -> &Vec<Located<PackageName>> {
        &self.path
    }
}

impl From<SourceLocation> for PackagePath {
    fn from(src: SourceLocation) -> Self {
        let name = src.name().replace('/', "::");
        let segments = name
            .split("::")
            .map(|segment| Located::new(PackageName(segment.into()), 0..0))
            .collect();

        Self {
            is_absolute: true,
            path: segments,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(TypeName, Arc<dyn Function>),
}

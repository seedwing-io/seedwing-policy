//! Policy source types.
use crate::lang::parser::SourceLocation;
use crate::runtime::PackagePath;
use std::fs::File;
use std::io::Read;
use std::iter::once;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// An in-memory source.
#[derive(Clone)]
pub struct Ephemeral {
    source: SourceLocation,
    content: String,
}

impl Ephemeral {
    pub fn new<P: Into<PackagePath>, C: Into<String>>(package: P, content: C) -> Self {
        Self {
            source: package.into().into(),
            content: content.into(),
        }
    }

    pub fn iter(self) -> impl Iterator<Item = (SourceLocation, String)> {
        once((self.source.clone(), self.content))
    }
}

/// A directory with policy files.
#[derive(Debug, Clone)]
pub struct Directory {
    dir: PathBuf,
}

impl Directory {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (SourceLocation, String)> + '_ {
        WalkDir::new(&self.dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy();
                if !name.ends_with(".dog") {
                    None
                } else {
                    let path = e.path();
                    if let Ok(path) = path.strip_prefix::<&Path>(&self.dir) {
                        let mut src = String::new();
                        if let Some(part) = path.parent() {
                            if !part.to_string_lossy().is_empty() {
                                src.push_str(&part.to_string_lossy());
                                src.push('/');
                            }
                        }
                        src.push_str(name.strip_suffix(".dog").unwrap());
                        if let Ok(mut file) = File::open(e.path()) {
                            let mut content = String::new();
                            file.read_to_string(&mut content);
                            Some((src.into(), content))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            })
    }
}

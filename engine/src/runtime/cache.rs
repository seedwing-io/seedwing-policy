//! A cache of all pattern sources.
use crate::lang::parser::SourceLocation;
use ariadne::{Cache, Source};
use std::collections::HashMap;
use std::fmt::{Debug, Display};

/// A cache of all pattern sources.
#[derive(Default)]
pub struct SourceCache {
    cache: HashMap<SourceLocation, Source>,
}

impl SourceCache {
    /// Create a new source cache.
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
        }
    }

    /// Add a source in a location to the cache.
    pub fn add(&mut self, id: SourceLocation, source: Source) {
        self.cache.insert(id, source);
    }
}

impl Cache<SourceLocation> for &SourceCache {
    fn fetch(&mut self, id: &SourceLocation) -> Result<&Source, Box<dyn Debug + '_>> {
        let source = self.cache.get(id);

        if let Some(source) = source {
            Ok(source)
        } else {
            Err(Box::new("No such source"))
        }
    }

    fn display<'a>(&self, id: &'a SourceLocation) -> Option<Box<dyn Display + 'a>> {
        Some(Box::new(id))
    }
}

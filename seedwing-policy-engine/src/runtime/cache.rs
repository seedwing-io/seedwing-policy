use crate::lang::parser::SourceLocation;
use ariadne::{Cache, Source};
use indexmap::IndexMap;
use std::fmt::{Debug, Display};

#[derive(Default)]
pub struct SourceCache {
    cache: IndexMap<SourceLocation, Source>,
}

impl SourceCache {
    pub fn new() -> Self {
        Self {
            cache: Default::default(),
        }
    }

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

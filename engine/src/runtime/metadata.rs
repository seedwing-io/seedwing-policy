use crate::lang;
use crate::lang::{lir, Expr, PackageMeta, PatternMeta, SyntacticSugar, ValuePattern};
use crate::runtime::{Example, PackagePath, Pattern, PatternName};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ComponentMetadata {
    Package(PackageMetadata),
    Pattern(PatternMetadata),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Documentation(pub Option<String>);

impl Display for Documentation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(docs) = &self.0 {
            f.write_str(docs)?;
        }
        Ok(())
    }
}

impl From<String> for Documentation {
    fn from(value: String) -> Self {
        Self(Some(value))
    }
}

impl From<&str> for Documentation {
    fn from(value: &str) -> Self {
        Self(Some(value.to_string()))
    }
}

impl From<Option<String>> for Documentation {
    fn from(value: Option<String>) -> Self {
        Self(value)
    }
}

impl Deref for Documentation {
    type Target = Option<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Documentation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Documentation {
    pub fn split(&self) -> (&str, &str) {
        match &self.0 {
            Some(docs) => docs.split_once("\n\n").unwrap_or((docs, "")),
            None => ("", ""),
        }
    }

    pub fn summary(&self) -> &str {
        self.split().0
    }

    pub fn details(&self) -> &str {
        self.split().1
    }

    pub fn summary_opt(&self) -> Option<&str> {
        let s = self.split().0;
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    pub fn details_opt(&self) -> Option<&str> {
        let s = self.split().1;
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PackageMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Documentation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<SubpackageMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<PatternMetadata>,
}

impl PackageMetadata {
    pub fn new(name: PackagePath) -> Self {
        Self {
            name: name.as_package_str(),
            documentation: Default::default(),
            packages: vec![],
            patterns: vec![],
        }
    }

    pub(crate) fn apply_meta(&mut self, meta: &PackageMeta) {
        self.documentation = Documentation(meta.documentation.clone());
    }

    pub(crate) fn sort(&mut self) {
        self.packages.sort_unstable_by(|l, r| l.name.cmp(&r.name));
        self.patterns.sort_unstable_by(|l, r| l.name.cmp(&r.name));
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SubpackageMetadata {
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Documentation,
}

impl SubpackageMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            documentation: Default::default(),
        }
    }

    pub(crate) fn apply_meta(&mut self, meta: &PackageMeta) {
        self.documentation = Documentation(meta.documentation.clone());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PatternMetadata {
    pub name: Option<String>,
    pub path: Option<String>,
    pub metadata: PatternMeta,
    pub parameters: Vec<String>,
    pub inner: InnerPatternMetadata,
    pub examples: Vec<Example>,
}

/// Pattern information specific to different pattern types.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum InnerPatternMetadata {
    /// Anything.
    Anything,
    /// Primordial.
    Primordial(PrimordialPattern),
    /// Bound.
    Bound(PatternOrReference, Bindings),
    /// Reference.
    Ref(
        SyntacticSugar,
        PatternOrReference,
        #[serde(default, skip_serializing_if = "Vec::is_empty")] Vec<PatternOrReference>,
    ),
    /// Deref.
    Deref(PatternOrReference),
    /// Argument.
    Argument(String),
    /// Const.
    Const(ValuePattern),
    /// Object.
    Object(ObjectPattern),
    /// Expression.
    Expr(Expr),
    /// List.
    List(#[serde(default, skip_serializing_if = "Vec::is_empty")] Vec<PatternOrReference>),
    /// Nothing.
    Nothing,
}

/// Bindings used to retrieve parameters during evaluation.
#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Bindings {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bindings: HashMap<String, PatternOrReference>,
}

/// Primordial patterns are the basic building blocks of all patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimordialPattern {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(SyntacticSugar, PatternRef),
}

/// Reference to a pattern in another package.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PatternRef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Package of referenced pattern.
    pub package: Vec<String>,
    /// Name of referenced pattern.
    pub name: String,
}

/// An object pattern contains fields that are patterns or reference to other patterns.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ObjectPattern {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Object fields.
    pub fields: Vec<Field>,
}

/// A field is a pattern within an object.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Pattern for a given field.
    pub ty: PatternOrReference,
    /// Whether the field is optional or not.
    pub optional: bool,
}

/// A pattern or a reference to another pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PatternOrReference {
    Pattern(Arc<InnerPatternMetadata>),
    Ref(PatternRef),
}

/// Errors when generating information.
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown pattern slot: {0}")]
    UnknownPatternSlot(usize),
}

pub trait WorldLike {
    fn get_by_slot(&self, slot: usize) -> Option<Arc<Pattern>>;
}

/// Convert type information into an `*Metadata` struct.
pub trait ToMetadata<T> {
    /// Convert internal type information into an `*Metadata` struct.
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<T, Error>;
}

impl ToMetadata<PatternMetadata> for Pattern {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<PatternMetadata, Error> {
        Ok(PatternMetadata {
            metadata: self.metadata().clone(),
            parameters: self.parameters().clone(),
            name: self.name().map(|name| name.name().into()),
            path: self.name().map(|name| name.as_type_str()),
            inner: self.inner().to_meta(world)?,
            examples: self.examples(),
        })
    }
}

impl ToMetadata<InnerPatternMetadata> for lir::InnerPattern {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<InnerPatternMetadata, Error> {
        Ok(match self {
            Self::Anything => InnerPatternMetadata::Anything,
            Self::Primordial(r#type) => InnerPatternMetadata::Primordial(r#type.to_meta(world)?),
            Self::Bound(r#type, bindings) => {
                InnerPatternMetadata::Bound(r#type.to_meta(world)?, bindings.to_meta(world)?)
            }
            Self::Ref(sugar, slot, types) => InnerPatternMetadata::Ref(
                sugar.clone(),
                world
                    .get_by_slot(*slot)
                    .ok_or(Error::UnknownPatternSlot(*slot))?
                    .to_meta(world)?,
                types
                    .iter()
                    .map(|t| t.to_meta(world))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Deref(r#type) => InnerPatternMetadata::Deref(r#type.to_meta(world)?),
            Self::Argument(name) => InnerPatternMetadata::Argument(name.clone()),
            Self::Const(r#type) => InnerPatternMetadata::Const(r#type.clone()),
            Self::Object(r#type) => InnerPatternMetadata::Object(r#type.to_meta(world)?),
            Self::Expr(expr) => InnerPatternMetadata::Expr(expr.deref().clone()),
            Self::List(types) => InnerPatternMetadata::List(
                types
                    .iter()
                    .map(|t| t.to_meta(world))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Nothing => InnerPatternMetadata::Nothing,
        })
    }
}

impl ToMetadata<PrimordialPattern> for lang::PrimordialPattern {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<PrimordialPattern, Error> {
        Ok(match self {
            Self::Integer => PrimordialPattern::Integer,
            Self::Decimal => PrimordialPattern::Decimal,
            Self::Boolean => PrimordialPattern::Boolean,
            Self::String => PrimordialPattern::String,
            Self::Function(sugar, r#type, _) => {
                PrimordialPattern::Function(sugar.clone(), r#type.to_meta(world)?)
            }
        })
    }
}

impl ToMetadata<PatternRef> for PatternName {
    fn to_meta<W: WorldLike>(&self, _world: &W) -> Result<PatternRef, Error> {
        Ok(PatternRef {
            name: self.name().to_string(),
            package: self.package().map(|p| p.segments()).unwrap_or_default(),
        })
    }
}

impl ToMetadata<PatternOrReference> for Pattern {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<PatternOrReference, Error> {
        Ok(match self.name() {
            Some(name) => PatternOrReference::Ref(name.to_meta(world)?),
            None => PatternOrReference::Pattern(Arc::new(self.inner().to_meta(world)?)),
        })
    }
}

impl ToMetadata<Bindings> for lir::Bindings {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<Bindings, Error> {
        Ok(Bindings {
            bindings: self
                .iter()
                .map(|(k, v)| v.to_meta(world).map(|v| (k.clone(), v)))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ToMetadata<ObjectPattern> for lir::ObjectPattern {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<ObjectPattern, Error> {
        Ok(ObjectPattern {
            fields: self
                .fields()
                .iter()
                .map(|f| f.to_meta(world))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ToMetadata<Field> for lir::Field {
    fn to_meta<W: WorldLike>(&self, world: &W) -> Result<Field, Error> {
        Ok(Field {
            ty: self.ty().to_meta(world)?,
            optional: self.optional(),
            name: self.name(),
        })
    }
}

use crate::lang;
use crate::lang::lir::InnerPattern;
use crate::lang::{lir, Expr, SyntacticSugar, ValuePattern};
use crate::runtime::{Example, PackageName, PackagePath, Pattern, PatternName, World};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ComponentMetadata {
    Package(PackageMetadata),
    Pattern(PatternMetadata),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PackageMetadata {
    pub name: String,
    pub documentation: Option<String>,
    pub packages: Vec<SubpackageMetadata>,
    pub patterns: Vec<PatternMetadata>,
}

impl PackageMetadata {
    pub fn new(name: PackagePath) -> Self {
        Self {
            name: name.as_package_str(),
            documentation: None,
            packages: vec![],
            patterns: vec![],
        }
    }

    pub fn add_pattern(&mut self, pattern: PatternMetadata) {
        if !&self.patterns.iter().any(|e| e.name == pattern.name) {
            self.patterns.push(pattern);
        }
    }

    pub fn add_subpackage(&mut self, name: PackageName) {
        if !self.packages.iter().any(|e| e.name == name.0) {
            self.packages.push(SubpackageMetadata {
                name: name.0,
                documentation: None,
            })
        }
    }

    pub fn sort(mut self) -> Self {
        self.packages.sort_by(|l, r| l.name.cmp(&r.name));
        self.patterns.sort_by(|l, r| l.name.cmp(&r.name));

        self
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct SubpackageMetadata {
    pub name: String,
    pub documentation: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct PatternMetadata {
    pub name: Option<String>,
    pub path: Option<String>,
    pub documentation: Option<String>,
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
    Pattern(Rc<InnerPatternMetadata>),
    Ref(PatternRef),
}

/// Errors when generating information.
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown pattern slot: {0}")]
    UnknownPatternSlot(usize),
}

/// Convert type information into an `*Metadata` struct.
pub trait ToMetadata<T> {
    /// Convert internal type information into an `*Metadata` struct.
    fn to_meta(&self, world: &World) -> Result<T, Error>;
}

impl ToMetadata<PatternMetadata> for Pattern {
    fn to_meta(&self, world: &World) -> Result<PatternMetadata, Error> {
        Ok(PatternMetadata {
            documentation: self.documentation(),
            parameters: self.parameters(),
            name: self.name().map(|name| name.name().into()),
            path: self.name().map(|name| name.as_type_str()),
            inner: self.inner().to_meta(world)?,
            examples: self.examples(),
        })
    }
}

impl ToMetadata<InnerPatternMetadata> for InnerPattern {
    fn to_meta(&self, world: &World) -> Result<InnerPatternMetadata, Error> {
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
    fn to_meta(&self, world: &World) -> Result<PrimordialPattern, Error> {
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
    fn to_meta(&self, _world: &World) -> Result<PatternRef, Error> {
        Ok(PatternRef {
            name: self.name().to_string(),
            package: self.package().map(|p| p.segments()).unwrap_or_default(),
        })
    }
}

impl ToMetadata<PatternOrReference> for Pattern {
    fn to_meta(&self, world: &World) -> Result<PatternOrReference, Error> {
        Ok(match self.name() {
            Some(name) => PatternOrReference::Ref(name.to_meta(world)?),
            None => PatternOrReference::Pattern(Rc::new(self.inner().to_meta(world)?)),
        })
    }
}

impl ToMetadata<Bindings> for lir::Bindings {
    fn to_meta(&self, world: &World) -> Result<Bindings, Error> {
        Ok(Bindings {
            bindings: self
                .iter()
                .map(|(k, v)| v.to_meta(world).map(|v| (k.clone(), v)))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ToMetadata<ObjectPattern> for lir::ObjectPattern {
    fn to_meta(&self, world: &World) -> Result<ObjectPattern, Error> {
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
    fn to_meta(&self, world: &World) -> Result<Field, Error> {
        Ok(Field {
            ty: self.ty().to_meta(world)?,
            optional: self.optional(),
            name: self.name(),
        })
    }
}

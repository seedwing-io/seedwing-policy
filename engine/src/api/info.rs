use crate::{
    lang::{
        self,
        lir::{self, InnerPattern},
        lir::{Expr, Pattern, ValuePattern},
        SyntacticSugar,
    },
    runtime::{Component, ModuleHandle, PatternName, World},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Deref, rc::Rc};

#[allow(missing_docs)]
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown pattern slot: {0}")]
    UnknownPatternSlot(usize),
}

/// Convert type information into an `*Information` struct.
pub trait ToInformation<T> {
    /// Convert internal type information into an `*Information` struct.
    fn to_info(&self, world: &World) -> Result<T, Error>;
}

/// Information for different component types.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ComponentInformation {
    /// Module component type.
    Module(ModuleHandle),
    /// Pattern component type.
    Pattern(PatternInformation),
}

impl ToInformation<ComponentInformation> for Component {
    fn to_info(&self, world: &World) -> Result<ComponentInformation, Error> {
        Ok(match self {
            Self::Module(module) => ComponentInformation::Module(module.clone()),
            Self::Pattern(pattern) => ComponentInformation::Pattern(pattern.to_info(world)?),
        })
    }
}

/// Information about a pattern.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PatternInformation {
    /// Pattern name.
    pub name: Option<String>,
    /// Pattern documentation.
    pub documentation: Option<String>,
    /// Pattern parameters.
    pub parameters: Vec<String>,
    /// Inner pattern information.
    pub inner: InnerPatternInformation,
}

impl ToInformation<PatternInformation> for Pattern {
    fn to_info(&self, world: &World) -> Result<PatternInformation, Error> {
        Ok(PatternInformation {
            documentation: self.documentation(),
            parameters: self.parameters(),
            name: self.name().map(|name| name.as_type_str()),
            inner: self.inner().to_info(world)?,
        })
    }
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

/// Pattern information specific to different pattern types.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum InnerPatternInformation {
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

/// Primordial patterns are the basic building blocks of all patterns.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimordialPattern {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(SyntacticSugar, PatternRef),
}

/// Bindings used to retrieve parameters during evaluation.
#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Bindings {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub bindings: HashMap<String, PatternOrReference>,
}

impl ToInformation<InnerPatternInformation> for InnerPattern {
    fn to_info(&self, world: &World) -> Result<InnerPatternInformation, Error> {
        Ok(match self {
            Self::Anything => InnerPatternInformation::Anything,
            Self::Primordial(r#type) => InnerPatternInformation::Primordial(r#type.to_info(world)?),
            Self::Bound(r#type, bindings) => {
                InnerPatternInformation::Bound(r#type.to_info(world)?, bindings.to_info(world)?)
            }
            Self::Ref(sugar, slot, types) => InnerPatternInformation::Ref(
                sugar.clone(),
                world
                    .get_by_slot(*slot)
                    .ok_or_else(|| Error::UnknownPatternSlot(*slot))?
                    .to_info(world)?,
                types
                    .iter()
                    .map(|t| t.to_info(world))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Deref(r#type) => InnerPatternInformation::Deref(r#type.to_info(world)?),
            Self::Argument(name) => InnerPatternInformation::Argument(name.clone()),
            Self::Const(r#type) => InnerPatternInformation::Const(r#type.clone()),
            Self::Object(r#type) => InnerPatternInformation::Object(r#type.to_info(world)?),
            Self::Expr(expr) => InnerPatternInformation::Expr(expr.deref().clone()),
            Self::List(types) => InnerPatternInformation::List(
                types
                    .iter()
                    .map(|t| t.to_info(world))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Nothing => InnerPatternInformation::Nothing,
        })
    }
}

impl ToInformation<PrimordialPattern> for lang::PrimordialPattern {
    fn to_info(&self, world: &World) -> Result<PrimordialPattern, Error> {
        Ok(match self {
            Self::Integer => PrimordialPattern::Integer,
            Self::Decimal => PrimordialPattern::Decimal,
            Self::Boolean => PrimordialPattern::Boolean,
            Self::String => PrimordialPattern::String,
            Self::Function(sugar, r#type, _) => {
                PrimordialPattern::Function(sugar.clone(), r#type.to_info(world)?)
            }
        })
    }
}

impl ToInformation<PatternRef> for PatternName {
    fn to_info(&self, _world: &World) -> Result<PatternRef, Error> {
        Ok(PatternRef {
            name: self.name().to_string(),
            package: self.package().map(|p| p.segments()).unwrap_or_default(),
        })
    }
}

impl ToInformation<PatternOrReference> for Pattern {
    fn to_info(&self, world: &World) -> Result<PatternOrReference, Error> {
        Ok(match self.name() {
            Some(name) => PatternOrReference::Ref(name.to_info(world)?),
            None => PatternOrReference::Pattern(Rc::new(self.inner().to_info(world)?)),
        })
    }
}

/// A pattern or a reference to another pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PatternOrReference {
    Pattern(Rc<InnerPatternInformation>),
    Ref(PatternRef),
}

impl ToInformation<Bindings> for lir::Bindings {
    fn to_info(&self, world: &World) -> Result<Bindings, Error> {
        Ok(Bindings {
            bindings: self
                .iter()
                .map(|(k, v)| v.to_info(world).map(|v| (k.clone(), v)))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ToInformation<ObjectPattern> for lir::ObjectPattern {
    fn to_info(&self, world: &World) -> Result<ObjectPattern, Error> {
        Ok(ObjectPattern {
            fields: self
                .fields()
                .iter()
                .map(|f| f.to_info(world))
                .collect::<Result<_, _>>()?,
        })
    }
}

impl ToInformation<Field> for lir::Field {
    fn to_info(&self, world: &World) -> Result<Field, Error> {
        Ok(Field {
            ty: self.ty().to_info(world)?,
            optional: self.optional(),
            name: self.name(),
        })
    }
}

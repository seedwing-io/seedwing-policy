use crate::{
    lang::{
        self,
        lir::{self, Expr, InnerType, Type, ValueType},
        SyntacticSugar,
    },
    runtime::{Component, ModuleHandle, TypeName, World},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Deref, rc::Rc};

#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown type slot: {0}")]
    UnknownTypeSlot(usize),
}

pub trait ToInformation<T> {
    /// Convert internal type information into an `*Information` struct.
    fn to_info(&self, world: &World) -> Result<T, Error>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ComponentInformation {
    Module(ModuleHandle),
    Type(TypeInformation),
}

impl ToInformation<ComponentInformation> for Component {
    fn to_info(&self, world: &World) -> Result<ComponentInformation, Error> {
        Ok(match self {
            Self::Module(module) => ComponentInformation::Module(module.clone()),
            Self::Type(r#type) => ComponentInformation::Type(r#type.to_info(world)?),
        })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct TypeInformation {
    pub name: Option<String>,
    pub documentation: Option<String>,
    pub parameters: Vec<String>,
    pub inner: InnerTypeInformation,
}

impl ToInformation<TypeInformation> for Type {
    fn to_info(&self, world: &World) -> Result<TypeInformation, Error> {
        Ok(TypeInformation {
            documentation: self.documentation(),
            parameters: self.parameters(),
            name: self.name().map(|name| name.as_type_str()),
            inner: self.inner().to_info(world)?,
        })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct TypeRef {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub package: Vec<String>,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ObjectType {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fields: Vec<Field>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Field {
    name: String,
    ty: TypeOrReference,
    optional: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum InnerTypeInformation {
    Anything,
    Primordial(PrimordialType),
    Bound(TypeOrReference, Bindings),
    Ref(
        SyntacticSugar,
        TypeOrReference,
        #[serde(default, skip_serializing_if = "Vec::is_empty")] Vec<TypeOrReference>,
    ),
    Deref(TypeOrReference),
    Argument(String),
    Const(ValueType),
    Object(ObjectType),
    Expr(Expr),
    List(#[serde(default, skip_serializing_if = "Vec::is_empty")] Vec<TypeOrReference>),
    Nothing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(SyntacticSugar, TypeRef),
}

#[derive(Clone, Serialize, Deserialize, Default, Debug, PartialEq)]
pub struct Bindings {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    bindings: HashMap<String, TypeOrReference>,
}

impl ToInformation<InnerTypeInformation> for InnerType {
    fn to_info(&self, world: &World) -> Result<InnerTypeInformation, Error> {
        Ok(match self {
            Self::Anything => InnerTypeInformation::Anything,
            Self::Primordial(r#type) => InnerTypeInformation::Primordial(r#type.to_info(world)?),
            Self::Bound(r#type, bindings) => {
                InnerTypeInformation::Bound(r#type.to_info(world)?, bindings.to_info(world)?)
            }
            Self::Ref(sugar, slot, types) => InnerTypeInformation::Ref(
                sugar.clone(),
                world
                    .get_by_slot(*slot)
                    .ok_or_else(|| Error::UnknownTypeSlot(*slot))?
                    .to_info(world)?,
                types
                    .iter()
                    .map(|t| t.to_info(world))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Deref(r#type) => InnerTypeInformation::Deref(r#type.to_info(world)?),
            Self::Argument(name) => InnerTypeInformation::Argument(name.clone()),
            Self::Const(r#type) => InnerTypeInformation::Const(r#type.clone()),
            Self::Object(r#type) => InnerTypeInformation::Object(r#type.to_info(world)?),
            Self::Expr(expr) => InnerTypeInformation::Expr(expr.deref().clone()),
            Self::List(types) => InnerTypeInformation::List(
                types
                    .iter()
                    .map(|t| t.to_info(world))
                    .collect::<Result<_, _>>()?,
            ),
            Self::Nothing => InnerTypeInformation::Nothing,
        })
    }
}

impl ToInformation<PrimordialType> for lang::PrimordialType {
    fn to_info(&self, world: &World) -> Result<PrimordialType, Error> {
        Ok(match self {
            Self::Integer => PrimordialType::Integer,
            Self::Decimal => PrimordialType::Decimal,
            Self::Boolean => PrimordialType::Boolean,
            Self::String => PrimordialType::String,
            Self::Function(sugar, r#type, _) => {
                PrimordialType::Function(sugar.clone(), r#type.to_info(world)?)
            }
        })
    }
}

impl ToInformation<TypeRef> for TypeName {
    fn to_info(&self, _world: &World) -> Result<TypeRef, Error> {
        Ok(TypeRef {
            name: self.name().to_string(),
            package: self.package().map(|p| p.segments()).unwrap_or_default(),
        })
    }
}

impl ToInformation<TypeOrReference> for Type {
    fn to_info(&self, world: &World) -> Result<TypeOrReference, Error> {
        Ok(match self.name() {
            Some(name) => TypeOrReference::Ref(name.to_info(world)?),
            None => TypeOrReference::Type(Rc::new(self.inner().to_info(world)?)),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TypeOrReference {
    Type(Rc<InnerTypeInformation>),
    Ref(TypeRef),
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

impl ToInformation<ObjectType> for lir::ObjectType {
    fn to_info(&self, world: &World) -> Result<ObjectType, Error> {
        Ok(ObjectType {
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

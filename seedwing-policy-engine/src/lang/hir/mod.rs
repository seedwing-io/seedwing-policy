use crate::core::Function;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::TypeName;
use crate::runtime::{Bindings, EvaluationResult, RuntimeError, TypeHandle};
use crate::value::Value;
use async_mutex::Mutex;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::iter::once;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct TypeDefn {
    name: Located<String>,
    ty: Located<Type>,
    parameters: Vec<Located<String>>,
}

impl TypeDefn {
    pub fn new(name: Located<String>, ty: Located<Type>, parameters: Vec<Located<String>>) -> Self {
        Self {
            name,
            ty,
            parameters,
        }
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        self.ty.qualify_types(types);
    }

    pub(crate) fn parameters(&self) -> Vec<Located<String>> {
        self.parameters.clone()
    }
}

#[derive(Clone)]
pub enum Type {
    Anything,
    Ref(Located<TypeName>, Vec<Located<Type>>),
    Parameter(Located<String>),
    Const(Located<Value>),
    Object(ObjectType),
    Expr(Located<Expr>),
    Join(Box<Located<Type>>, Box<Located<Type>>),
    Meet(Box<Located<Type>>, Box<Located<Type>>),
    Refinement(Box<Located<Type>>, Box<Located<Type>>),
    List(Box<Located<Type>>),
    // todo: replace with simple functions
    MemberQualifier(Located<MemberQualifier>, Box<Located<Type>>),
    Nothing,
}

// todo: replace with simple functions
#[derive(Debug, Clone)]
pub enum MemberQualifier {
    All,
    Any,
    N(Located<u32>),
}

impl Type {
    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        match self {
            Type::Anything => Vec::default(),
            Type::Ref(inner, arguuments) => once(inner.clone())
                .chain(arguuments.iter().flat_map(|e| e.referenced_types()))
                .collect(),
            Type::Const(_) => Vec::default(),
            Type::Object(inner) => inner.referenced_types(),
            Type::Expr(_) => Vec::default(),
            Type::Join(lhs, rhs) => lhs
                .referenced_types()
                .iter()
                .chain(rhs.referenced_types().iter())
                .cloned()
                .collect(),
            Type::Meet(lhs, rhs) => lhs
                .referenced_types()
                .iter()
                .chain(rhs.referenced_types().iter())
                .cloned()
                .collect(),
            Type::Refinement(primary, refinement) => primary
                .referenced_types()
                .iter()
                .chain(refinement.referenced_types().iter())
                .cloned()
                .collect(),
            Type::List(inner) => inner.referenced_types(),
            Type::Nothing => Vec::default(),
            Type::MemberQualifier(_, inner) => inner.referenced_types(),
            Type::Parameter(_) => Vec::default(),
        }
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        match self {
            Type::Anything => {}
            Type::Ref(ref mut name, arguments) => {
                if !name.is_qualified() {
                    // it's a simple single-word name, needs qualifying, perhaps.
                    if let Some(Some(qualified)) = types.get(&name.name()) {
                        *name = qualified.clone();
                    }
                }
                for arg in arguments {
                    arg.qualify_types(types);
                }
            }
            Type::Const(_) => {}
            Type::Object(inner) => {
                inner.qualify_types(types);
            }
            Type::Expr(_) => {}
            Type::Join(lhs, rhs) => {
                lhs.qualify_types(types);
                rhs.qualify_types(types);
            }
            Type::Meet(lhs, rhs) => {
                lhs.qualify_types(types);
                rhs.qualify_types(types);
            }
            Type::Refinement(primary, refinement) => {
                primary.qualify_types(types);
                refinement.qualify_types(types);
            }
            Type::List(inner) => {
                inner.qualify_types(types);
            }
            Type::MemberQualifier(_, inner) => {
                inner.qualify_types(types);
            }
            Type::Nothing => {}
            Type::Parameter(_) => {}
        }
    }
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "Anything"),
            Type::Ref(r, args) => write!(f, "{:?}<{:?}>", r, args),
            Type::Const(value) => write!(f, "{:?}", value),
            Type::Join(l, r) => write!(f, "Join({:?}, {:?})", l, r),
            Type::Meet(l, r) => write!(f, "Meet({:?}, {:?})", l, r),
            Type::Nothing => write!(f, "Nothing"),
            Type::Object(obj) => write!(f, "{:?}", obj),
            Type::Refinement(fn_name, ty) => write!(f, "{:?}({:?})", fn_name, ty),
            Type::List(ty) => write!(f, "[{:?}]", ty),
            Type::Expr(expr) => write!(f, "#({:?})", expr),
            Type::MemberQualifier(qualifier, ty) => write!(f, "{:?}::{:?}", qualifier, ty),
            Type::Parameter(name) => write!(f, "{:?}", name),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectType {
    fields: Vec<Located<Field>>,
}

impl Default for ObjectType {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectType {
    pub fn new() -> Self {
        Self { fields: vec![] }
    }

    pub fn add_field(&mut self, field: Located<Field>) -> &Self {
        self.fields.push(field);
        self
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.fields
            .iter()
            .flat_map(|e| e.referenced_types())
            .collect()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        for field in &mut self.fields {
            field.qualify_types(types);
        }
    }

    pub fn fields(&self) -> &Vec<Located<Field>> {
        &self.fields
    }
}

#[derive(Clone, Debug)]
pub struct Field {
    name: Located<String>,
    ty: Located<Type>,
}

impl Field {
    pub fn new(name: Located<String>, ty: Located<Type>) -> Self {
        Self { name, ty }
    }

    pub fn name(&self) -> &Located<String> {
        &self.name
    }

    pub fn ty(&self) -> &Located<Type> {
        &self.ty
    }

    pub(crate) fn referenced_types(&self) -> Vec<Located<TypeName>> {
        self.ty.referenced_types()
    }

    pub(crate) fn qualify_types(&mut self, types: &HashMap<String, Option<Located<TypeName>>>) {
        self.ty.qualify_types(types)
    }
}

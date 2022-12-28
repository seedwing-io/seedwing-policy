use crate::core::Function;
use crate::lang::hir::MemberQualifier;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::TypeName;
use crate::runtime::{Bindings, TypeHandle};
use crate::value::Value;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

pub enum Type {
    Anything,
    Primordial(PrimordialType),
    Bound(Arc<TypeHandle>, Bindings),
    Argument(Located<String>),
    Const(Located<Value>),
    Object(ObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Arc<TypeHandle>, Arc<TypeHandle>),
    Meet(Arc<TypeHandle>, Arc<TypeHandle>),
    Refinement(Arc<TypeHandle>, Arc<TypeHandle>),
    List(Arc<TypeHandle>),
    MemberQualifier(Located<MemberQualifier>, Arc<TypeHandle>),
    Nothing,
}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Anything => write!(f, "anything"),
            Type::Primordial(inner) => write!(f, "{:?}", inner),
            Type::Const(inner) => write!(f, "{:?}", inner),
            Type::Object(inner) => write!(f, "{:?}", inner),
            Type::Expr(inner) => write!(f, "$({:?})", inner),
            Type::Join(lhs, rhs) => write!(f, "({:?} || {:?})", lhs, rhs),
            Type::Meet(lhs, rhs) => write!(f, "({:?} && {:?})", lhs, rhs),
            Type::Refinement(primary, refinement) => {
                write!(f, "{:?}({:?})", primary, refinement)
            }
            Type::List(inner) => write!(f, "[{:?}]", inner),
            Type::MemberQualifier(qualifier, ty) => write!(f, "{:?}::{:?}", qualifier, ty),
            Type::Argument(name) => write!(f, "{:?}", name),
            Type::Bound(primary, bindings) => write!(f, "{:?}<{:?}>", primary, bindings),
            Type::Nothing => write!(f, "nothing"),
        }
    }
}

#[derive(Debug)]
pub enum PrimordialType {
    Integer,
    Decimal,
    Boolean,
    String,
    Function(TypeName, Arc<dyn Function>),
}

#[derive(Debug)]
pub struct Field {
    name: Located<String>,
    ty: Arc<TypeHandle>,
}

impl Field {
    pub fn new(name: Located<String>, ty: Arc<TypeHandle>) -> Self {
        Self { name, ty }
    }

    pub fn name(&self) -> Located<String> {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<TypeHandle> {
        self.ty.clone()
    }
}

#[derive(Debug)]
pub struct ObjectType {
    fields: Vec<Arc<Located<Field>>>,
}

impl ObjectType {
    pub fn new(fields: Vec<Arc<Located<Field>>>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Arc<Located<Field>>> {
        &self.fields
    }

    pub async fn to_html(&self) -> String {
        let mut html = String::new();
        html.push_str("<div>{");
        for f in &self.fields {
            html.push_str("<div style='padding-left: 1em'>");
            html.push_str(
                format!(
                    "{}: {},",
                    f.name().inner(),
                    f.ty().ty().await.to_html().await
                )
                .as_str(),
            );
            html.push_str("</div>");
        }
        html.push_str("}</div>");

        html
    }
}

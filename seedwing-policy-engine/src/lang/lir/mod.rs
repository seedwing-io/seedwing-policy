use crate::core::Function;
use crate::lang::hir::MemberQualifier;
use crate::lang::mir::TypeHandle;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::{lir, mir, PrimordialType, TypeName};
use crate::runtime::{EvaluationResult, RuntimeError};
use crate::value::{InnerValue, InputValue, Object, RationaleResult};
use serde::Serialize;
use std::any::Any;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::future::{ready, Future};
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub(crate) static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize)]
pub struct Type {
    pub(crate) id: u64,
    name: Option<TypeName>,
    documentation: Option<String>,
    inner: InnerType,
}

impl Type {
    fn new(name: Option<TypeName>, documentation: Option<String>, inner: InnerType) -> Self {
        Self {
            id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            name,
            documentation,
            inner,
        }
    }

    pub fn name(&self) -> Option<TypeName> {
        self.name.clone()
    }

    pub fn documentation(&self) -> Option<String> {
        self.documentation.clone()
    }

    pub fn inner(&self) -> &InnerType {
        &self.inner
    }

    pub fn evaluate<'v>(
        self: &'v Arc<Self>,
        value: Rc<InputValue>,
        bindings: &'v Bindings,
    ) -> Pin<Box<dyn Future<Output = Result<RationaleResult, RuntimeError>> + 'v>> {
        match &self.inner {
            InnerType::Anything => Box::pin(async move {
                let mut locked_value = (*value).borrow();
                Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
            }),
            InnerType::Argument(name) => Box::pin(async move {
                if let Some(bound) = bindings.get(name) {
                    let result = bound.evaluate(value.clone(), bindings).await?;
                    let mut locked_value = (*value).borrow();
                    if result.is_some() {
                        Ok(locked_value
                            .rationale(self.clone(), RationaleResult::Same(value.clone())))
                    } else {
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                } else {
                    let mut locked_value = (*value).borrow();
                    Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                }
            }),
            InnerType::Primordial(inner) => match inner {
                PrimordialType::Integer => Box::pin(async move {
                    let mut locked_value = (*value).borrow();
                    if locked_value.is_integer() {
                        Ok(locked_value
                            .rationale(self.clone(), RationaleResult::Same(value.clone())))
                    } else {
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                }),
                PrimordialType::Decimal => Box::pin(async move {
                    let mut locked_value = (*value).borrow();
                    if locked_value.is_decimal() {
                        Ok(locked_value
                            .rationale(self.clone(), RationaleResult::Same(value.clone())))
                    } else {
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                }),
                PrimordialType::Boolean => Box::pin(async move {
                    let mut locked_value = (*value).borrow();

                    if locked_value.is_boolean() {
                        Ok(locked_value
                            .rationale(self.clone(), RationaleResult::Same(value.clone())))
                    } else {
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                }),
                PrimordialType::String => Box::pin(async move {
                    let mut locked_value = (*value).borrow();
                    if locked_value.is_string() {
                        Ok(locked_value
                            .rationale(self.clone(), RationaleResult::Same(value.clone())))
                    } else {
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                }),
                PrimordialType::Function(name, func) => Box::pin(async move {
                    let mut result = func.call(value.clone(), bindings).await;
                    if let Ok(result) = result {
                        let mut locked_value = (*value).borrow();
                        Ok(locked_value.rationale(self.clone(), result))
                    } else {
                        let mut locked_value = (*value).borrow();
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                }),
            },
            InnerType::Const(inner) => Box::pin(async move {
                let mut locked_value = (*value).borrow();
                //if (*inner).eq(&*locked_value.into()) {
                if inner.is_equal(locked_value) {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
                } else {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                }
            }),
            InnerType::Object(inner) => Box::pin(async move {
                let mut locked_value = (*value).borrow();
                if locked_value.is_object() {
                    let mut obj = locked_value.try_get_object();
                    let mut matches = vec![];
                    let mut mismatch = vec![];
                    if let Some(obj) = obj {
                        for field in inner.fields() {
                            if let Some(ref field_value) = obj.get(field.name()) {
                                let result =
                                    field.ty().evaluate(field_value.clone(), bindings).await?;
                                if result.is_none() {
                                    return Ok(
                                        locked_value.rationale(self.clone(), RationaleResult::None)
                                    );
                                }
                                matches.push((field, result));
                            } else {
                                mismatch.push(field);
                                break;
                            }
                        }
                        if !mismatch.is_empty() {
                            for e in mismatch {
                                locked_value.rationale(e.clone(), RationaleResult::None);
                            }
                            Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                        } else {
                            for (f, v) in matches {
                                locked_value.rationale(f.clone(), v);
                            }
                            Ok(locked_value
                                .rationale(self.clone(), RationaleResult::Same(value.clone())))
                        }
                    } else {
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                } else {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                }
            }),
            InnerType::Expr(expr) => Box::pin(async move {
                let result = expr.evaluate(value.clone()).await?;
                let mut locked_value = (*value).borrow();
                let locked_result = (*result).borrow();
                if let Some(true) = locked_result.try_get_boolean() {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
                } else {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                }
            }),
            InnerType::Join(lhs, rhs) => Box::pin(async move {
                let lhs_result = lhs.evaluate(value.clone(), bindings).await?;
                let rhs_result = rhs.evaluate(value.clone(), bindings).await?;

                let mut locked_value = (*value).borrow();

                if rhs_result.is_some() || lhs_result.is_some() {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
                } else {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                }
            }),
            InnerType::Meet(lhs, rhs) => Box::pin(async move {
                let lhs_result = lhs.evaluate(value.clone(), bindings).await?;
                let rhs_result = rhs.evaluate(value.clone(), bindings).await?;

                let mut locked_value = (*value).borrow();

                if rhs_result.is_some() && lhs_result.is_some() {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
                } else {
                    Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                }
            }),
            InnerType::Refinement(primary, refinement) => Box::pin(async move {
                let mut result = primary.evaluate(value.clone(), bindings).await?;
                match &result {
                    RationaleResult::None => {
                        let mut locked_value = (*value).borrow();
                        Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                    }
                    RationaleResult::Same(primary_value) => {
                        let refinement_result =
                            refinement.evaluate(primary_value.clone(), bindings).await?;
                        let mut locked_value = (*value).borrow();
                        Ok(locked_value.rationale(self.clone(), refinement_result))
                    }
                    RationaleResult::Transform(primary_value) => {
                        let refinement_result =
                            refinement.evaluate(primary_value.clone(), bindings).await?;
                        if refinement_result.is_none() {
                            let mut locked_value = (*value).borrow();
                            Ok(locked_value.rationale(self.clone(), RationaleResult::None))
                        } else {
                            let mut locked_value = (*value).borrow();
                            Ok(locked_value.rationale(
                                self.clone(),
                                RationaleResult::Transform(primary_value.clone()),
                            ))
                        }
                    }
                }
            }),
            InnerType::List(_) => todo!(),
            InnerType::Bound(primary, bindings) => {
                Box::pin(async move { primary.evaluate(value, bindings).await })
            }
            InnerType::Nothing => Box::pin(ready(Ok(RationaleResult::None))),
        }
    }
}

#[derive(Serialize)]
pub enum InnerType {
    Anything,
    Primordial(PrimordialType),
    Bound(Arc<Type>, Bindings),
    Argument(String),
    Const(ValueType),
    Object(ObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Arc<Type>, Arc<Type>),
    Meet(Arc<Type>, Arc<Type>),
    Refinement(Arc<Type>, Arc<Type>),
    List(Arc<Type>),
    Nothing,
}

#[derive(Serialize, Default, Debug)]
pub struct Bindings {
    bindings: HashMap<String, Arc<Type>>,
}

impl Bindings {
    pub fn new() -> Self {
        Self {
            bindings: Default::default(),
        }
    }

    pub fn bind(&mut self, name: String, ty: Arc<Type>) {
        self.bindings.insert(name, ty);
    }

    pub fn get<S: Into<String>>(&self, name: S) -> Option<Arc<Type>> {
        self.bindings.get(&name.into()).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<Type>)> {
        self.bindings.iter()
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl Debug for InnerType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InnerType::Anything => write!(f, "anything"),
            InnerType::Primordial(inner) => write!(f, "{:?}", inner),
            InnerType::Const(inner) => write!(f, "{:?}", inner),
            InnerType::Object(inner) => write!(f, "{:?}", inner),
            InnerType::Expr(inner) => write!(f, "$({:?})", inner),
            InnerType::Join(lhs, rhs) => write!(f, "({:?} || {:?})", lhs, rhs),
            InnerType::Meet(lhs, rhs) => write!(f, "({:?} && {:?})", lhs, rhs),
            InnerType::Refinement(primary, refinement) => {
                write!(f, "{:?}({:?})", primary, refinement)
            }
            InnerType::List(inner) => write!(f, "[{:?}]", inner),
            InnerType::Argument(name) => write!(f, "{:?}", name),
            InnerType::Bound(primary, bindings) => write!(f, "{:?}<{:?}>", primary, bindings),
            InnerType::Nothing => write!(f, "nothing"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Field {
    pub(crate) id: u64,
    name: String,
    ty: Arc<Type>,
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Field {
    pub fn new(name: String, ty: Arc<Type>) -> Self {
        Self {
            id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            name,
            ty,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<Type> {
        self.ty.clone()
    }
}

#[derive(Debug, Serialize, Clone)]
pub enum ValueType {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    Object(ObjectType),
    List(Vec<Arc<ValueType>>),
    Octets(Vec<u8>),
}

impl From<&ValueType> for InputValue {
    fn from(ty: &ValueType) -> Self {
        match ty {
            ValueType::Null => InputValue::null(),
            ValueType::String(val) => val.clone().into(),
            ValueType::Integer(val) => (*val).into(),
            ValueType::Decimal(val) => (*val).into(),
            ValueType::Boolean(val) => (*val).into(),
            ValueType::Object(val) => val.into(),
            ValueType::List(val) => InputValue::new(InnerValue::List(
                val.iter()
                    .map(|e| {
                        let copy = &*e.clone();
                        Rc::new(InputValue::from(copy))
                    })
                    .collect(),
            )),
            ValueType::Octets(val) => InputValue::new(InnerValue::Octets(val.clone())),
        }
    }
}

impl ValueType {
    pub fn is_equal(&self, other: &InputValue) -> bool {
        match (self, &other.inner) {
            (ValueType::Null, InnerValue::Null) => true,
            (ValueType::String(lhs), InnerValue::String(rhs)) => lhs.eq(rhs),
            (ValueType::Integer(lhs), InnerValue::Integer(rhs)) => lhs.eq(rhs),
            (ValueType::Decimal(lhs), InnerValue::Decimal(rhs)) => lhs.eq(rhs),
            (ValueType::Boolean(lhs), InnerValue::Boolean(rhs)) => lhs.eq(rhs),
            (ValueType::Object(lhs), InnerValue::Object(rhs)) => todo!(),
            (ValueType::List(lhs), InnerValue::List(rhs)) => todo!(),
            (ValueType::Octets(lhs), InnerValue::Octets(rhs)) => todo!(),
            _ => false,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ObjectType {
    fields: Vec<Arc<Field>>,
}

impl From<&ObjectType> for InputValue {
    fn from(ty: &ObjectType) -> Self {
        todo!()
    }
}

impl ObjectType {
    pub fn new(fields: Vec<Arc<Field>>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Arc<Field>> {
        &self.fields
    }
}

#[derive(Clone, Debug)]
pub struct World {
    types: HashMap<TypeName, Arc<Type>>,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            types: Default::default(),
        }
    }

    pub(crate) async fn add(&mut self, path: TypeName, handle: Arc<TypeHandle>) {
        let ty = handle.ty().await;
        let name = handle.name();
        let converted = convert(name, handle.documentation(), &ty).await;
        self.types.insert(path, converted);
    }

    pub async fn evaluate<P: Into<String>, V: Into<InputValue>>(
        &self,
        path: P,
        value: V,
    ) -> Result<RationaleResult, RuntimeError> {
        let value = Rc::new(value.into());
        let path = TypeName::from(path.into());
        let ty = self.types.get(&path);
        if let Some(ty) = ty {
            let bindings = Bindings::default();
            match ty.evaluate(value.clone(), &bindings).await? {
                RationaleResult::None => Ok(RationaleResult::None),
                RationaleResult::Same(_) | RationaleResult::Transform(_) => {
                    Ok(RationaleResult::Same(value))
                }
            }
        } else {
            Err(RuntimeError::NoSuchType(path))
        }
    }

    pub fn get<S: Into<String>>(&self, name: S) -> Option<Component> {
        let name = name.into();
        let path = TypeName::from(name);

        if let Some(ty) = self.types.get(&path) {
            return Some(Component::Type(ty.clone()));
        }

        let mut module_handle = ModuleHandle::new();
        let path = path.as_type_str();
        for (name, ty) in self.types.iter() {
            let name = name.as_type_str();
            if let Some(relative_name) = name.strip_prefix(&path) {
                let relative_name = relative_name.strip_prefix("::").unwrap_or(relative_name);
                let parts: Vec<&str> = relative_name.split("::").collect();
                if parts.len() == 1 {
                    module_handle.types.push(parts[0].into());
                } else if !module_handle.modules.contains(&parts[0].into()) {
                    module_handle.modules.push(parts[0].into())
                }
            }
        }

        if module_handle.is_empty() {
            None
        } else {
            Some(Component::Module(module_handle.sort()))
        }
    }
}

#[derive(Debug)]
pub enum Component {
    Module(ModuleHandle),
    Type(Arc<Type>),
}

#[derive(Serialize, Debug)]
pub struct ModuleHandle {
    modules: Vec<String>,
    types: Vec<String>,
}

impl ModuleHandle {
    fn new() -> Self {
        Self {
            modules: vec![],
            types: vec![],
        }
    }

    fn sort(mut self) -> Self {
        self.modules.sort();
        self.types.sort();
        self
    }

    fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.types.is_empty()
    }
}

fn convert(
    name: Option<TypeName>,
    documentation: Option<String>,
    ty: &Arc<Located<mir::Type>>,
) -> Pin<Box<dyn Future<Output = Arc<Type>> + '_>> {
    match &***ty {
        mir::Type::Anything => Box::pin(async move {
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Anything,
            ))
        }),
        mir::Type::Primordial(primordial) => Box::pin(async move {
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Primordial(primordial.clone()),
            ))
        }),
        mir::Type::Bound(primary, mir_bindings) => Box::pin(async move {
            let primary =
                convert(primary.name(), primary.documentation(), &primary.ty().await).await;
            let mut bindings = Bindings::new();
            for (key, value) in mir_bindings.iter() {
                bindings.bind(
                    key.clone(),
                    convert(value.name(), value.documentation(), &value.ty().await).await,
                )
            }
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Bound(primary, bindings),
            ))
        }),
        mir::Type::Argument(name) => Box::pin(async move {
            Arc::new(lir::Type::new(
                None,
                None,
                lir::InnerType::Argument(name.inner()),
            ))
        }),
        mir::Type::Const(value) => Box::pin(async move {
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Const(value.inner()),
            ))
        }),
        mir::Type::Object(mir_object) => Box::pin(async move {
            let mut fields: Vec<Arc<Field>> = Default::default();
            for f in mir_object.fields().iter() {
                let ty = f.ty();
                let field = Arc::new(lir::Field::new(
                    f.name().inner(),
                    convert(ty.name(), ty.documentation(), &ty.ty().await).await,
                ));
                fields.push(field);
            }
            let object = ObjectType::new(fields);
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Object(object),
            ))
        }),
        mir::Type::Expr(expr) => Box::pin(async move {
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Expr(expr.clone()),
            ))
        }),
        mir::Type::Join(lhs, rhs) => Box::pin(async move {
            let lhs = convert(lhs.name(), lhs.documentation(), &lhs.ty().await).await;
            let rhs = convert(rhs.name(), rhs.documentation(), &rhs.ty().await).await;
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Join(lhs, rhs),
            ))
        }),
        mir::Type::Meet(lhs, rhs) => Box::pin(async move {
            let lhs = convert(lhs.name(), lhs.documentation(), &lhs.ty().await).await;
            let rhs = convert(rhs.name(), rhs.documentation(), &rhs.ty().await).await;
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Meet(lhs, rhs),
            ))
        }),
        mir::Type::Refinement(primary, refinement) => Box::pin(async move {
            let primary =
                convert(primary.name(), primary.documentation(), &primary.ty().await).await;
            let refinement = convert(
                refinement.name(),
                refinement.documentation(),
                &refinement.ty().await,
            )
            .await;
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::Refinement(primary, refinement),
            ))
        }),
        mir::Type::List(inner) => Box::pin(async move {
            let inner = convert(inner.name(), inner.documentation(), &inner.ty().await).await;
            Arc::new(lir::Type::new(
                name,
                documentation,
                lir::InnerType::List(inner),
            ))
        }),
        mir::Type::Nothing => Box::pin(async move {
            Arc::new(lir::Type::new(name, documentation, lir::InnerType::Nothing))
        }),
    }
}

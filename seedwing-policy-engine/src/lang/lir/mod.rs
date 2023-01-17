use crate::core::Function;
use crate::lang::hir::MemberQualifier;
use crate::lang::mir::TypeHandle;
use crate::lang::parser::expr::Expr;
use crate::lang::parser::Located;
use crate::lang::{lir, mir, PrimordialType};
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvaluationResult, ModuleHandle, Output, RuntimeError};
use crate::runtime::{TypeName, World};
use crate::value::{Object, RationaleResult, RuntimeValue};
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

#[derive(Debug, Serialize)]
pub struct Type {
    name: Option<TypeName>,
    documentation: Option<String>,
    parameters: Vec<String>,
    inner: InnerType,
}

impl Type {
    pub(crate) fn new(
        name: Option<TypeName>,
        documentation: Option<String>,
        parameters: Vec<String>,
        inner: InnerType,
    ) -> Self {
        Self {
            name,
            documentation,
            parameters,
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

    pub fn parameters(&self) -> Vec<String> {
        self.parameters.clone()
    }

    pub fn evaluate<'v>(
        self: &'v Arc<Self>,
        value: Rc<RuntimeValue>,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output=Result<EvaluationResult, RuntimeError>> + 'v>> {
        match &self.inner {
            InnerType::Anything => Box::pin(async move {
                let mut locked_value = (*value).borrow();
                //Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::Anything,
                    Output::Identity,
                ))
            }),
            InnerType::Ref(slot, bindings) => Box::pin(async move {
                if let Some(ty) = world.get_by_slot(*slot) {
                    let bindings = (ty.parameters(), bindings, world).into();
                    ty.evaluate(value, &bindings, world).await
                } else {
                    Err(RuntimeError::NoSuchTypeSlot(*slot))
                }
            }),
            InnerType::Bound(ty, bindings) => {
                Box::pin(async move { ty.evaluate(value, bindings, world).await })
            }
            InnerType::Argument(name) => Box::pin(async move {
                if let Some(bound) = bindings.get(name) {
                    bound.evaluate(value.clone(), bindings, world).await
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::InvalidArgument(name.clone()),
                        Output::None,
                    ))
                }
            }),
            InnerType::Primordial(inner) => match inner {
                PrimordialType::Integer => Box::pin(async move {
                    let mut locked_value = (*value).borrow();
                    if locked_value.is_integer() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                }),
                PrimordialType::Decimal => Box::pin(async move {
                    let mut locked_value = (*value).borrow();
                    if locked_value.is_decimal() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                }),
                PrimordialType::Boolean => Box::pin(async move {
                    let mut locked_value = (*value).borrow();

                    if locked_value.is_boolean() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                }),
                PrimordialType::String => Box::pin(async move {
                    let mut locked_value = (*value).borrow();
                    if locked_value.is_string() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                }),
                PrimordialType::Function(name, func) => Box::pin(async move {
                    let mut result = func.call(value.clone(), bindings, world).await?;
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Function(result.output().is_some(), result.supporting()),
                        result.output(),
                    ))
                }),
            },
            InnerType::Const(inner) => Box::pin(async move {
                let mut locked_value = (*value).borrow();
                if inner.is_equal(locked_value).await {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Const(true),
                        Output::Identity,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Const(false),
                        Output::Identity,
                    ))
                }
            }),
            InnerType::Object(inner) => Box::pin(async move {
                let mut locked_value = (*value).borrow();
                if let Some(obj) = locked_value.try_get_object() {
                    let mut result = HashMap::new();
                    for field in &inner.fields {
                        if let Some(ref field_value) = obj.get(field.name()) {
                            result.insert(
                                field.name(),
                                field
                                    .ty()
                                    .evaluate(field_value.clone(), bindings, world)
                                    .await?,
                            );
                        } else if !field.optional() {
                            result.insert(
                                field.name(),
                                EvaluationResult::new(
                                    None,
                                    field.ty(),
                                    Rationale::MissingField(field.name()),
                                    Output::None,
                                ),
                            );
                        }
                    }
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Object(result),
                        Output::Identity,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::NotAnObject,
                        Output::None,
                    ))
                }
            }),
            InnerType::Expr(expr) => Box::pin(async move {
                let result = expr.evaluate(value.clone()).await?;
                let mut locked_value = (*value).borrow();
                let locked_result = (*result).borrow();
                if let Some(true) = locked_result.try_get_boolean() {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Expression(true),
                        Output::Identity,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Expression(false),
                        Output::None,
                    ))
                }
            }),
            InnerType::Join(terms) => Box::pin(async move {
                let mut result = Vec::new();
                for e in terms {
                    result.push(e.evaluate(value.clone(), bindings, world).await?);
                }

                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::Join(result),
                    Output::Identity,
                ))
            }),
            InnerType::Meet(terms) => Box::pin(async move {
                let mut result = Vec::new();
                for e in terms {
                    result.push(e.evaluate(value.clone(), bindings, world).await?);
                }

                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::Meet(result),
                    Output::Identity,
                ))
            }),
            InnerType::Refinement(primary, refinement) => Box::pin(async move {
                let mut result = primary.evaluate(value.clone(), bindings, world).await?;

                if !result.satisfied() {
                    return Ok(result);
                }

                if let Some(output) = result.output() {
                    let refinement_result = refinement.evaluate(output, bindings, world).await?;
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Refinement(Box::new(result), Some(Box::new(refinement_result))),
                        Output::None,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Refinement(Box::new(result), None),
                        Output::None,
                    ))
                }
            }),
            InnerType::List(terms) => Box::pin(async move {
                if let Some(list_value) = value.try_get_list() {
                    if list_value.len() == terms.len() {
                        let mut result = Vec::new();
                        for (term, element) in terms.iter().zip(list_value.iter()) {
                            result.push(term.evaluate(element.clone(), bindings, world).await?);
                        }
                        return Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::List(result),
                            Output::Identity,
                        ));
                    }
                }
                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::NotAList,
                    Output::None,
                ))
            }),
            //InnerType::Bound(primary, bindings) => {
            //Box::pin(async move { primary.evaluate(value, bindings).await })
            //}
            InnerType::Nothing => Box::pin(async move {
                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::Nothing,
                    Output::None,
                ))
            }),
        }
    }
}

#[derive(Serialize)]
pub enum InnerType {
    Anything,
    Primordial(PrimordialType),
    Bound(Arc<Type>, Bindings),
    Ref(usize, Vec<Arc<Type>>),
    Argument(String),
    Const(ValueType),
    Object(ObjectType),
    Expr(Arc<Located<Expr>>),
    Join(Vec<Arc<Type>>),
    Meet(Vec<Arc<Type>>),
    Refinement(Arc<Type>, Arc<Type>),
    List(Vec<Arc<Type>>),
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

    pub fn iter(&self) -> impl Iterator<Item=(&String, &Arc<Type>)> {
        self.bindings.iter()
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl From<(Vec<String>, &Vec<Arc<Type>>, &World)> for Bindings {
    fn from((parameters, arguments, world): (Vec<String>, &Vec<Arc<Type>>, &World)) -> Self {
        let mut bindings = Self::new();
        for (param, arg) in parameters.iter().zip(arguments.iter()) {
            if let InnerType::Ref(slot, unresolved_bindings) = &arg.inner {
                if let Some(resolved_type) = world.get_by_slot(*slot) {
                    if resolved_type.parameters().is_empty() {
                        bindings.bind(param.clone(), resolved_type.clone())
                    } else {
                        let resolved_bindings =
                            (resolved_type.parameters(), unresolved_bindings, world).into();
                        bindings.bind(
                            param.clone(),
                            Arc::new(Type::new(
                                resolved_type.name(),
                                resolved_type.documentation(),
                                resolved_type.parameters(),
                                InnerType::Bound(resolved_type, resolved_bindings),
                            )),
                        )
                    }
                }
            } else {
                bindings.bind(param.clone(), arg.clone())
            }
        }
        bindings
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
            InnerType::Join(terms) => write!(f, "||({:?})", terms),
            InnerType::Meet(terms) => write!(f, "&&({:?})", terms),
            InnerType::Refinement(primary, refinement) => {
                write!(f, "{:?}({:?})", primary, refinement)
            }
            InnerType::List(inner) => write!(f, "[{:?}]", inner),
            InnerType::Argument(name) => write!(f, "{:?}", name),
            InnerType::Ref(slot, bindings) => write!(f, "ref {:?}<{:?}>", slot, bindings),
            InnerType::Bound(primary, bindings) => write!(f, "bound {:?}<{:?}>", primary, bindings),
            InnerType::Nothing => write!(f, "nothing"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Field {
    name: String,
    ty: Arc<Type>,
    optional: bool,
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Field {
    pub fn new(name: String, ty: Arc<Type>, optional: bool) -> Self {
        Self {
            name,
            ty,
            optional,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<Type> {
        self.ty.clone()
    }

    pub fn optional(&self) -> bool {
        self.optional
    }
}

#[derive(Debug, Serialize, Clone)]
pub enum ValueType {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    List(Vec<Arc<ValueType>>),
    Octets(Vec<u8>),
}

impl From<&ValueType> for RuntimeValue {
    fn from(ty: &ValueType) -> Self {
        match ty {
            ValueType::Null => RuntimeValue::null(),
            ValueType::String(val) => val.clone().into(),
            ValueType::Integer(val) => (*val).into(),
            ValueType::Decimal(val) => (*val).into(),
            ValueType::Boolean(val) => (*val).into(),
            ValueType::List(val) => Self::List(
                val.iter()
                    .map(|e| {
                        let copy = &*e.clone();
                        Rc::new(RuntimeValue::from(copy))
                    })
                    .collect(),
            ),
            ValueType::Octets(val) => Self::Octets(val.clone()),
        }
    }
}

impl ValueType {
    pub fn is_equal<'e>(
        &'e self,
        other: &'e RuntimeValue,
    ) -> Pin<Box<dyn Future<Output=bool> + 'e>> {
        match (self, &other) {
            (ValueType::Null, RuntimeValue::Null) => Box::pin(ready(true)),
            (ValueType::String(lhs), RuntimeValue::String(rhs)) => {
                Box::pin(async move { lhs.eq(rhs) })
            }
            (ValueType::Integer(lhs), RuntimeValue::Integer(rhs)) => {
                Box::pin(async move { lhs.eq(rhs) })
            }
            (ValueType::Decimal(lhs), RuntimeValue::Decimal(rhs)) => {
                Box::pin(async move { lhs.eq(rhs) })
            }
            (ValueType::Boolean(lhs), RuntimeValue::Boolean(rhs)) => {
                Box::pin(async move { lhs.eq(rhs) })
            }
            (ValueType::List(lhs), RuntimeValue::List(rhs)) => Box::pin(async move {
                if lhs.len() != rhs.len() {
                    false
                } else {
                    for (l, r) in lhs.iter().zip(rhs.iter()) {
                        if !l.is_equal(r).await {
                            return false;
                        }
                    }
                    true
                }
            }),
            (ValueType::Octets(lhs), RuntimeValue::Octets(rhs)) => {
                Box::pin(async move { lhs.eq(rhs) })
            }
            _ => Box::pin(ready(false)),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ObjectType {
    fields: Vec<Arc<Field>>,
}

impl ObjectType {
    pub fn new(fields: Vec<Arc<Field>>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Arc<Field>> {
        &self.fields
    }
}

pub(crate) fn convert(
    name: Option<TypeName>,
    documentation: Option<String>,
    parameters: Vec<String>,
    ty: &Arc<Located<mir::Type>>,
) -> Arc<Type> {
    match &***ty {
        mir::Type::Anything => Arc::new(lir::Type::new(
            name,
            documentation,
            parameters,
            lir::InnerType::Anything,
        )),
        mir::Type::Primordial(primordial) => Arc::new(lir::Type::new(
            name,
            documentation,
            parameters,
            lir::InnerType::Primordial(primordial.clone()),
        )),
        /*
        mir::Type::Bound(primary, mir_bindings) => {
            let primary = convert(
                primary.name(),
                primary.documentation(),
                primary.parameters().iter().map(|e| e.inner()).collect(),
                &primary.ty(),
            );
            let mut bindings = Bindings::new();
            for (key, value) in mir_bindings.iter() {
                bindings.bind(
                    key.clone(),
                    convert(
                        value.name(),
                        value.documentation(),
                        value.parameters().iter().map(|e| e.inner()).collect(),
                        &value.ty(),
                    ),
                )
            }
            Arc::new(lir::Type::new(
                name,
                documentation,
                parameters,
                lir::InnerType::Bound(primary, bindings),
            ))
        }
         */
        mir::Type::Ref(slot, bindings) => {
            let mut lir_bindings = Vec::default();
            for e in bindings {
                lir_bindings.push(convert(
                    e.name(),
                    e.documentation(),
                    e.parameters().iter().map(|e| e.inner()).collect(),
                    &e.ty(),
                ));
            }

            Arc::new(lir::Type::new(
                None,
                None,
                parameters,
                lir::InnerType::Ref(*slot, lir_bindings),
            ))
        }
        mir::Type::Argument(name) => Arc::new(lir::Type::new(
            None,
            None,
            parameters,
            lir::InnerType::Argument(name.clone()),
        )),
        mir::Type::Const(value) => Arc::new(lir::Type::new(
            name,
            documentation,
            parameters,
            lir::InnerType::Const(value.clone()),
        )),
        mir::Type::Object(mir_object) => {
            let mut fields: Vec<Arc<Field>> = Default::default();
            for f in mir_object.fields().iter() {
                let ty = f.ty();
                let field = Arc::new(lir::Field::new(
                    f.name().inner(),
                    convert(
                        ty.name(),
                        ty.documentation(),
                        ty.parameters().iter().map(|e| e.inner()).collect(),
                        &ty.ty(),
                    ),
                    f.optional(),
                ));
                fields.push(field);
            }
            let object = ObjectType::new(fields);
            Arc::new(lir::Type::new(
                name,
                documentation,
                parameters,
                lir::InnerType::Object(object),
            ))
        }
        mir::Type::Expr(expr) => Arc::new(lir::Type::new(
            name,
            documentation,
            parameters,
            lir::InnerType::Expr(expr.clone()),
        )),
        mir::Type::Join(terms) => {
            let mut inner = Vec::new();
            for e in terms {
                inner.push(convert(
                    e.name(),
                    e.documentation(),
                    e.parameters().iter().map(|e| e.inner()).collect(),
                    &e.ty(),
                ));
            }
            Arc::new(lir::Type::new(
                name,
                documentation,
                parameters,
                lir::InnerType::Join(inner),
            ))
        }
        mir::Type::Meet(terms) => {
            let mut inner = Vec::new();
            for e in terms {
                inner.push(convert(
                    e.name(),
                    e.documentation(),
                    e.parameters().iter().map(|e| e.inner()).collect(),
                    &e.ty(),
                ));
            }
            Arc::new(lir::Type::new(
                name,
                documentation,
                parameters,
                lir::InnerType::Meet(inner),
            ))
        }
        mir::Type::Refinement(primary, refinement) => {
            let primary = convert(
                primary.name(),
                primary.documentation(),
                primary.parameters().iter().map(|e| e.inner()).collect(),
                &primary.ty(),
            );
            let refinement = convert(
                refinement.name(),
                refinement.documentation(),
                refinement.parameters().iter().map(|e| e.inner()).collect(),
                &refinement.ty(),
            );
            Arc::new(lir::Type::new(
                name,
                documentation,
                parameters,
                lir::InnerType::Refinement(primary, refinement),
            ))
        }
        mir::Type::List(terms) => {
            let mut inner = Vec::new();
            for e in terms {
                inner.push(convert(
                    e.name(),
                    e.documentation(),
                    e.parameters().iter().map(|e| e.inner()).collect(),
                    &e.ty(),
                ))
            }

            Arc::new(lir::Type::new(
                name,
                documentation,
                parameters,
                lir::InnerType::List(inner),
            ))
        }
        mir::Type::Nothing => Arc::new(lir::Type::new(
            name,
            documentation,
            Vec::default(),
            lir::InnerType::Nothing,
        )),
    }
}

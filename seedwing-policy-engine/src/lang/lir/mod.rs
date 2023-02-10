use crate::core::Function;

use crate::lang::parser::Located;
use crate::lang::{lir, mir, PrimordialType, SyntacticSugar};
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvaluationResult, Output, RuntimeError, TraceResult};
use crate::runtime::{TypeName, World};
use crate::value::RuntimeValue;
use serde::Serialize;

use std::borrow::Borrow;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::future::{ready, Future};
use std::hash::Hasher;
use std::pin::Pin;

use std::sync::Arc;
use std::time::Instant;

#[derive(Serialize, Debug, Clone)]
pub enum Expr {
    SelfLiteral(),
    Value(ValueType),
    Function(String, Arc<Expr>),
    Add(Arc<Expr>, Arc<Expr>),
    Subtract(Arc<Expr>, Arc<Expr>),
    Multiply(Arc<Expr>, Arc<Expr>),
    Divide(Arc<Expr>, Arc<Expr>),
    LessThan(Arc<Expr>, Arc<Expr>),
    LessThanEqual(Arc<Expr>, Arc<Expr>),
    GreaterThan(Arc<Expr>, Arc<Expr>),
    GreaterThanEqual(Arc<Expr>, Arc<Expr>),
    Equal(Arc<Expr>, Arc<Expr>),
    NotEqual(Arc<Expr>, Arc<Expr>),
    Not(Arc<Expr>),
    LogicalAnd(Arc<Expr>, Arc<Expr>),
    LogicalOr(Arc<Expr>, Arc<Expr>),
}

pub type ExprFuture =
    Pin<Box<dyn Future<Output = Result<Arc<RuntimeValue>, RuntimeError>> + 'static>>;

impl Expr {
    #[allow(clippy::let_and_return)]
    pub fn evaluate(&self, value: Arc<RuntimeValue>) -> ExprFuture {
        let this = self.clone();

        Box::pin(async move {
            match &this {
                Expr::SelfLiteral() => Ok(value.clone()),
                Expr::Value(ref inner) => Ok(Arc::new(inner.into())),
                Expr::Function(_, _) => todo!(),
                Expr::Add(_, _) => todo!(),
                Expr::Subtract(_, _) => todo!(),
                Expr::Multiply(_, _) => todo!(),
                Expr::Divide(_, _) => todo!(),
                Expr::LessThan(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Less) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Arc::new(true.into()))
                    } else {
                        Ok(Arc::new(false.into()))
                    };

                    result
                }
                Expr::LessThanEqual(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Less | Ordering::Equal) =
                        (*lhs).partial_cmp(&(*rhs))
                    {
                        Ok(Arc::new(true.into()))
                    } else {
                        Ok(Arc::new(false.into()))
                    };

                    result
                }
                Expr::GreaterThan(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Greater) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Arc::new(true.into()))
                    } else {
                        Ok(Arc::new(false.into()))
                    };

                    result
                }
                Expr::GreaterThanEqual(lhs, rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Greater | Ordering::Equal) =
                        (*lhs).partial_cmp(&(*rhs))
                    {
                        Ok(Arc::new(true.into()))
                    } else {
                        Ok(Arc::new(false.into()))
                    };

                    result
                }
                Expr::Equal(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Equal) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Arc::new(true.into()))
                    } else {
                        Ok(Arc::new(false.into()))
                    };

                    result
                }
                Expr::NotEqual(ref lhs, ref rhs) => {
                    let lhs = lhs.clone().evaluate(value.clone()).await?;
                    let rhs = rhs.clone().evaluate(value.clone()).await?;

                    let result = if let Some(Ordering::Equal) = (*lhs).partial_cmp(&(*rhs)) {
                        Ok(Arc::new(false.into()))
                    } else {
                        Ok(Arc::new(true.into()))
                    };

                    result
                }
                Expr::Not(_) => todo!(),
                Expr::LogicalAnd(_, _) => todo!(),
                Expr::LogicalOr(_, _) => todo!(),
            }
        })
    }
}

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

    pub fn order(&self, world: &World) -> u8 {
        self.inner.order(world)
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

    /// Attempt to retrieve a const-ish value from this type.
    pub fn try_get_resolved_value(&self) -> Option<ValueType> {
        if let InnerType::Const(val) = &self.inner {
            Some(val.clone())
        } else {
            None
        }
    }

    pub fn evaluate<'v>(
        self: &'v Arc<Self>,
        value: Arc<RuntimeValue>,
        ctx: &'v mut EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>> {
        let trace = ctx.trace();
        match &self.inner {
            InnerType::Anything => Box::pin(async move {
                let _locked_value = (*value).borrow();
                //Ok(locked_value.rationale(self.clone(), RationaleResult::Same(value.clone())))
                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::Anything,
                    Output::Identity,
                    trace.done(),
                ))
            }),
            InnerType::Ref(sugar, slot, arguments) => Box::pin(async move {
                #[allow(clippy::ptr_arg)]
                fn build_bindings<'b>(
                    value: Arc<RuntimeValue>,
                    mut bindings: Bindings,
                    ctx: &'b mut EvalContext,
                    parameters: Vec<String>,
                    arguments: &'b Vec<Arc<Type>>,
                    world: &'b World,
                ) -> Pin<Box<dyn Future<Output = Result<Bindings, RuntimeError>> + 'b>>
                {
                    Box::pin(async move {
                        for (param, arg) in parameters.iter().zip(arguments.iter()) {
                            if let InnerType::Ref(_sugar, slot, unresolved_bindings) = &arg.inner {
                                if let Some(resolved_type) = world.get_by_slot(*slot) {
                                    if resolved_type.parameters().is_empty() {
                                        bindings.bind(param.clone(), resolved_type.clone())
                                    } else {
                                        let resolved_bindings = build_bindings(
                                            value.clone(),
                                            bindings.clone(),
                                            ctx,
                                            resolved_type.parameters(),
                                            unresolved_bindings,
                                            world,
                                        )
                                        .await?;
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
                            } else if let InnerType::Argument(name) = &arg.inner {
                                bindings.bind(param.clone(), bindings.get(name).unwrap());
                            } else if let InnerType::Deref(_inner) = &arg.inner {
                                let result = arg
                                    .evaluate(value.clone(), ctx, &Bindings::default(), world)
                                    .await?;

                                if result.satisfied() {
                                    if let Some(output) = result.output() {
                                        bindings.bind(param.clone(), Arc::new(output.into()))
                                    } else {
                                        bindings.bind(
                                            param.clone(),
                                            Arc::new(Type::new(
                                                None,
                                                None,
                                                Vec::default(),
                                                InnerType::Nothing,
                                            )),
                                        )
                                    }
                                } else {
                                    bindings.bind(
                                        param.clone(),
                                        Arc::new(Type::new(
                                            None,
                                            None,
                                            Vec::default(),
                                            InnerType::Nothing,
                                        )),
                                    )
                                }
                            } else {
                                bindings.bind(param.clone(), arg.clone())
                            }
                        }

                        Ok(bindings)
                    })
                };

                if let Some(ty) = world.get_by_slot(*slot) {
                    let bindings = build_bindings(
                        value.clone(),
                        bindings.clone(),
                        ctx,
                        ty.parameters(),
                        arguments,
                        world,
                    )
                    .await?;

                    let result = ty.evaluate(value.clone(), ctx, &bindings, world).await?;
                    if let SyntacticSugar::Chain = sugar {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            result.rationale().clone(),
                            result.raw_output().clone(),
                            trace.done(),
                        ))
                    } else {
                        Ok(result)
                    }
                } else {
                    Err(RuntimeError::NoSuchTypeSlot(*slot))
                }
            }),
            InnerType::Deref(inner) => {
                Box::pin(async move { inner.evaluate(value.clone(), ctx, bindings, world).await })
            }
            InnerType::Bound(ty, bindings) => {
                Box::pin(async move { ty.evaluate(value, ctx, bindings, world).await })
            }
            InnerType::Argument(name) => Box::pin(async move {
                if let Some(bound) = bindings.get(name) {
                    bound.evaluate(value.clone(), ctx, bindings, world).await
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::InvalidArgument(name.clone()),
                        Output::None,
                        trace.done(),
                    ))
                }
            }),
            InnerType::Primordial(inner) => match inner {
                PrimordialType::Integer => Box::pin(async move {
                    let locked_value = (*value).borrow();
                    if locked_value.is_integer() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                            trace.done(),
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                            trace.done(),
                        ))
                    }
                }),
                PrimordialType::Decimal => Box::pin(async move {
                    let locked_value = (*value).borrow();
                    if locked_value.is_decimal() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                            trace.done(),
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                            trace.done(),
                        ))
                    }
                }),
                PrimordialType::Boolean => Box::pin(async move {
                    let locked_value = (*value).borrow();

                    if locked_value.is_boolean() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                            trace.done(),
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                            trace.done(),
                        ))
                    }
                }),
                PrimordialType::String => Box::pin(async move {
                    let locked_value = (*value).borrow();
                    if locked_value.is_string() {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                            trace.done(),
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                            trace.done(),
                        ))
                    }
                }),
                PrimordialType::Function(_sugar, _name, func) => Box::pin(async move {
                    let result = func.call(value.clone(), ctx, bindings, world).await?;
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Function(
                            result.output().is_some(),
                            result.rationale().map(Box::new),
                            result.supporting(),
                        ),
                        result.output(),
                        trace.done(),
                    ))
                }),
            },
            InnerType::Const(inner) => Box::pin(async move {
                let locked_value = (*value).borrow();
                if inner.is_equal(locked_value).await {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Const(true),
                        Output::Identity,
                        trace.done(),
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Const(false),
                        Output::Identity,
                        trace.done(),
                    ))
                }
            }),
            InnerType::Object(inner) => Box::pin(async move {
                let locked_value = (*value).borrow();
                if let Some(obj) = locked_value.try_get_object() {
                    let mut result = HashMap::new();
                    for field in &inner.fields {
                        if let Some(ref field_value) = obj.get(field.name()) {
                            result.insert(
                                field.name(),
                                field
                                    .ty()
                                    .evaluate(field_value.clone(), ctx, bindings, world)
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
                                    trace.done(),
                                ),
                            );
                        }
                    }
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Object(result),
                        Output::Identity,
                        trace.done(),
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::NotAnObject,
                        Output::None,
                        trace.done(),
                    ))
                }
            }),
            InnerType::Expr(expr) => Box::pin(async move {
                let result = expr.evaluate(value.clone()).await?;
                let _locked_value = (*value).borrow();
                let locked_result = (*result).borrow();
                if let Some(true) = locked_result.try_get_boolean() {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Expression(true),
                        Output::Identity,
                        trace.done(),
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        Some(value.clone()),
                        self.clone(),
                        Rationale::Expression(false),
                        Output::None,
                        trace.done(),
                    ))
                }
            }),
            InnerType::List(terms) => Box::pin(async move {
                if let Some(list_value) = value.try_get_list() {
                    if list_value.len() == terms.len() {
                        let mut result = Vec::new();
                        for (term, element) in terms.iter().zip(list_value.iter()) {
                            result
                                .push(term.evaluate(element.clone(), ctx, bindings, world).await?);
                        }
                        return Ok(EvaluationResult::new(
                            Some(value.clone()),
                            self.clone(),
                            Rationale::List(result),
                            Output::Identity,
                            trace.done(),
                        ));
                    }
                }
                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::NotAList,
                    Output::None,
                    trace.done(),
                ))
            }),
            InnerType::Nothing => Box::pin(async move {
                Ok(EvaluationResult::new(
                    Some(value.clone()),
                    self.clone(),
                    Rationale::Nothing,
                    Output::None,
                    trace.done(),
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
    Ref(SyntacticSugar, usize, Vec<Arc<Type>>),
    Deref(Arc<Type>),
    Argument(String),
    Const(ValueType),
    Object(ObjectType),
    Expr(Arc<Expr>),
    List(Vec<Arc<Type>>),
    Nothing,
}

impl InnerType {
    fn order(&self, world: &World) -> u8 {
        match self {
            Self::Anything => 128,
            Self::Primordial(t) => t.order(),
            Self::Bound(t, _) => t.order(world),
            Self::Ref(_, slot, _) => world
                .get_by_slot(*slot)
                .map(|t| t.order(world))
                .unwrap_or(128),
            Self::Deref(inner) => inner.order(world),
            Self::Argument(_s) => 2,
            Self::Const(_s) => 1,
            Self::Object(_o) => 64,
            Self::Expr(_e) => 128,
            Self::List(l) => l.iter().map(|e| e.order(world)).max().unwrap_or(128),
            Self::Nothing => 0,
        }
    }
}

#[derive(Serialize, Default, Debug, Clone)]
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
    #[allow(clippy::uninlined_format_args)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InnerType::Anything => write!(f, "anything"),
            InnerType::Primordial(inner) => write!(f, "{:?}", inner),
            InnerType::Const(inner) => write!(f, "{:?}", inner),
            InnerType::Object(inner) => write!(f, "{:?}", inner),
            InnerType::Expr(inner) => write!(f, "$({:?})", inner),
            InnerType::List(inner) => write!(f, "[{:?}]", inner),
            InnerType::Argument(name) => write!(f, "{:?}", name),
            InnerType::Ref(_sugar, slot, bindings) => write!(f, "ref {:?}<{:?}>", slot, bindings),
            InnerType::Deref(inner) => write!(f, "* {:?}", inner),
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
        Self { name, ty, optional }
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
                        Arc::new(RuntimeValue::from(copy))
                    })
                    .collect(),
            ),
            ValueType::Octets(val) => Self::Octets(val.clone()),
        }
    }
}

impl From<Arc<RuntimeValue>> for Type {
    fn from(val: Arc<RuntimeValue>) -> Self {
        Type::new(
            None,
            None,
            Vec::default(),
            match &*val {
                RuntimeValue::Null => InnerType::Const(ValueType::Null),
                RuntimeValue::String(inner) => InnerType::Const(ValueType::String(inner.clone())),
                RuntimeValue::Integer(inner) => InnerType::Const(ValueType::Integer(*inner)),
                RuntimeValue::Decimal(inner) => InnerType::Const(ValueType::Decimal(*inner)),
                RuntimeValue::Boolean(inner) => InnerType::Const(ValueType::Boolean(*inner)),
                RuntimeValue::Object(_) => {
                    todo!()
                }
                RuntimeValue::List(inner) => InnerType::List(
                    inner
                        .iter()
                        .map(|e| Arc::new(Self::from(e.clone())))
                        .collect(),
                ),
                RuntimeValue::Octets(inner) => InnerType::Const(ValueType::Octets(inner.clone())),
            },
        )
    }
}

impl ValueType {
    pub fn is_equal<'e>(
        &'e self,
        other: &'e RuntimeValue,
    ) -> Pin<Box<dyn Future<Output = bool> + 'e>> {
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
        mir::Type::Ref(sugar, slot, bindings) => {
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
                name,
                documentation,
                parameters,
                lir::InnerType::Ref(sugar.clone(), *slot, lir_bindings),
            ))
        }
        mir::Type::Deref(inner) => Arc::new(lir::Type::new(
            name,
            documentation,
            parameters,
            lir::InnerType::Deref(convert(
                inner.name(),
                inner.documentation(),
                inner.parameters().iter().map(|e| e.inner()).collect(),
                &inner.ty(),
            )),
        )),
        mir::Type::Argument(arg_name) => Arc::new(lir::Type::new(
            name,
            documentation,
            parameters,
            lir::InnerType::Argument(arg_name.clone()),
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
            lir::InnerType::Expr(Arc::new(expr.lower())),
        )),
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

#[derive(Debug)]
pub struct EvalContext {
    trace: EvalTrace,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            trace: EvalTrace::Disabled,
        }
    }
}

impl EvalContext {
    pub fn new(trace: EvalTrace) -> Self {
        Self { trace }
    }

    pub fn trace(&self) -> TraceHandle {
        match self.trace {
            EvalTrace::Enabled => TraceHandle {
                start: Some(Instant::now()),
            },
            EvalTrace::Disabled => TraceHandle { start: None },
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum EvalTrace {
    Enabled,
    Disabled,
}

#[derive(Clone, Debug, Copy)]
pub struct TraceHandle {
    start: Option<Instant>,
}

impl TraceHandle {
    fn done(self) -> Option<TraceResult> {
        self.start
            .map(|start| TraceResult::new(Instant::now() - start))
    }
}

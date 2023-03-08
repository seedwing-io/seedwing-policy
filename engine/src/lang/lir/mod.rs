use crate::lang::parser::Located;
use crate::lang::{lir, mir, PrimordialPattern, SyntacticSugar};
use crate::runtime::rationale::Rationale;
use crate::runtime::{EvalContext, EvaluationResult, Output, RuntimeError};
use crate::runtime::{PatternName, World};
use crate::value::RuntimeValue;
use serde::{Deserialize, Serialize};

use std::borrow::Borrow;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::future::Future;

use std::pin::Pin;

use std::sync::Arc;

/// Represents an expression of patterns.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Expr {
    SelfLiteral(),
    Value(ValuePattern),
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

/// A compiled pattern that can be evaluated.
#[derive(Debug, Serialize)]
pub struct Pattern {
    name: Option<PatternName>,
    documentation: Option<String>,
    parameters: Vec<String>,
    inner: InnerPattern,
}

impl Pattern {
    pub(crate) fn new(
        name: Option<PatternName>,
        documentation: Option<String>,
        parameters: Vec<String>,
        inner: InnerPattern,
    ) -> Self {
        Self {
            name,
            documentation,
            parameters,
            inner,
        }
    }

    /// Computational order of this pattern.
    pub fn order(&self, world: &World) -> u8 {
        self.inner.order(world)
    }

    /// Name of the pattern.
    pub fn name(&self) -> Option<PatternName> {
        self.name.clone()
    }

    /// Documentation for the pattern.
    pub fn documentation(&self) -> Option<String> {
        self.documentation.clone()
    }

    /// The inner pattern type.
    pub(crate) fn inner(&self) -> &InnerPattern {
        &self.inner
    }

    /// Parameters accepted by this pattern.
    pub fn parameters(&self) -> Vec<String> {
        self.parameters.clone()
    }

    /// Attempt to retrieve a const-ish value from this type.
    pub fn try_get_resolved_value(&self) -> Option<ValuePattern> {
        if let InnerPattern::Const(val) = &self.inner {
            Some(val.clone())
        } else {
            None
        }
    }

    /// Evaluate this pattern with the given input and bindings with the world and context for additional lookups.
    pub fn evaluate<'v>(
        self: &'v Arc<Self>,
        value: Arc<RuntimeValue>,
        ctx: &'v EvalContext,
        bindings: &'v Bindings,
        world: &'v World,
    ) -> Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>> {
        let trace = ctx.trace(value.clone(), self.clone());
        match &self.inner {
            InnerPattern::Anything => trace.run(Box::pin(async move {
                Ok(EvaluationResult::new(
                    value.clone(),
                    self.clone(),
                    Rationale::Anything,
                    Output::Identity,
                ))
            })),
            InnerPattern::Ref(sugar, slot, arguments) => trace.run(Box::pin(async move {
                if let Some(ty) = world.get_by_slot(*slot) {
                    let bindings = build_bindings(
                        value.clone(),
                        bindings.clone(),
                        ctx,
                        ty.parameters(),
                        arguments,
                        world,
                    )
                    .await;

                    let bindings = bindings.unwrap();
                    let result = ty.evaluate(value.clone(), ctx, &bindings, world).await?;
                    if let SyntacticSugar::Chain = sugar {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            result.rationale().clone(),
                            result.raw_output().clone(),
                        ))
                    } else {
                        Ok(result)
                    }
                } else {
                    Err(RuntimeError::NoSuchPatternSlot(*slot))
                }
            })),
            InnerPattern::Deref(inner) => trace.run(Box::pin(async move {
                inner.evaluate(value.clone(), ctx, bindings, world).await
            })),
            InnerPattern::Bound(ty, bindings) => trace.run(Box::pin(async move {
                ty.evaluate(value, ctx, bindings, world).await
            })),
            InnerPattern::Argument(name) => trace.run(Box::pin(async move {
                if let Some(bound) = bindings.get(name) {
                    bound.evaluate(value.clone(), ctx, bindings, world).await
                } else {
                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        Rationale::InvalidArgument(name.clone()),
                        Output::None,
                    ))
                }
            })),
            InnerPattern::Primordial(inner) => match inner {
                PrimordialPattern::Integer => trace.run(Box::pin(async move {
                    let locked_value = (*value).borrow();
                    if locked_value.is_integer() {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                })),
                PrimordialPattern::Decimal => trace.run(Box::pin(async move {
                    let locked_value = (*value).borrow();
                    if locked_value.is_decimal() {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                })),
                PrimordialPattern::Boolean => trace.run(Box::pin(async move {
                    let locked_value = (*value).borrow();

                    if locked_value.is_boolean() {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                })),
                PrimordialPattern::String => trace.run(Box::pin(async move {
                    let locked_value = (*value).borrow();
                    if locked_value.is_string() {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(true),
                            Output::Identity,
                        ))
                    } else {
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Primordial(false),
                            Output::None,
                        ))
                    }
                })),
                PrimordialPattern::Function(_sugar, _name, func) => {
                    trace.run(Box::pin(async move {
                        let result = func.call(value.clone(), ctx, bindings, world).await?;
                        Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::Function(
                                result.output().is_some(),
                                result.rationale().map(Box::new),
                                result.supporting(),
                            ),
                            result.output(),
                        ))
                    }))
                }
            },
            InnerPattern::Const(inner) => trace.run(Box::pin(async move {
                let locked_value = (*value).borrow();
                if inner.is_equal(locked_value) {
                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        Rationale::Const(true),
                        Output::Identity,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        Rationale::Const(false),
                        Output::Identity,
                    ))
                }
            })),
            InnerPattern::Object(inner) => trace.run(Box::pin(async move {
                let locked_value = (*value).borrow();
                if let Some(obj) = locked_value.try_get_object() {
                    let mut result = HashMap::new();
                    for field in &inner.fields {
                        if let Some(ref field_value) = obj.get(field.name()) {
                            result.insert(
                                field.name(),
                                Some(
                                    field
                                        .ty()
                                        .evaluate(field_value.clone(), ctx, bindings, world)
                                        .await?,
                                ),
                            );
                        } else if !field.optional() {
                            result.insert(field.name(), None);
                        }
                    }

                    let rationale = Rationale::Object(result);
                    let output = if rationale.satisfied() {
                        Output::Identity
                    } else {
                        Output::None
                    };

                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        rationale,
                        output,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        Rationale::NotAnObject,
                        Output::None,
                    ))
                }
            })),
            InnerPattern::Expr(expr) => trace.run(Box::pin(async move {
                let result = expr.evaluate(value.clone()).await?;
                let _locked_value = (*value).borrow();
                let locked_result = (*result).borrow();
                if let Some(true) = locked_result.try_get_boolean() {
                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        Rationale::Expression(true),
                        Output::Identity,
                    ))
                } else {
                    Ok(EvaluationResult::new(
                        value.clone(),
                        self.clone(),
                        Rationale::Expression(false),
                        Output::None,
                    ))
                }
            })),
            InnerPattern::List(terms) => trace.run(Box::pin(async move {
                if let Some(list_value) = value.try_get_list() {
                    if list_value.len() == terms.len() {
                        let mut result = Vec::new();
                        for (term, element) in terms.iter().zip(list_value.iter()) {
                            result
                                .push(term.evaluate(element.clone(), ctx, bindings, world).await?);
                        }
                        return Ok(EvaluationResult::new(
                            value.clone(),
                            self.clone(),
                            Rationale::List(result),
                            Output::Identity,
                        ));
                    }
                }
                Ok(EvaluationResult::new(
                    value.clone(),
                    self.clone(),
                    Rationale::NotAList,
                    Output::None,
                ))
            })),
            InnerPattern::Nothing => trace.run(Box::pin(async move {
                Ok(EvaluationResult::new(
                    value.clone(),
                    self.clone(),
                    Rationale::Nothing,
                    Output::None,
                ))
            })),
        }
    }
}

#[derive(Serialize)]
pub(crate) enum InnerPattern {
    Anything,
    Primordial(PrimordialPattern),
    Bound(Arc<Pattern>, Bindings),
    Ref(SyntacticSugar, usize, Vec<Arc<Pattern>>),
    Deref(Arc<Pattern>),
    Argument(String),
    Const(ValuePattern),
    Object(ObjectPattern),
    Expr(Arc<Expr>),
    List(Vec<Arc<Pattern>>),
    Nothing,
}

impl InnerPattern {
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

/// Bindings from names to patterns.
#[derive(Serialize, Default, Debug, Clone)]
pub struct Bindings {
    bindings: HashMap<String, Arc<Pattern>>,
}

impl Bindings {
    pub(crate) fn bind(&mut self, name: String, ty: Arc<Pattern>) {
        self.bindings.insert(name, ty);
    }

    /// Get the binding for a given name.
    pub fn get<S: Into<String>>(&self, name: S) -> Option<Arc<Pattern>> {
        self.bindings.get(&name.into()).cloned()
    }

    /// Iterator over all bindings.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<Pattern>)> {
        self.bindings.iter()
    }

    /// Number of bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Check if there are no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

impl Debug for InnerPattern {
    #[allow(clippy::uninlined_format_args)]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InnerPattern::Anything => write!(f, "anything"),
            InnerPattern::Primordial(inner) => write!(f, "{:?}", inner),
            InnerPattern::Const(inner) => write!(f, "{:?}", inner),
            InnerPattern::Object(inner) => write!(f, "{:?}", inner),
            InnerPattern::Expr(inner) => write!(f, "$({:?})", inner),
            InnerPattern::List(inner) => write!(f, "[{:?}]", inner),
            InnerPattern::Argument(name) => write!(f, "{:?}", name),
            InnerPattern::Ref(_sugar, slot, bindings) => {
                write!(f, "ref {:?}<{:?}>", slot, bindings)
            }
            InnerPattern::Deref(inner) => write!(f, "* {:?}", inner),
            InnerPattern::Bound(primary, bindings) => {
                write!(f, "bound {:?}<{:?}>", primary, bindings)
            }
            InnerPattern::Nothing => write!(f, "nothing"),
        }
    }
}

/// A field within an object pattern.
#[derive(Serialize, Debug)]
pub struct Field {
    name: String,
    ty: Arc<Pattern>,
    optional: bool,
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Field {
    pub fn new(name: String, ty: Arc<Pattern>, optional: bool) -> Self {
        Self { name, ty, optional }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn ty(&self) -> Arc<Pattern> {
        self.ty.clone()
    }

    pub fn optional(&self) -> bool {
        self.optional
    }
}

/// Pattern matching a specific value.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ValuePattern {
    Null,
    String(String),
    Integer(i64),
    Decimal(f64),
    Boolean(bool),
    List(Vec<Arc<ValuePattern>>),
    Octets(Vec<u8>),
}

impl From<&ValuePattern> for RuntimeValue {
    fn from(ty: &ValuePattern) -> Self {
        match ty {
            ValuePattern::Null => RuntimeValue::null(),
            ValuePattern::String(val) => val.clone().into(),
            ValuePattern::Integer(val) => (*val).into(),
            ValuePattern::Decimal(val) => (*val).into(),
            ValuePattern::Boolean(val) => (*val).into(),
            ValuePattern::List(val) => Self::List(
                val.iter()
                    .map(|e| {
                        let copy = &*e.clone();
                        Arc::new(RuntimeValue::from(copy))
                    })
                    .collect(),
            ),
            ValuePattern::Octets(val) => Self::Octets(val.clone()),
        }
    }
}

impl ValuePattern {
    pub fn is_equal(&self, other: &RuntimeValue) -> bool {
        match (self, &other) {
            (ValuePattern::Null, RuntimeValue::Null) => true,
            (ValuePattern::String(lhs), RuntimeValue::String(rhs)) => lhs.eq(rhs),
            (ValuePattern::Integer(lhs), RuntimeValue::Integer(rhs)) => lhs.eq(rhs),
            (ValuePattern::Decimal(lhs), RuntimeValue::Decimal(rhs)) => lhs.eq(rhs),
            (ValuePattern::Boolean(lhs), RuntimeValue::Boolean(rhs)) => lhs.eq(rhs),
            (ValuePattern::List(lhs), RuntimeValue::List(rhs)) => {
                if lhs.len() != rhs.len() {
                    false
                } else {
                    for (l, r) in lhs.iter().zip(rhs.iter()) {
                        if !l.is_equal(r) {
                            return false;
                        }
                    }
                    true
                }
            }
            (ValuePattern::Octets(lhs), RuntimeValue::Octets(rhs)) => lhs.eq(rhs),
            _ => false,
        }
    }
}

impl From<Arc<RuntimeValue>> for Pattern {
    fn from(val: Arc<RuntimeValue>) -> Self {
        Pattern::new(
            None,
            None,
            Vec::default(),
            match &*val {
                RuntimeValue::Null => InnerPattern::Const(ValuePattern::Null),
                RuntimeValue::String(inner) => {
                    InnerPattern::Const(ValuePattern::String(inner.clone()))
                }
                RuntimeValue::Integer(inner) => InnerPattern::Const(ValuePattern::Integer(*inner)),
                RuntimeValue::Decimal(inner) => InnerPattern::Const(ValuePattern::Decimal(*inner)),
                RuntimeValue::Boolean(inner) => InnerPattern::Const(ValuePattern::Boolean(*inner)),
                RuntimeValue::Object(_) => {
                    todo!("objects into patterns not yet implemented")
                }
                RuntimeValue::List(inner) => InnerPattern::List(
                    inner
                        .iter()
                        .map(|e| Arc::new(Self::from(e.clone())))
                        .collect(),
                ),
                RuntimeValue::Octets(inner) => {
                    InnerPattern::Const(ValuePattern::Octets(inner.clone()))
                }
            },
        )
    }
}

#[derive(Serialize, Debug, Clone)]
pub(crate) struct ObjectPattern {
    fields: Vec<Arc<Field>>,
}

impl ObjectPattern {
    pub fn new(fields: Vec<Arc<Field>>) -> Self {
        Self { fields }
    }

    pub fn fields(&self) -> &Vec<Arc<Field>> {
        &self.fields
    }
}

pub(crate) fn convert(
    name: Option<PatternName>,
    documentation: Option<String>,
    parameters: Vec<String>,
    ty: &Arc<Located<mir::Pattern>>,
) -> Arc<Pattern> {
    match &***ty {
        mir::Pattern::Anything => Arc::new(lir::Pattern::new(
            name,
            documentation,
            parameters,
            lir::InnerPattern::Anything,
        )),
        mir::Pattern::Primordial(primordial) => Arc::new(lir::Pattern::new(
            name,
            documentation,
            parameters,
            lir::InnerPattern::Primordial(primordial.clone()),
        )),
        mir::Pattern::Ref(sugar, slot, bindings) => {
            let mut lir_bindings = Vec::default();
            for e in bindings {
                lir_bindings.push(convert(
                    e.name(),
                    e.documentation(),
                    e.parameters().iter().map(|e| e.inner()).collect(),
                    &e.ty(),
                ));
            }

            Arc::new(lir::Pattern::new(
                name,
                documentation,
                parameters,
                lir::InnerPattern::Ref(sugar.clone(), *slot, lir_bindings),
            ))
        }
        mir::Pattern::Deref(inner) => Arc::new(lir::Pattern::new(
            name,
            documentation,
            parameters,
            lir::InnerPattern::Deref(convert(
                inner.name(),
                inner.documentation(),
                inner.parameters().iter().map(|e| e.inner()).collect(),
                &inner.ty(),
            )),
        )),
        mir::Pattern::Argument(arg_name) => Arc::new(lir::Pattern::new(
            name,
            documentation,
            parameters,
            lir::InnerPattern::Argument(arg_name.clone()),
        )),
        mir::Pattern::Const(value) => Arc::new(lir::Pattern::new(
            name,
            documentation,
            parameters,
            lir::InnerPattern::Const(value.clone()),
        )),
        mir::Pattern::Object(mir_object) => {
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
            let object = ObjectPattern::new(fields);
            Arc::new(lir::Pattern::new(
                name,
                documentation,
                parameters,
                lir::InnerPattern::Object(object),
            ))
        }
        mir::Pattern::Expr(expr) => Arc::new(lir::Pattern::new(
            name,
            documentation,
            parameters,
            lir::InnerPattern::Expr(Arc::new(expr.lower())),
        )),
        mir::Pattern::List(terms) => {
            let mut inner = Vec::new();
            for e in terms {
                inner.push(convert(
                    e.name(),
                    e.documentation(),
                    e.parameters().iter().map(|e| e.inner()).collect(),
                    &e.ty(),
                ))
            }

            Arc::new(lir::Pattern::new(
                name,
                documentation,
                parameters,
                lir::InnerPattern::List(inner),
            ))
        }
        mir::Pattern::Nothing => Arc::new(lir::Pattern::new(
            name,
            documentation,
            Vec::default(),
            lir::InnerPattern::Nothing,
        )),
    }
}

fn build_bindings<'b>(
    value: Arc<RuntimeValue>,
    mut bindings: Bindings,
    ctx: &'b EvalContext,
    parameters: Vec<String>,
    arguments: &'b Vec<Arc<Pattern>>,
    world: &'b World,
) -> Pin<Box<dyn Future<Output = Result<Bindings, RuntimeError>> + 'b>> {
    Box::pin(async move {
        for (param, arg) in parameters.iter().zip(arguments.iter()) {
            if let InnerPattern::Ref(_sugar, slot, unresolved_bindings) = &arg.inner {
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
                            Arc::new(Pattern::new(
                                resolved_type.name(),
                                resolved_type.documentation(),
                                resolved_type.parameters(),
                                InnerPattern::Bound(resolved_type, resolved_bindings),
                            )),
                        )
                    }
                }
            } else if let InnerPattern::Argument(name) = &arg.inner {
                bindings.bind(param.clone(), bindings.get(name).unwrap());
            } else if let InnerPattern::Deref(_) | InnerPattern::List(_) = &arg.inner {
                bindings.bind(
                    param.clone(),
                    possibly_deref(value.clone(), arg.clone(), ctx, world).await?,
                );
            } else {
                bindings.bind(param.clone(), arg.clone())
            }
        }

        Ok(bindings)
    })
}

fn possibly_deref<'b>(
    value: Arc<RuntimeValue>,
    arg: Arc<Pattern>,
    ctx: &'b EvalContext,
    world: &'b World,
) -> Pin<Box<dyn Future<Output = Result<Arc<Pattern>, RuntimeError>> + 'b>> {
    Box::pin(async move {
        if let InnerPattern::Deref(_inner) = &arg.inner {
            let result = arg
                .evaluate(value.clone(), ctx, &Bindings::default(), world)
                .await?;

            if result.satisfied() {
                if let Some(output) = result.output() {
                    Ok(Arc::new(output.into()))
                } else {
                    Ok(Arc::new(Pattern::new(
                        None,
                        None,
                        Vec::default(),
                        InnerPattern::Nothing,
                    )))
                }
            } else {
                Ok(Arc::new(Pattern::new(
                    None,
                    None,
                    Vec::default(),
                    InnerPattern::Nothing,
                )))
            }
        } else if let InnerPattern::List(terms) = &arg.inner {
            let mut replacement = Vec::new();
            for term in terms {
                replacement.push(possibly_deref(value.clone(), term.clone(), ctx, world).await?);
            }

            Ok(Arc::new(Pattern::new(
                None,
                None,
                Vec::default(),
                InnerPattern::List(replacement),
            )))
        } else {
            Ok(arg)
        }
    })
}

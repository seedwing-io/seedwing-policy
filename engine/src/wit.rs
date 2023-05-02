use crate::data::MemDataSource;
use crate::data::MemDataSourceType;
use crate::lang::builder::Builder;
use crate::lang::lir::InnerPattern;
use crate::lang::lir::ObjectPattern;
use crate::lang::Expr;
use crate::lang::PrimordialPattern;
use crate::lang::Severity;
use crate::lang::SyntacticSugar;
use crate::lang::ValuePattern;
use crate::runtime::rationale::Rationale;
use crate::runtime::sources::Ephemeral;
use crate::runtime::EvalContext;
use crate::runtime::EvaluationResult;
use crate::runtime::Example;
use crate::runtime::Pattern;
use crate::runtime::PatternName;
use crate::value::RuntimeValue;
use crate::wit::exports::seedwing::policy::engine::Engine;
use crate::wit::seedwing::policy::types as wit_types;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

wit_bindgen::generate!("engine-world");

struct Exports;

struct WitContext {
    pattern_map: HashMap<String, wit_types::Pattern>,
    evaluation_result_map: HashMap<String, wit_types::EvaluationResult>,
    rationale_map: HashMap<String, wit_types::Rationale>,
    expr_map: HashMap<String, wit_types::Expr>,
}

impl WitContext {
    fn new() -> Self {
        Self {
            pattern_map: HashMap::new(),
            evaluation_result_map: HashMap::new(),
            rationale_map: HashMap::new(),
            expr_map: HashMap::new(),
        }
    }
}

impl Engine for Exports {
    fn version() -> String {
        crate::version().to_string()
    }

    fn eval(
        _policies: Vec<String>,
        data: Vec<(String, wit_types::DataType)>,
        policy: String,
        name: String,
        input: wit_types::RuntimeValue,
    ) -> Result<wit_types::EvaluationResultOuter, String> {
        let mut builder = Builder::new();
        builder.data(MemDataSource::from(data));
        let _res = builder.build(Ephemeral::new("wit", policy).iter()).unwrap();
        let evaluation_result = futures::executor::block_on(async {
            let runtime = builder.finish().await;
            runtime
                .unwrap()
                .evaluate(format!("wit::{name}"), &input, EvalContext::default())
                .await
        });
        match evaluation_result {
            Ok(result) => {
                let mut eval_context = WitContext::new();
                let wit_evaluation_result =
                    wit_types::EvaluationResult::from_with_context(&result, &mut eval_context);

                let wit_result_outer = wit_types::EvaluationResultOuter {
                    evaluation_result: wit_evaluation_result,
                    pattern_map: Vec::from_iter(eval_context.pattern_map.into_iter()),
                    evaluation_result_map: Vec::from_iter(
                        eval_context.evaluation_result_map.into_iter(),
                    ),
                    rationale_map: Vec::from_iter(eval_context.rationale_map.into_iter()),
                    expr_map: Vec::from_iter(eval_context.expr_map.into_iter()),
                };
                Ok(wit_result_outer)
            }
            Err(e) => Err(format!("Error processing rule: {e}")),
        }
    }
}

export_engine_world!(Exports);

impl From<Vec<(String, wit_types::DataType)>> for MemDataSource {
    fn from(ds: Vec<(String, wit_types::DataType)>) -> Self {
        let mut map = HashMap::new();
        for item in ds {
            map.insert(item.0, item.1.into());
        }
        MemDataSource::new(map)
    }
}

impl From<wit_types::DataType> for MemDataSourceType {
    fn from(wit_dt: wit_types::DataType) -> Self {
        match wit_dt {
            wit_types::DataType::String(string) => MemDataSourceType::String(string),
            wit_types::DataType::Bytes(bytes) => MemDataSourceType::Bytes(bytes),
        }
    }
}

impl wit_types::Rationale {
    fn from_with_context(rationale: &Rationale, context: &mut WitContext) -> Self {
        match rationale {
            Rationale::Anything => wit_types::Rationale::Anything,
            Rationale::Nothing => wit_types::Rationale::Nothing,
            Rationale::NotAnObject => wit_types::Rationale::NotAnObject,
            Rationale::NotAList => wit_types::Rationale::NotAList,
            Rationale::MissingField(field) => wit_types::Rationale::MissingField(field.to_string()),
            Rationale::InvalidArgument(arg) => {
                wit_types::Rationale::InvalidArgument(arg.to_string())
            }
            Rationale::Const(boolean) => wit_types::Rationale::Const(*boolean),
            Rationale::Primordial(boolean) => wit_types::Rationale::Primordial(*boolean),
            Rationale::Expression(boolean) => wit_types::Rationale::Expression(*boolean),
            Rationale::Object(obj_map) => {
                let mut eval_refs: Vec<(String, Option<wit_types::EvaluationResultRef>)> =
                    Vec::with_capacity(obj_map.len());

                for (key, res) in obj_map.iter() {
                    let eval_result_ref = res.as_ref().map(|ev| {
                        let eval_id = Uuid::new_v4().to_string();
                        let wit_eval_result = wit_types::EvaluationResult::from_with_context(
                            &(**ev).clone(),
                            context,
                        );
                        context
                            .evaluation_result_map
                            .insert(eval_id.to_string(), wit_eval_result);
                        wit_types::EvaluationResultRef { eval_id }
                    });
                    eval_refs.push((key.to_string(), eval_result_ref));
                }
                wit_types::Rationale::Object(eval_refs)
            }
            Rationale::Chain(list) => {
                let mut eval_refs: Vec<wit_types::EvaluationResultRef> =
                    Vec::with_capacity(list.len());
                for evaluation_result in &**list {
                    let wit_eval_result =
                        wit_types::EvaluationResult::from_with_context(evaluation_result, context);
                    let eval_id = Uuid::new_v4().to_string();
                    context
                        .evaluation_result_map
                        .insert(eval_id.to_string(), wit_eval_result);
                    eval_refs.push(wit_types::EvaluationResultRef { eval_id });
                }
                wit_types::Rationale::Chain(eval_refs)
            }
            Rationale::List(list) => {
                let mut eval_refs: Vec<wit_types::EvaluationResultRef> =
                    Vec::with_capacity(list.len());
                for evaluation_result in &**list {
                    let wit_eval_result =
                        wit_types::EvaluationResult::from_with_context(evaluation_result, context);
                    let eval_id = Uuid::new_v4().to_string();
                    context
                        .evaluation_result_map
                        .insert(eval_id.to_string(), wit_eval_result);
                    eval_refs.push(wit_types::EvaluationResultRef { eval_id });
                }
                wit_types::Rationale::List(eval_refs)
            }
            Rationale::Bound(rationale, bindings) => {
                let wit_rationale = Self::from_with_context(rationale, context);
                let rationale_id = Uuid::new_v4().to_string();
                context
                    .rationale_map
                    .insert(rationale_id.to_string(), wit_rationale);
                let rationale_ref = wit_types::RationaleRef { rationale_id };
                let mut wit_bindings = Vec::with_capacity(bindings.len());
                for (name, pattern) in bindings.iter() {
                    let pattern_id = wit_types::Pattern::add_to_map(&pattern, context);
                    let pattern_ref = wit_types::PatternRef { pattern_id };
                    wit_bindings.push((name.to_string(), pattern_ref));
                }
                let wb = wit_types::Bindings {
                    bindings_map: wit_bindings,
                };
                wit_types::Rationale::Bound((rationale_ref, wb))
            }
            Rationale::Function {
                severity,
                rationale,
                supporting,
            } => {
                let wit_severity = (*severity).into();

                let wit_rationale = rationale.as_ref().map(|r| {
                    let wit_rationale = Self::from_with_context(&*r, context);
                    let rationale_id = Uuid::new_v4().to_string();
                    context
                        .rationale_map
                        .insert(rationale_id.to_string(), wit_rationale);
                    let rationale_ref = wit_types::RationaleRef { rationale_id };
                    rationale_ref
                });

                let mut eval_refs: Vec<wit_types::EvaluationResultRef> =
                    Vec::with_capacity(supporting.len());
                for evaluation_result in supporting.iter() {
                    let wit_eval_result =
                        wit_types::EvaluationResult::from_with_context(evaluation_result, context);
                    let eval_id = Uuid::new_v4().to_string();
                    context
                        .evaluation_result_map
                        .insert(eval_id.to_string(), wit_eval_result);
                    eval_refs.push(wit_types::EvaluationResultRef { eval_id });
                }

                let function = wit_types::Function {
                    severity: wit_severity,
                    rationale: wit_rationale,
                    supporting: eval_refs,
                };
                wit_types::Rationale::Function(function)
            }
        }
    }
}

impl wit_types::EvaluationResult {
    fn from_with_context(evaluation_result: &EvaluationResult, context: &mut WitContext) -> Self {
        let ty = &evaluation_result.ty;
        let wit_ty = wit_types::Pattern::from_with_context(ty, context);
        let wit_rationale =
            wit_types::Rationale::from_with_context(&evaluation_result.rationale, context);

        Self {
            input: (&(*evaluation_result.input)).into(),
            ty: wit_ty,
            rationale: wit_rationale,
            output: format!("{:?}", evaluation_result.output),
        }
    }
}

impl From<Severity> for wit_types::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::None => wit_types::Severity::None,
            Severity::Advice => wit_types::Severity::Advice,
            Severity::Warning => wit_types::Severity::Warning,
            Severity::Error => wit_types::Severity::Error,
        }
    }
}

impl From<Example> for wit_types::Example {
    fn from(example: Example) -> Self {
        wit_types::Example {
            name: example.name,
            summary: example.summary,
            description: example.description,
            value: example.value.as_str().unwrap_or("").to_string(),
        }
    }
}

impl From<&wit_types::RuntimeValue> for crate::value::RuntimeValue {
    fn from(input: &wit_types::RuntimeValue) -> Self {
        match input {
            wit_types::RuntimeValue::Null => RuntimeValue::Null,
            wit_types::RuntimeValue::String(value) => {
                return RuntimeValue::String(value.to_string().into());
            }
            wit_types::RuntimeValue::Integer(value) => RuntimeValue::Integer(*value),
            wit_types::RuntimeValue::Decimal(value) => RuntimeValue::Decimal(value.clone()),
            wit_types::RuntimeValue::Boolean(value) => RuntimeValue::Boolean(*value),
            wit_types::RuntimeValue::List(list) => {
                let mut core_rt_values = Vec::with_capacity(list.len());
                for item in list {
                    core_rt_values.push(Arc::new(item.into()));
                }
                RuntimeValue::List(core_rt_values)
            }
            wit_types::RuntimeValue::Object(list) => {
                let mut engine_object = crate::value::Object::new();
                for item in list {
                    let key = item.key.as_str().clone();
                    let value = RuntimeValue::from(&item.value);
                    engine_object.set(key, value);
                }
                crate::value::RuntimeValue::Object(engine_object)
            }
            wit_types::RuntimeValue::Octets(bytes) => {
                crate::value::RuntimeValue::Octets(bytes.to_vec())
            }
        }
    }
}

impl From<&RuntimeValue> for wit_types::RuntimeValue {
    fn from(value: &RuntimeValue) -> Self {
        match value {
            RuntimeValue::Null => wit_types::RuntimeValue::Null,
            RuntimeValue::String(value) => {
                wit_types::RuntimeValue::String(value.to_string().into())
            }
            RuntimeValue::Integer(value) => wit_types::RuntimeValue::Integer(*value),
            RuntimeValue::Decimal(value) => wit_types::RuntimeValue::Decimal(value.clone()),
            RuntimeValue::Boolean(value) => wit_types::RuntimeValue::Boolean(*value),
            RuntimeValue::List(list) => {
                let mut values: Vec<wit_types::BaseValue> = Vec::with_capacity(list.len());
                for item in list {
                    let rt_value = &**item;
                    values.push(rt_value.into());
                }
                wit_types::RuntimeValue::List(values)
            }
            RuntimeValue::Octets(value) => wit_types::RuntimeValue::Octets(value.to_vec()),
            RuntimeValue::Object(object) => {
                let mut list: Vec<wit_types::Object> = Vec::new();
                for (key, value) in object.iter() {
                    let key = &**key;
                    let value = &**value;
                    let wit_object = wit_types::Object {
                        key: key.into(),
                        value: value.into(),
                    };
                    list.push(wit_object);
                }
                wit_types::RuntimeValue::Object(list)
            }
        }
    }
}

impl From<&RuntimeValue> for wit_types::ObjectValue {
    fn from(value: &RuntimeValue) -> Self {
        match value {
            RuntimeValue::Null => wit_types::ObjectValue::Null,
            RuntimeValue::String(value) => wit_types::ObjectValue::String(value.to_string().into()),
            RuntimeValue::Integer(value) => wit_types::ObjectValue::Integer(*value),
            RuntimeValue::Decimal(value) => wit_types::ObjectValue::Decimal(value.clone()),
            RuntimeValue::Boolean(value) => wit_types::ObjectValue::Boolean(*value),
            RuntimeValue::List(list) => {
                let mut values: Vec<wit_types::BaseValue> = Vec::with_capacity(list.len());
                for item in list {
                    let rt_value = &**item;
                    values.push(rt_value.into());
                }
                wit_types::ObjectValue::List(values)
            }
            RuntimeValue::Octets(value) => wit_types::ObjectValue::Octets(value.to_vec()),
            RuntimeValue::Object(_) => wit_types::ObjectValue::Null,
        }
    }
}

impl From<&RuntimeValue> for wit_types::BaseValue {
    fn from(value: &RuntimeValue) -> Self {
        match value {
            RuntimeValue::Null => wit_types::BaseValue::Null,
            RuntimeValue::String(value) => wit_types::BaseValue::String(value.to_string().into()),
            RuntimeValue::Integer(value) => wit_types::BaseValue::Integer(*value),
            RuntimeValue::Decimal(value) => wit_types::BaseValue::Decimal(value.clone()),
            RuntimeValue::Boolean(value) => wit_types::BaseValue::Boolean(*value),
            _ => wit_types::BaseValue::Null,
        }
    }
}

impl From<&wit_types::ObjectValue> for crate::value::RuntimeValue {
    fn from(obj: &wit_types::ObjectValue) -> Self {
        match obj {
            wit_types::ObjectValue::Null => RuntimeValue::Null,
            wit_types::ObjectValue::String(value) => RuntimeValue::String(value.to_string().into()),
            wit_types::ObjectValue::Integer(value) => RuntimeValue::Integer(*value),
            wit_types::ObjectValue::Decimal(value) => RuntimeValue::Decimal(value.clone()),
            wit_types::ObjectValue::Boolean(value) => RuntimeValue::Boolean(*value),
            wit_types::ObjectValue::List(list) => {
                let mut core_rt_values: Vec<Arc<RuntimeValue>> = Vec::with_capacity(list.len());
                for item in list {
                    core_rt_values.push(Arc::new(item.into()));
                }
                RuntimeValue::List(core_rt_values)
            }
            wit_types::ObjectValue::Octets(bytes) => {
                crate::value::RuntimeValue::Octets(bytes.to_vec())
            }
        }
    }
}

impl From<&wit_types::BaseValue> for RuntimeValue {
    fn from(base: &wit_types::BaseValue) -> Self {
        match &base {
            wit_types::BaseValue::Null => RuntimeValue::Null,
            wit_types::BaseValue::String(value) => {
                return RuntimeValue::String(value.to_string().into());
            }
            wit_types::BaseValue::Integer(value) => RuntimeValue::Integer(*value),
            wit_types::BaseValue::Decimal(value) => RuntimeValue::Decimal(value.clone()),
            wit_types::BaseValue::Boolean(value) => RuntimeValue::Boolean(*value),
            wit_types::BaseValue::Octets(bytes) => {
                crate::value::RuntimeValue::Octets(bytes.to_vec())
            }
        }
    }
}

impl From<PatternName> for wit_types::PatternName {
    fn from(pattern_name: PatternName) -> Self {
        let package = match pattern_name.package {
            Some(ref package_path) => {
                let ps: Vec<String> = package_path
                    .path
                    .iter()
                    .map(|name| name.to_string())
                    .collect();
                Some(wit_types::PackagePath { path: ps })
            }
            None => None,
        };
        let name = pattern_name.name().to_string();
        wit_types::PatternName { package, name }
    }
}

impl wit_types::Pattern {
    fn from_with_context(pattern: &Pattern, context: &mut WitContext) -> Self {
        let pattern_name = match pattern.name() {
            Some(pattern_name) => Some(pattern_name.into()),
            None => None,
        };

        let deprecation = match &pattern.metadata().deprecation {
            Some(ref d) => Some(wit_types::Deprecation {
                reason: d.reason.clone(),
                since: d.since.clone(),
            }),
            None => None,
        };

        let reporting = wit_types::Reporting {
            severity: pattern.metadata().reporting.severity.into(),
            explanation: pattern.metadata().reporting.explanation.clone(),
            authoritative: pattern.metadata().reporting.authoritative,
        };

        let metadata = wit_types::PatternMeta {
            documentation: pattern.metadata().documentation.0.clone(),
            unstable: pattern.metadata().unstable,
            deprecation,
            reporting,
        };

        let pattern_id = Uuid::new_v4().to_string();
        let wit_pattern = wit_types::Pattern {
            id: pattern_id.clone(),
            name: pattern_name,
            metadata,
            examples: pattern.examples().into_iter().map(|e| e.into()).collect(),
            parameters: pattern
                .parameters()
                .into_iter()
                .map(|p| p.to_string())
                .collect(),
            inner: wit_types::InnerPattern::from_with_context((*pattern.inner()).clone(), context),
        };
        context.pattern_map.insert(pattern_id, wit_pattern.clone());
        wit_pattern
    }

    fn add_to_map(pattern: &Pattern, context: &mut WitContext) -> String {
        let pattern_name = match pattern.name() {
            Some(pattern_name) => Some(pattern_name.into()),
            None => None,
        };

        let deprecation = match &pattern.metadata().deprecation {
            Some(ref d) => Some(wit_types::Deprecation {
                reason: d.reason.clone(),
                since: d.since.clone(),
            }),
            None => None,
        };

        let reporting = wit_types::Reporting {
            severity: pattern.metadata().reporting.severity.into(),
            explanation: pattern.metadata().reporting.explanation.clone(),
            authoritative: pattern.metadata().reporting.authoritative,
        };

        let metadata = wit_types::PatternMeta {
            documentation: pattern.metadata().documentation.0.clone(),
            unstable: pattern.metadata().unstable,
            deprecation,
            reporting,
        };

        let pattern_id = Uuid::new_v4().to_string();
        let wit_pattern = wit_types::Pattern {
            id: pattern_id.clone(),
            name: pattern_name,
            metadata,
            examples: pattern.examples().into_iter().map(|e| e.into()).collect(),
            parameters: pattern
                .parameters()
                .into_iter()
                .map(|p| p.to_string())
                .collect(),
            inner: wit_types::InnerPattern::from_with_context((*pattern.inner()).clone(), context),
        };
        context
            .pattern_map
            .insert(pattern_id.clone(), wit_pattern.clone());
        pattern_id
    }
}

impl wit_types::InnerPattern {
    fn from_with_context(inner: InnerPattern, context: &mut WitContext) -> Self {
        match inner {
            InnerPattern::Anything => wit_types::InnerPattern::Anything,
            InnerPattern::Primordial(pattern) => {
                wit_types::InnerPattern::Primordial(pattern.into())
            }
            InnerPattern::Argument(value) => wit_types::InnerPattern::Argument(value.to_string()),
            InnerPattern::Const(value_pattern) => {
                wit_types::InnerPattern::Const(value_pattern.into())
            }
            InnerPattern::Object(object_pattern) => wit_types::InnerPattern::Object(
                wit_types::ObjectPattern::from_with_context(object_pattern, context),
            ),
            InnerPattern::Nothing => wit_types::InnerPattern::Nothing,
            InnerPattern::Ref(suger, slot, list) => {
                let mut pattern_refs = Vec::with_capacity(list.len());
                for pattern in list {
                    let pattern_id = wit_types::Pattern::add_to_map(&pattern, context);
                    pattern_refs.push(wit_types::PatternRef { pattern_id });
                }
                wit_types::InnerPattern::Ref((suger.into(), slot as u32, pattern_refs))
            }
            InnerPattern::Bound(pattern, bindings) => {
                let pattern_id = wit_types::Pattern::add_to_map(&pattern, context);
                let mut wit_bindings = Vec::with_capacity(bindings.len());
                for (name, pattern) in bindings.iter() {
                    let pattern_id = wit_types::Pattern::add_to_map(&pattern, context);
                    let pattern_ref = wit_types::PatternRef { pattern_id };
                    wit_bindings.push((name.to_string(), pattern_ref));
                }
                let wb = wit_types::Bindings {
                    bindings_map: wit_bindings,
                };
                wit_types::InnerPattern::Bound((wit_types::PatternRef { pattern_id }, wb))
            }
            InnerPattern::Expr(expr) => {
                let wit_expr = wit_types::Expr::from_with_context((*expr).clone(), context);
                wit_types::InnerPattern::Expr(wit_expr)
            }
            _ => wit_types::InnerPattern::Anything,
        }
    }
}

impl wit_types::Expr {
    fn from_with_context(expr: Expr, context: &mut WitContext) -> Self {
        match expr {
            Expr::SelfLiteral() => wit_types::Expr::SelfLiteral,
            Expr::Value(pattern) => wit_types::Expr::Value(pattern.into()),
            Expr::Function(string, expr) => {
                let wit_expr = Self::from_with_context((*expr).clone(), context);
                let expr_ref = Self::add_to_map(wit_expr, context);
                wit_types::Expr::Function((string, expr_ref))
            }
            Expr::Add(lhs, rhs) => {
                wit_types::Expr::Add(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::Subtract(lhs, rhs) => {
                wit_types::Expr::Subtract(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::Multiply(lhs, rhs) => {
                wit_types::Expr::Multiply(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::Divide(lhs, rhs) => {
                wit_types::Expr::Divide(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::LessThan(lhs, rhs) => {
                wit_types::Expr::LessThan(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::LessThanEqual(lhs, rhs) => wit_types::Expr::LessThanEqual(Self::binary(
                (*lhs).clone(),
                (*rhs).clone(),
                context,
            )),
            Expr::GreaterThan(lhs, rhs) => {
                wit_types::Expr::GreaterThan(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::GreaterThanEqual(lhs, rhs) => wit_types::Expr::GreaterThanEqual(Self::binary(
                (*lhs).clone(),
                (*rhs).clone(),
                context,
            )),
            Expr::Equal(lhs, rhs) => {
                wit_types::Expr::Equal(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::NotEqual(lhs, rhs) => {
                wit_types::Expr::NotEqual(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::LogicalAnd(lhs, rhs) => {
                wit_types::Expr::LogicalAnd(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::LogicalOr(lhs, rhs) => {
                wit_types::Expr::LogicalOr(Self::binary((*lhs).clone(), (*rhs).clone(), context))
            }
            Expr::Not(expr) => {
                let wit_ref =
                    Self::add_to_map(Self::from_with_context((*expr).clone(), context), context);
                wit_types::Expr::Not(wit_ref)
            }
        }
    }

    fn binary(
        lhs: Expr,
        rhs: Expr,
        context: &mut WitContext,
    ) -> (wit_types::ExprRef, wit_types::ExprRef) {
        let wit_lhs_expr = Self::from_with_context(lhs, context);
        let wit_lhs_ref = Self::add_to_map(wit_lhs_expr, context);
        let wit_rhs_expr = Self::from_with_context(rhs, context);
        let wit_rhs_ref = Self::add_to_map(wit_rhs_expr, context);
        (wit_lhs_ref, wit_rhs_ref)
    }

    fn add_to_map(expr: wit_types::Expr, context: &mut WitContext) -> wit_types::ExprRef {
        let expr_id = Uuid::new_v4().to_string();
        context.expr_map.insert(expr_id.to_string(), expr);
        wit_types::ExprRef { expr_id }
    }
}

impl wit_types::ObjectPattern {
    fn from_with_context(object_pattern: ObjectPattern, context: &mut WitContext) -> Self {
        let fields = object_pattern
            .fields()
            .iter()
            .map(|f| {
                let pattern_id = wit_types::Pattern::add_to_map(&f.ty(), context);
                wit_types::Field {
                    ty: wit_types::PatternRef {
                        pattern_id: pattern_id.clone(),
                    },
                    name: f.name().to_string(),
                    optional: f.optional(),
                }
            })
            .collect();
        wit_types::ObjectPattern { fields }
    }
}

impl From<ValuePattern> for wit_types::ValuePattern {
    fn from(value_pattern: ValuePattern) -> Self {
        match value_pattern {
            ValuePattern::Null => wit_types::ValuePattern::Null,
            ValuePattern::String(value) => wit_types::ValuePattern::String(value.to_string()),
            ValuePattern::Integer(value) => wit_types::ValuePattern::Integer(value),
            ValuePattern::Decimal(value) => wit_types::ValuePattern::Decimal(value),
            ValuePattern::Boolean(value) => wit_types::ValuePattern::Boolean(value),
            ValuePattern::Octets(bytes) => wit_types::ValuePattern::Octets(bytes),
            _ => wit_types::ValuePattern::Null,
        }
    }
}

impl From<PrimordialPattern> for wit_types::PrimordialPattern {
    fn from(p: PrimordialPattern) -> Self {
        match p {
            PrimordialPattern::Integer => wit_types::PrimordialPattern::Integer,
            PrimordialPattern::Decimal => wit_types::PrimordialPattern::Decimal,
            PrimordialPattern::Boolean => wit_types::PrimordialPattern::Boolean,
            PrimordialPattern::String => wit_types::PrimordialPattern::String,
            PrimordialPattern::Function(syn, name, _) => {
                wit_types::PrimordialPattern::Function((syn.into(), name.into()))
            }
        }
    }
}

impl From<SyntacticSugar> for wit_types::SyntacticSugar {
    fn from(syn: SyntacticSugar) -> Self {
        match syn {
            SyntacticSugar::None => wit_types::SyntacticSugar::None,
            SyntacticSugar::And => wit_types::SyntacticSugar::And,
            SyntacticSugar::Or => wit_types::SyntacticSugar::Or,
            SyntacticSugar::Refine => wit_types::SyntacticSugar::Refine,
            SyntacticSugar::Traverse => wit_types::SyntacticSugar::Traverse,
            SyntacticSugar::Chain => wit_types::SyntacticSugar::Chain,
            SyntacticSugar::Not => wit_types::SyntacticSugar::Not,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn from_engine_runtime() {
        println!("from_engine_runtime...");
    }
}

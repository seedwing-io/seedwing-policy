use crate::core::FunctionInput;
use crate::lang::lir::{InnerPattern, Pattern};
use crate::lang::{PrimordialPattern, ValuePattern};
use crate::runtime::World;
use schemars::schema::{InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec};
use serde_json::{json, Value};
use std::sync::Arc;

impl Pattern {
    pub fn as_json_schema(&self, world: &World, bindings: &Vec<Arc<Pattern>>) -> SchemaObject {
        match &self.inner {
            InnerPattern::Anything => Schema::Bool(true).into_object(),
            InnerPattern::Primordial(inner) => match inner {
                PrimordialPattern::Integer => SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                    ..Default::default()
                },
                PrimordialPattern::Decimal => SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                    ..Default::default()
                },
                PrimordialPattern::Boolean => SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Boolean))),
                    ..Default::default()
                },
                PrimordialPattern::String => SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                    ..Default::default()
                },
                PrimordialPattern::Function(_, _name, func) => match &func.input(bindings) {
                    FunctionInput::Anything => Schema::Bool(true).into_object(),
                    FunctionInput::String => SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                        ..Default::default()
                    },
                    FunctionInput::Boolean => SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Boolean))),
                        ..Default::default()
                    },
                    FunctionInput::Integer => SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                        ..Default::default()
                    },
                    FunctionInput::Decimal => SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                        ..Default::default()
                    },
                    FunctionInput::Pattern(_pattern) => Schema::Bool(false).into_object(),
                },
            },
            //InnerPattern::Bound(_, _) => {}
            InnerPattern::Ref(_, slot, bindings) => {
                if let Some(inner) = world.get_by_slot(*slot) {
                    if inner.parameters.is_empty() {
                        if let Some(name) = &inner.name {
                            let type_name = name.as_type_str();
                            if type_name == "string" {
                                SchemaObject {
                                    instance_type: Some(SingleOrVec::Single(Box::new(
                                        InstanceType::String,
                                    ))),
                                    ..Default::default()
                                }
                            } else if type_name == "boolean" {
                                SchemaObject {
                                    instance_type: Some(SingleOrVec::Single(Box::new(
                                        InstanceType::Boolean,
                                    ))),
                                    ..Default::default()
                                }
                            } else if type_name == "decimal" {
                                SchemaObject {
                                    instance_type: Some(SingleOrVec::Single(Box::new(
                                        InstanceType::Number,
                                    ))),
                                    ..Default::default()
                                }
                            } else if type_name == "integer" {
                                SchemaObject {
                                    instance_type: Some(SingleOrVec::Single(Box::new(
                                        InstanceType::String,
                                    ))),
                                    ..Default::default()
                                }
                            } else {
                                SchemaObject::new_ref(name.as_type_str())
                            }
                        } else {
                            inner.as_json_schema(world, bindings)
                        }
                    } else {
                        inner.as_json_schema(world, bindings)
                    }
                } else {
                    Schema::Bool(false).into_object()
                }
            }
            //InnerPattern::Deref(_) => {}
            //InnerPattern::Argument(_) => {}
            InnerPattern::Const(val) => match val {
                ValuePattern::Null => SchemaObject {
                    const_value: Some(Value::Null),
                    ..Default::default()
                },
                ValuePattern::String(inner) => SchemaObject {
                    const_value: Some(json!(inner)),
                    ..Default::default()
                },
                ValuePattern::Integer(inner) => SchemaObject {
                    const_value: Some(json!(inner)),
                    ..Default::default()
                },
                ValuePattern::Decimal(inner) => SchemaObject {
                    const_value: Some(json!(inner)),
                    ..Default::default()
                },
                ValuePattern::Boolean(inner) => SchemaObject {
                    const_value: Some(json!(*inner)),
                    ..Default::default()
                },
                ValuePattern::List(_) => {
                    todo!("impl list")
                }
                ValuePattern::Octets(_) => {
                    todo!("impl octets")
                }
            },
            InnerPattern::Object(obj) => {
                let mut validation = ObjectValidation::default();
                for field in obj.fields() {
                    validation.properties.insert(
                        field.name().to_string(),
                        field.ty().as_json_schema(world, bindings).into(),
                    );
                }

                SchemaObject {
                    instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Object))),
                    object: Some(Box::new(validation)),
                    ..Default::default()
                }
            }
            //InnerPattern::Expr(_) => {}
            //InnerPattern::List(_) => {}
            InnerPattern::Nothing => Schema::Bool(false).into(),
            _ => Schema::Bool(false).into(),
        }
    }
}

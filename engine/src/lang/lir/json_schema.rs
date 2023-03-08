use std::sync::Arc;
use schemars::schema::{InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec};
use serde_json::{json, Value};
use crate::core::{FunctionInput};
use crate::lang::lir::{InnerPattern, Pattern};
use crate::lang::{PrimordialPattern, ValuePattern};
use crate::runtime::World;

impl Pattern {
    pub fn as_json_schema(&self, world: &World, bindings: &Vec<Arc<Pattern>>) -> Schema {
        match &self.inner {
            InnerPattern::Anything => {
                Schema::Bool(true)
            }
            InnerPattern::Primordial(inner) => {
                match inner {
                    PrimordialPattern::Integer => {
                        Schema::Object(
                            SchemaObject {
                                instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                                ..Default::default()
                            }
                        )
                    }
                    PrimordialPattern::Decimal => {
                        Schema::Object(
                            SchemaObject {
                                instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                                ..Default::default()
                            }
                        )
                    }
                    PrimordialPattern::Boolean => {
                        Schema::Object(
                            SchemaObject {
                                instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Boolean))),
                                ..Default::default()
                            }
                        )
                    }
                    PrimordialPattern::String => {
                        Schema::Object(
                            SchemaObject {
                                instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                                ..Default::default()
                            }
                        )
                    }
                    PrimordialPattern::Function(_, _name, func) => {
                        match &func.input(bindings) {
                            FunctionInput::Anything => {
                                Schema::Bool(true)
                            }
                            FunctionInput::String => {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                                        ..Default::default()
                                    }
                                )
                            }
                            FunctionInput::Boolean => {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Boolean))),
                                        ..Default::default()
                                    }
                                )
                            }
                            FunctionInput::Integer => {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                                        ..Default::default()
                                    }
                                )
                            }
                            FunctionInput::Decimal => {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                                        ..Default::default()
                                    }
                                )
                            }
                            FunctionInput::Pattern(_pattern) => {
                                Schema::Bool(false)
                            }
                        }
                    }
                }
            }
            //InnerPattern::Bound(_, _) => {}
            InnerPattern::Ref(_, slot, bindings) => {
                if let Some(inner) = world.get_by_slot(*slot) {
                    if inner.parameters.is_empty() {
                        if let Some(name) = &inner.name {
                            let type_name = name.as_type_str();
                            if type_name == "string" {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                                        ..Default::default()
                                    }
                                )
                            } else if type_name == "boolean" {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Boolean))),
                                        ..Default::default()
                                    }
                                )
                            } else if type_name == "decimal" {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Number))),
                                        ..Default::default()
                                    }
                                )
                            } else if type_name == "integer" {
                                Schema::Object(
                                    SchemaObject {
                                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::String))),
                                        ..Default::default()
                                    }
                                )
                            } else {
                                Schema::Object(
                                    SchemaObject::new_ref(name.as_type_str())
                                )
                            }
                        } else {
                            inner.as_json_schema(world, bindings)
                        }
                    } else {
                        inner.as_json_schema(world, bindings)
                    }
                } else {
                    Schema::Bool(false)
                }
            }
            //InnerPattern::Deref(_) => {}
            //InnerPattern::Argument(_) => {}
            InnerPattern::Const(val) => {
                match val {
                    ValuePattern::Null => {
                        Schema::Object(
                            SchemaObject {
                                const_value: Some(Value::Null),
                                ..Default::default()
                            }
                        )
                    }
                    ValuePattern::String(inner) => {
                        Schema::Object(
                            SchemaObject {
                                const_value: Some(json!(inner)),
                                ..Default::default()
                            }
                        )
                    }
                    ValuePattern::Integer(inner) => {
                        Schema::Object(
                            SchemaObject {
                                const_value: Some(json!(inner)),
                                ..Default::default()
                            }
                        )
                    }
                    ValuePattern::Decimal(inner) => {
                        Schema::Object(
                            SchemaObject {
                                const_value: Some(json!(inner)),
                                ..Default::default()
                            }
                        )
                    }
                    ValuePattern::Boolean(inner) => {
                        Schema::Object(
                            SchemaObject {
                                const_value: Some(json!(*inner)),
                                ..Default::default()
                            }
                        )
                    }
                    ValuePattern::List(_) => {
                        todo!("impl list")
                    }
                    ValuePattern::Octets(_) => {
                        todo!("impl octets")
                    }
                }
            }
            InnerPattern::Object(obj) => {
                let mut validation = ObjectValidation::default();
                for field in obj.fields() {
                    validation.properties.insert(field.name(), field.ty().as_json_schema(world, bindings));
                }

                Schema::Object(
                    SchemaObject {
                        instance_type: Some(SingleOrVec::Single(Box::new(InstanceType::Object))),
                        object: Some(Box::new(validation)),
                       ..Default::default()
                    }
                )
            }
            //InnerPattern::Expr(_) => {}
            //InnerPattern::List(_) => {}
            InnerPattern::Nothing => {
                Schema::Bool(false)
            }
            _ => {
                Schema::Bool(false)
            }
        }
    }
}
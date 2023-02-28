use crate::value::{Object, RuntimeValue};
use serde::{ser, Serialize};
use std::fmt::Display;

use std::sync::Arc;

pub struct Serializer {}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("missing key")]
    MissingKey,
    #[error("{0}")]
    Custom(String),
}

impl ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: Display,
    {
        todo!()
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = RuntimeValue;
    type Error = Error;
    type SerializeSeq = SerializeList<'a>;
    type SerializeTuple = SerializeList<'a>;
    type SerializeTupleStruct = SerializeList<'a>;
    type SerializeTupleVariant = SerializeList<'a>;
    type SerializeMap = SerializeMap<'a>;
    type SerializeStruct = SerializeMap<'a>;
    type SerializeStructVariant = SerializeMap<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Boolean(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v as _))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v as _))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v as _))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v as _))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v as _))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Integer(v as _))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        if v < i64::MAX as u64 {
            Ok(RuntimeValue::Integer(v as _))
        } else {
            Ok(RuntimeValue::Decimal(v as _))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Decimal(v as _))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Decimal(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::String(v.into()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Octets(v.to_vec()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Null)
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(&mut *self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Ok(RuntimeValue::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize,
    {
        let mut result = Object::new();
        result.set(variant, value.serialize(self)?);
        Ok(RuntimeValue::Object(result))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SerializeList::new(&mut *self, len))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(SerializeList::with_parent(self, variant, Some(len)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(SerializeMap::new(self))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(SerializeMap::with_parent(self, variant))
    }
}

pub struct SerializeList<'a> {
    ser: &'a mut Serializer,
    output: Vec<Arc<RuntimeValue>>,
    parent: Option<&'a str>,
}

impl<'a> SerializeList<'a> {
    fn new(ser: &'a mut Serializer, capacity: Option<usize>) -> Self {
        Self {
            ser,
            output: Vec::with_capacity(capacity.unwrap_or(16)),
            parent: None,
        }
    }

    fn with_parent(ser: &'a mut Serializer, parent: &'a str, capacity: Option<usize>) -> Self {
        Self {
            ser,
            output: Vec::with_capacity(capacity.unwrap_or(16)),
            parent: Some(parent),
        }
    }

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize,
    {
        self.output.push(Arc::new(value.serialize(&mut *self.ser)?));
        Ok(())
    }

    fn end(self) -> Result<RuntimeValue, Error> {
        let value = RuntimeValue::List(self.output);
        Ok(wrapped(value, self.parent))
    }
}

fn wrapped(value: RuntimeValue, parent: Option<&str>) -> RuntimeValue {
    match parent {
        Some(field) => {
            let mut parent = Object::new();
            parent.set(field, value);
            RuntimeValue::Object(parent)
        }
        None => value,
    }
}

impl<'a> ser::SerializeSeq for SerializeList<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        SerializeList::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeList::end(self)
    }
}

impl<'a> ser::SerializeTuple for SerializeList<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        SerializeList::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeList::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for SerializeList<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        SerializeList::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeList::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for SerializeList<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        SerializeList::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeList::end(self)
    }
}

pub struct SerializeMap<'a> {
    ser: &'a mut Serializer,
    field: Option<&'a str>,
    output: Object,
    key: Option<String>,
}

impl<'a> SerializeMap<'a> {
    fn new(ser: &'a mut Serializer) -> Self {
        Self {
            ser,
            output: Default::default(),
            key: None,
            field: None,
        }
    }

    fn with_parent(ser: &'a mut Serializer, field: &'a str) -> Self {
        Self {
            ser,
            output: Default::default(),
            key: None,
            field: Some(field),
        }
    }

    fn end(self) -> Result<RuntimeValue, Error> {
        let value = RuntimeValue::Object(self.output);
        Ok(wrapped(value, self.field))
    }
}

impl<'a> ser::SerializeMap for SerializeMap<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.key = Some(key.serialize(&mut *self.ser)?.to_string());
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.output.set(
            self.key.take().ok_or(Error::MissingKey)?,
            value.serialize(&mut *self.ser)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

impl<'a> ser::SerializeStruct for SerializeMap<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.output.set(key, value.serialize(&mut *self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

impl<'a> ser::SerializeStructVariant for SerializeMap<'a> {
    type Ok = RuntimeValue;
    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: Serialize,
    {
        self.output.set(key, value.serialize(&mut *self.ser)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::value::test::assert_yaml;
    use crate::value::{Object, RuntimeValue};

    fn to_value<S: Serialize>(value: &S) -> Result<RuntimeValue, Error> {
        let mut serializer = Serializer {};

        value.serialize(&mut serializer)
    }

    #[test]
    fn test_ser_none() {
        assert_eq!(Ok(RuntimeValue::Null), to_value(&None::<()>));
    }

    #[test]
    fn test_ser_more_complex() {
        #[derive(serde::Serialize)]
        struct Example {
            foo: String,
            bar: bool,
            baz: Option<Baz>,
        }
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Baz {
            n: i64,
            another_field: Vec<More>,
        }
        #[derive(serde::Serialize)]
        struct More(f64);
        assert_eq!(
            Ok(RuntimeValue::Object({
                let mut o = Object::new();
                o.set("foo", "Hello");
                o.set("bar", true);
                o.set("baz", {
                    let mut o = Object::new();
                    o.set("n", 42i64);
                    o.set(
                        "anotherField",
                        RuntimeValue::with_iter(vec![1f64, 2f64, 3f64]),
                    );
                    o
                });
                o
            })),
            to_value(&Example {
                foo: "Hello".to_string(),
                bar: true,
                baz: Some(Baz {
                    n: 42,
                    another_field: vec![More(1f64), More(2f64), More(3f64)],
                })
            })
        );
    }

    fn number_ok<S: Serialize>(s: S, expected: RuntimeValue) {
        assert_eq!(Ok(expected), to_value(&s));
    }

    #[test]
    fn test_numbers() {
        number_ok(0i32, RuntimeValue::Integer(0));
        number_ok(5i32, RuntimeValue::Integer(5));

        number_ok(i64::MIN, RuntimeValue::Integer(i64::MIN));
        number_ok(i64::MAX, RuntimeValue::Integer(i64::MAX));

        number_ok(u64::MIN, RuntimeValue::Integer(0));
        number_ok(u64::MAX, RuntimeValue::Decimal(u64::MAX as _));

        number_ok(f64::MIN, RuntimeValue::Decimal(f64::MIN));
        number_ok(f64::MAX, RuntimeValue::Decimal(f64::MAX));
    }

    #[test]
    fn test_yaml() {
        assert_yaml(|y| to_value(&serde_yaml::from_str::<serde_yaml::Value>(y).unwrap()));
    }
}

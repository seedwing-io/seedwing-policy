use js_sys::{Array, Object};
use monaco::sys::MarkerSeverity;
use ropey::Rope;
use wasm_bindgen::JsValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerData {
    pub message: String,
    pub severity: MarkerSeverity,
    pub range: std::ops::Range<Position>,
    pub source: Option<String>,
}

impl MarkerData {
    pub fn new<M, R, P>(message: M, severity: MarkerSeverity, range: R) -> Self
    where
        M: Into<String>,
        R: Into<std::ops::Range<P>>,
        P: Into<Position>,
    {
        let range = range.into();
        Self {
            message: message.into(),
            severity,
            range: std::ops::Range {
                start: range.start.into(),
                end: range.end.into(),
            },
            source: None,
        }
    }

    pub fn to_object(&self) -> Result<Object, JsValue> {
        let result = Object::new();
        for (k, v) in [
            ("message", JsValue::from(&self.message)),
            ("severity", (self.severity as i32).into()),
            ("startLineNumber", self.range.start.line.into()),
            ("startColumn", self.range.start.position.into()),
            ("endLineNumber", self.range.end.line.into()),
            ("endColumn", self.range.end.position.into()),
        ] {
            js_sys::Reflect::set(&result, &JsValue::from(k), &v)?;
        }

        if let Some(source) = &self.source {
            js_sys::Reflect::set(&result, &JsValue::from("source"), &JsValue::from(source))?;
        }

        Ok(result)
    }

    pub fn array(markers: &[MarkerData]) -> Array {
        let result = Array::new();

        for marker in markers {
            result.push(&marker.to_object().unwrap());
        }

        result
    }
}

impl TryFrom<MarkerData> for Object {
    type Error = JsValue;

    fn try_from(value: MarkerData) -> Result<Self, Self::Error> {
        Ok(value.to_object()?)
    }
}

impl TryFrom<MarkerData> for JsValue {
    type Error = JsValue;

    fn try_from(value: MarkerData) -> Result<Self, Self::Error> {
        let value: Object = value.try_into()?;
        Ok(value.into())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub position: usize,
}

impl From<(usize, usize)> for Position {
    fn from((line, position): (usize, usize)) -> Self {
        Self { line, position }
    }
}

pub struct ByteRange<'r>(pub &'r Rope, pub std::ops::Range<usize>);

impl TryFrom<ByteRange<'_>> for std::ops::Range<Position> {
    type Error = ropey::Error;

    fn try_from(value: ByteRange<'_>) -> Result<Self, Self::Error> {
        Ok(Self {
            start: (value.0, value.1.start).try_into()?,
            end: (value.0, value.1.end).try_into()?,
        })
    }
}

impl TryFrom<(&Rope, usize)> for Position {
    type Error = ropey::Error;

    fn try_from((rope, position): (&Rope, usize)) -> Result<Self, Self::Error> {
        let line = rope.try_byte_to_line(position)?;
        let position = position - rope.try_line_to_byte(line)?;
        Ok(Self {
            line: line + 1,
            position: position + 1,
        })
    }
}

pub mod sources;

use crate::core::Function;
use crate::lang::hir::MemberQualifier;
use crate::lang::lir::{Bindings, Field, ObjectType, Type};
use crate::lang::mir::TypeHandle;
use crate::lang::parser::{
    CompilationUnit, Located, ParserError, ParserInput, PolicyParser, SourceLocation, SourceSpan,
};
use crate::lang::{hir, lir};
use crate::package::Package;
use crate::runtime::cache::SourceCache;
use crate::runtime::rationale::Rationale;
use crate::value::RuntimeValue;
use ariadne::Cache;
use chumsky::{Error, Stream};
use serde::{Serialize, Serializer};
use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::future::{ready, Future};
use std::mem;
use std::ops::Deref;
use std::path::PathBuf;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::ready;

pub mod cache;
pub mod rationale;

#[derive(Clone, Debug, thiserror::Error)]
pub enum BuildError {
    #[error("type ({2}) not found (@ {0}:{1:?})")]
    TypeNotFound(SourceLocation, SourceSpan, String),
    #[error("failed to parse (@ {0}): {1}")]
    Parser(SourceLocation, ParserError),
    #[error("argument mismatch (@ {0}:{1:?})")]
    ArgumentMismatch(SourceLocation, SourceSpan),
}

impl BuildError {
    pub fn source_location(&self) -> SourceLocation {
        match self {
            BuildError::TypeNotFound(loc, _, _) => loc.clone(),
            BuildError::Parser(loc, _) => loc.clone(),
            BuildError::ArgumentMismatch(loc, _) => loc.clone(),
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            BuildError::TypeNotFound(_, span, _) => span.clone(),
            BuildError::Parser(_, err) => err.span(),
            BuildError::ArgumentMismatch(_, span) => span.clone(),
        }
    }
}

impl From<(SourceLocation, ParserError)> for BuildError {
    fn from(inner: (SourceLocation, ParserError)) -> Self {
        Self::Parser(inner.0, inner.1)
    }
}

#[derive(Debug, Clone)]
pub enum Output {
    None,
    Identity,
    Transform(Rc<RuntimeValue>),
}

impl Output {
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    input: Option<Rc<RuntimeValue>>,
    ty: Arc<Type>,
    rationale: Rationale,
    output: Output,
}

impl EvaluationResult {
    pub fn new(
        input: Option<Rc<RuntimeValue>>,
        ty: Arc<Type>,
        rationale: Rationale,
        output: Output,
    ) -> Self {
        Self {
            input,
            ty,
            rationale,
            output,
        }
    }

    pub fn satisfied(&self) -> bool {
        self.rationale.satisfied()
    }

    pub fn ty(&self) -> Arc<Type> {
        self.ty.clone()
    }

    pub fn input(&self) -> Option<Rc<RuntimeValue>> {
        self.input.clone()
    }

    pub fn rationale(&self) -> &Rationale {
        &self.rationale
    }

    pub fn output(&self) -> Option<Rc<RuntimeValue>> {
        match &self.output {
            Output::None => None,
            Output::Identity => self.input.clone(),
            Output::Transform(inner) => Some(inner.clone()),
        }
    }

    pub fn raw_output(&self) -> &Output {
        &self.output
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("lock")]
    Lock,
    #[error("invalid state")]
    InvalidState,
    #[error("no such type: {0}")]
    NoSuchType(TypeName),
    #[error("no such type slot: {0}")]
    NoSuchTypeSlot(usize),
    #[error("error parsing JSON file {0}: {1}")]
    JsonError(PathBuf, serde_json::Error),
    #[error("error reading file: {0}")]
    FileUnreadable(PathBuf),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::{Directory, Ephemeral};
    use crate::value::RationaleResult;
    use serde_json::json;
    use std::default::Default;
    use std::env;
    use std::iter::once;

    #[actix_rt::test]
    async fn ephemeral_sources() {
        let src = Ephemeral::new("foo::bar", "pattern bob");

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.finish().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn link_test_data() {
        let src = Directory::new(env::current_dir().unwrap().join("test-data"));

        let mut builder = Builder::new();

        let result = builder.build(src.iter());

        let result = builder.finish().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn evaluate_function() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
            // Single-line comment, yay
            pattern signed-thing = {
                digest: sigstore::SHA256(
                    list::Any<{
                        apiVersion: "0.0.1",
                        spec: {
                            signature: {
                                publicKey: {
                                    content: base64::Base64(
                                        x509::PEM( list::Any<{
                                            version: 2,
                                            extensions: list::Any<{
                                                subjectAlternativeName: list::Any<{
                                                    rfc822: "bob@mcwhirter.org",
                                                }>
                                            }>
                                        }> )
                                    )
                                }
                            }
                        }
                    }>
                )
            }
        "#,
        );

        let mut builder = Builder::new();

        let result = builder.build(src.iter());
        println!("---> {:?}", result);
        let runtime = builder.finish().await.unwrap();

        let value = json!(
            {
                "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
            }
        );

        let result = runtime.evaluate("foo::bar::signed-thing", value).await;

        assert!(result.unwrap().satisfied())
        //assert!(matches!(result, Ok(RationaleResult::Same(_)),))
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_literals() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        pattern named<name> = {
            name: name
        }

        pattern jim = named<"Jim">
        pattern bob = named<"Bob">

        pattern folks = jim || bob

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        let good_bob = json!(
            {
                "name": "Bob",
                "age": 52,
            }
        );

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 52,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_types() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
                pattern named<name> = {
                    name: name
                }

                pattern jim = named<integer>
                pattern bob = named<"Bob">

                pattern folks = jim || bob

                "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 52,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let src = Ephemeral::new(
            "foo::bar",
            r#"
        pattern bob = {
            name: "Bob",
            age: $(self > 48),
        }

        pattern jim = {
            name: "Jim",
            age: $(self > 52),
        }

        pattern folks = bob || jim

        "#,
        );

        let mut builder = Builder::new();
        let result = builder.build(src.iter());
        let runtime = builder.finish().await.unwrap();

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 49,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());

        assert!(!runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Jim",
                        "age": 49,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());

        assert!(!runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Bob",
                        "age": 42,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());

        assert!(runtime
            .evaluate(
                "foo::bar::folks",
                json!(
                    {
                        "name": "Jim",
                        "age": 53,
                    }
                ),
            )
            .await
            .unwrap()
            .satisfied());
    }
}

#[derive(Clone, Debug)]
pub struct World {
    types: HashMap<TypeName, usize>,
    type_slots: Vec<Arc<Type>>,
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
            type_slots: Default::default(),
        }
    }

    pub fn get_by_slot(&self, slot: usize) -> Option<Arc<Type>> {
        if slot < self.type_slots.len() {
            Some(self.type_slots[slot].clone())
        } else {
            None
        }
    }

    pub(crate) fn add(&mut self, path: TypeName, handle: Arc<TypeHandle>) {
        let ty = handle.ty();
        let name = handle.name();
        let parameters = handle.parameters().iter().map(|e| e.inner()).collect();
        let converted = lir::convert(name, handle.documentation(), parameters, &ty);
        self.type_slots.push(converted);
        self.types.insert(path, self.type_slots.len() - 1);
    }

    pub async fn evaluate<P: Into<String>, V: Into<RuntimeValue>>(
        &self,
        path: P,
        value: V,
    ) -> Result<EvaluationResult, RuntimeError> {
        let value = Rc::new(value.into());
        let path = TypeName::from(path.into());
        let slot = self.types.get(&path);
        if let Some(slot) = slot {
            let ty = self.type_slots[*slot].clone();
            let bindings = Bindings::default();
            ty.evaluate(value.clone(), &bindings, self).await
        } else {
            Err(RuntimeError::NoSuchType(path))
        }
    }

    pub fn get<S: Into<String>>(&self, name: S) -> Option<Component> {
        let name = name.into();
        let path = TypeName::from(name);

        if let Some(slot) = self.types.get(&path) {
            let ty = self.type_slots[*slot].clone();
            return Some(Component::Type(ty));
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

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct TypeName {
    package: Option<PackagePath>,
    name: String,
}

impl Serialize for TypeName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_type_str().serialize(serializer)
    }
}

impl Display for TypeName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_type_str())
    }
}

impl TypeName {
    pub fn new(package: Option<PackagePath>, name: String) -> Self {
        Self { package, name }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn is_qualified(&self) -> bool {
        self.package.is_some()
    }

    pub fn as_type_str(&self) -> String {
        let mut fq = String::new();
        if let Some(package) = &self.package {
            fq.push_str(&package.as_package_str());
            fq.push_str("::");
        }

        fq.push_str(&self.name);

        fq
    }

    pub fn segments(&self) -> Vec<String> {
        let mut segments = Vec::new();
        if let Some(package) = &self.package {
            segments.extend_from_slice(&*package.segments())
        }

        segments.push(self.name.clone());
        segments
    }
}

impl From<String> for TypeName {
    fn from(path: String) -> Self {
        let mut segments = path.split("::").map(|e| e.into()).collect::<Vec<String>>();
        if segments.is_empty() {
            Self::new(None, "".into())
        } else {
            let tail = segments.pop().unwrap();
            if segments.is_empty() {
                Self {
                    package: None,
                    name: tail,
                }
            } else {
                let package = Some(segments.into());
                Self {
                    package,
                    name: tail,
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PackageName(pub(crate) String);

impl PackageName {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

impl Deref for PackageName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PackagePath {
    is_absolute: bool,
    path: Vec<Located<PackageName>>,
}

impl From<&str> for PackagePath {
    fn from(segments: &str) -> Self {
        let segments: Vec<String> = segments.split("::").map(|e| e.into()).collect();
        segments.into()
    }
}

impl From<String> for PackagePath {
    fn from(mut segments: String) -> Self {
        if let Some(stripped) = segments.strip_suffix("::") {
            segments = stripped.into();
        }

        let segments: Vec<String> = segments.split("::").map(|e| e.into()).collect();
        segments.into()
    }
}

impl From<Vec<String>> for PackagePath {
    fn from(mut segments: Vec<String>) -> Self {
        let first = segments.get(0).unwrap();
        let is_absolute = first.is_empty();
        if is_absolute {
            segments = segments[1..].to_vec()
        }

        Self {
            is_absolute: true,
            path: segments
                .iter()
                .map(|e| Located::new(PackageName(e.clone()), 0..0))
                .collect(),
        }
    }
}

impl From<Vec<Located<PackageName>>> for PackagePath {
    fn from(mut segments: Vec<Located<PackageName>>) -> Self {
        Self {
            is_absolute: true,
            path: segments,
        }
    }
}

impl PackagePath {
    pub fn from_parts(segments: Vec<&str>) -> Self {
        Self {
            is_absolute: true,
            path: segments
                .iter()
                .map(|e| Located::new(PackageName(String::from(*e)), 0..0))
                .collect(),
        }
    }

    pub fn is_absolute(&self) -> bool {
        self.is_absolute
    }

    pub fn is_qualified(&self) -> bool {
        self.path.len() > 1
    }

    pub fn type_name(&self, name: String) -> TypeName {
        if self.path.is_empty() {
            TypeName::new(None, name)
        } else {
            TypeName::new(Some(self.clone()), name)
        }
    }

    pub fn as_package_str(&self) -> String {
        let mut fq = String::new();

        fq.push_str(
            &self
                .path
                .iter()
                .map(|e| e.inner().0)
                .collect::<Vec<String>>()
                .join("::"),
        );

        fq
    }

    pub fn path(&self) -> &Vec<Located<PackageName>> {
        &self.path
    }

    pub fn segments(&self) -> Vec<String> {
        self.path.iter().map(|e| e.0.clone()).collect()
    }
}

impl From<SourceLocation> for PackagePath {
    fn from(src: SourceLocation) -> Self {
        let name = src.name().replace('/', "::");
        let segments = name
            .split("::")
            .map(|segment| Located::new(PackageName(segment.into()), 0..0))
            .collect();

        Self {
            is_absolute: true,
            path: segments,
        }
    }
}

#[derive(Debug)]
pub enum Component {
    Module(ModuleHandle),
    Type(Arc<Type>),
}

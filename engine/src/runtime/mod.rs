//! Policy evaluation runtime.
//!
//! All policies are parsed and compiled into a `World` used to evaluate policy decisions for different inputs.
use crate::runtime::cache::SourceCache;
use ariadne::{Label, Report, ReportKind};
use chumsky::error::SimpleReason;
use core::future::Future;
use core::pin::Pin;
use std::time::Instant;

use std::io;

use crate::lang::lir;
use crate::lang::lir::Bindings;
pub use crate::lang::lir::Pattern;
use crate::lang::mir::PatternHandle;
use crate::lang::parser::{Located, ParserError, SourceLocation, SourceSpan};

use crate::runtime::rationale::Rationale;
use crate::value::RuntimeValue;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

use std::ops::Deref;
use std::path::PathBuf;

use std::sync::Arc;

use std::time::Duration;

use config::EvalConfig;
#[cfg(feature = "monitor")]
use {monitor::dispatcher::Monitor, tokio::sync::Mutex};

pub mod cache;
pub mod config;
pub mod metadata;
pub mod monitor;
pub mod rationale;
pub mod response;
pub mod sources;
pub mod statistics;

pub use crate::core::Example;
use crate::runtime::metadata::{PackageMetadata, PatternMetadata, ToMetadata};
pub use response::Response;

#[derive(Clone, Debug, thiserror::Error)]
pub enum BuildError {
    #[error("type ({2}) not found (@ {0}:{1:?})")]
    PatternNotFound(SourceLocation, SourceSpan, String),
    #[error("failed to parse (@ {0}): {1}")]
    Parser(SourceLocation, ParserError),
    #[error("argument mismatch (@ {0}:{1:?})")]
    ArgumentMismatch(SourceLocation, SourceSpan),
}

impl BuildError {
    pub fn source_location(&self) -> SourceLocation {
        match self {
            BuildError::PatternNotFound(loc, _, _) => loc.clone(),
            BuildError::Parser(loc, _) => loc.clone(),
            BuildError::ArgumentMismatch(loc, _) => loc.clone(),
        }
    }

    pub fn span(&self) -> SourceSpan {
        match self {
            BuildError::PatternNotFound(_, span, _) => span.clone(),
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

/// Provides readable error reports when building policies.
pub struct ErrorPrinter<'c> {
    cache: &'c SourceCache,
}

impl<'c> ErrorPrinter<'c> {
    /// Create a new printer instance.
    pub fn new(cache: &'c SourceCache) -> Self {
        Self { cache }
    }

    /// Write errors in a pretty format that can be used to locate the source of the error.
    pub fn write_to<W: io::Write>(&self, errors: &[BuildError], mut w: &mut W) {
        for error in errors {
            let source_id = error.source_location();
            let span = error.span();
            let full_span = (source_id.clone(), error.span());
            let report = Report::<(SourceLocation, SourceSpan)>::build(
                ReportKind::Error,
                source_id.clone(),
                span.start,
            )
            .with_label(Label::new(full_span).with_message(match error {
                BuildError::ArgumentMismatch(_, _) => "argument mismatch".to_string(),
                BuildError::PatternNotFound(_, _, name) => {
                    format!("pattern not found: {name}")
                }
                BuildError::Parser(_, inner) => match inner.reason() {
                    SimpleReason::Unexpected => {
                        format!("unexpected character found {}", inner.found().unwrap())
                    }
                    SimpleReason::Unclosed { span: _, delimiter } => {
                        format!("unclosed delimiter {delimiter}")
                    }
                    SimpleReason::Custom(inner) => inner.clone(),
                },
            }))
            .finish();

            let _ = report.write(self.cache, &mut w);
        }
    }

    /// Write errors to standard out.
    pub fn display(&self, errors: &[BuildError]) {
        self.write_to(errors, &mut std::io::stdout().lock())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Output {
    None,
    Identity,
    Transform(Arc<RuntimeValue>),
}

impl Output {
    pub fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    input: Arc<RuntimeValue>,
    ty: Arc<Pattern>,
    rationale: Rationale,
    output: Output,
    trace: Option<TraceResult>,
}

impl EvaluationResult {
    pub fn new(
        input: Arc<RuntimeValue>,
        ty: Arc<Pattern>,
        rationale: Rationale,
        output: Output,
    ) -> Self {
        Self {
            input,
            ty,
            rationale,
            output,
            trace: None,
        }
    }

    pub fn satisfied(&self) -> bool {
        self.rationale.satisfied()
    }

    pub fn ty(&self) -> Arc<Pattern> {
        self.ty.clone()
    }

    pub fn input(&self) -> Arc<RuntimeValue> {
        self.input.clone()
    }

    pub fn rationale(&self) -> &Rationale {
        &self.rationale
    }

    pub fn output(&self) -> Option<Arc<RuntimeValue>> {
        match &self.output {
            Output::None => None,
            Output::Identity => Some(self.input.clone()),
            Output::Transform(inner) => Some(inner.clone()),
        }
    }

    pub fn raw_output(&self) -> &Output {
        &self.output
    }

    pub fn trace(&self) -> Option<TraceResult> {
        self.trace
    }

    #[allow(dead_code)]
    pub(crate) fn with_trace_result(&mut self, trace: TraceResult) {
        self.trace.replace(trace);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("invalid state")]
    InvalidState,
    #[error("no such pattern: {0}")]
    NoSuchPattern(PatternName),
    #[error("no such type slot: {0}")]
    NoSuchPatternSlot(usize),
    #[error("error parsing JSON file {0}: {1}")]
    JsonError(PathBuf, serde_json::Error),
    #[error("error parsing YAML file {0}: {1}")]
    YamlError(PathBuf, serde_yaml::Error),
    #[error("error reading file: {0}")]
    FileUnreadable(PathBuf),
}

#[derive(Clone, Debug)]
pub struct World {
    config: EvalConfig,
    types: HashMap<PatternName, usize>,
    type_slots: Vec<Arc<Pattern>>,
}

impl World {
    pub fn new(config: EvalConfig) -> Self {
        Self {
            config,
            types: Default::default(),
            type_slots: Default::default(),
        }
    }

    pub fn all(&self) -> Vec<(PatternName, Arc<Pattern>)> {
        let mut all = Vec::new();
        for (k, slot) in &self.types {
            all.push((k.clone(), self.type_slots[*slot].clone()))
        }
        all
    }

    pub fn get_by_slot(&self, slot: usize) -> Option<Arc<Pattern>> {
        if slot < self.type_slots.len() {
            Some(self.type_slots[slot].clone())
        } else {
            None
        }
    }

    pub(crate) fn add(&mut self, path: PatternName, handle: Arc<PatternHandle>) {
        let ty = handle.ty();
        let name = handle.name();
        let parameters = handle.parameters().iter().map(|e| e.inner()).collect();
        let converted = lir::convert(
            name,
            handle.metadata().clone(),
            handle.examples(),
            parameters,
            &ty,
        );
        self.type_slots.push(converted);
        self.types.insert(path, self.type_slots.len() - 1);
    }

    pub async fn evaluate<P: Into<String>, V: Into<RuntimeValue>>(
        &self,
        path: P,
        value: V,
        mut ctx: EvalContext,
    ) -> Result<EvaluationResult, RuntimeError> {
        ctx.merge_config(&self.config);
        let value = Arc::new(value.into());
        let path = PatternName::from(path.into());
        let slot = self.types.get(&path);
        if let Some(slot) = slot {
            let ty = self.type_slots[*slot].clone();
            let bindings = Bindings::default();
            ty.evaluate(value.clone(), &ctx, &bindings, self).await
        } else {
            Err(RuntimeError::NoSuchPattern(path))
        }
    }

    pub fn get_package_meta<S: Into<PackagePath>>(&self, name: S) -> Option<PackageMetadata> {
        let name = name.into();
        let mut meta = PackageMetadata::new(name.clone());
        let path = name.as_package_str();

        for (name, slot) in self.types.iter() {
            let name = name.as_type_str();
            if let Some(relative_name) = name.strip_prefix(&path) {
                let relative_name = relative_name.strip_prefix("::").unwrap_or(relative_name);
                let parts: Vec<&str> = relative_name.split("::").collect();
                if parts.len() == 1 {
                    let pattern = self.type_slots[*slot].clone();
                    if let Ok(pattern) = pattern.to_meta(self) {
                        meta.add_pattern(pattern);
                    }
                } else {
                    meta.add_subpackage(PackageName::new(parts[0].into()));
                }
            }
        }

        if meta.packages.is_empty() && meta.patterns.is_empty() {
            None
        } else {
            Some(meta.sort())
        }
    }

    pub fn get_pattern_meta<S: Into<PatternName>>(&self, name: S) -> Option<PatternMetadata> {
        let name = name.into();
        if let Some(slot) = self.types.get(&name) {
            let pattern = self.type_slots[*slot].clone();
            pattern.to_meta(self).ok()
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct PatternName {
    pub package: Option<PackagePath>,
    pub name: String,
}

impl Serialize for PatternName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_type_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PatternName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        Ok(s.into())
    }
}

impl Display for PatternName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_type_str())
    }
}

impl PatternName {
    pub fn new(package: Option<PackagePath>, name: String) -> Self {
        Self { package, name }
    }

    pub fn name(&self) -> &str {
        &self.name
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
            segments.extend_from_slice(&package.segments())
        }

        segments.push(self.name.clone());
        segments
    }

    pub fn package(&self) -> Option<PackagePath> {
        self.package.clone()
    }
}

impl From<String> for PatternName {
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
    fn from(segments: Vec<Located<PackageName>>) -> Self {
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

    pub fn type_name(&self, name: String) -> PatternName {
        if self.path.is_empty() {
            PatternName::new(None, name)
        } else {
            PatternName::new(Some(self.clone()), name)
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

/// Tracing information such as evaluation time.
#[derive(Debug, Clone, Copy)]
pub struct TraceResult {
    pub duration: Duration,
}

impl TraceResult {
    pub fn new(duration: Duration) -> Self {
        Self { duration }
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            trace: TraceConfig::Disabled,
            config: EvalConfig::default(),
        }
    }
}

#[derive(Debug)]
pub struct EvalContext {
    trace: TraceConfig,
    config: EvalConfig,
}

impl EvalContext {
    pub fn new(trace: TraceConfig, config: EvalConfig) -> Self {
        Self { trace, config }
    }

    pub fn new_with_config(config: EvalConfig) -> Self {
        Self {
            trace: TraceConfig::Disabled,
            config,
        }
    }

    pub fn config(&self) -> &EvalConfig {
        &self.config
    }

    pub fn merge_config(&mut self, defaults: &EvalConfig) {
        self.config.merge_defaults(defaults);
    }

    pub fn trace(&self, input: Arc<RuntimeValue>, ty: Arc<Pattern>) -> TraceHandle {
        match &self.trace {
            #[cfg(feature = "monitor")]
            TraceConfig::Enabled(_monitor) => TraceHandle {
                context: self,
                ty,
                input,
                start: Some(Instant::now()),
            },
            TraceConfig::Disabled => TraceHandle {
                context: self,
                ty,
                input,
                start: None,
            },
        }
    }

    async fn correlation(&self) -> Option<u64> {
        match &self.trace {
            #[cfg(feature = "monitor")]
            TraceConfig::Enabled(monitor) => Some(monitor.lock().await.init()),
            TraceConfig::Disabled => None,
        }
    }

    #[allow(unused_variables)]
    pub async fn start(&self, correlation: u64, input: Arc<RuntimeValue>, ty: Arc<Pattern>) {
        #[cfg(feature = "monitor")]
        if let TraceConfig::Enabled(monitor) = &self.trace {
            monitor.lock().await.start(correlation, input, ty).await;
        }
    }

    #[allow(unused_variables)]
    async fn complete(
        &self,
        correlation: u64,
        ty: Arc<Pattern>,
        result: &mut Result<EvaluationResult, RuntimeError>,
        elapsed: Option<Duration>,
    ) {
        #[cfg(feature = "monitor")]
        if let TraceConfig::Enabled(monitor) = &self.trace {
            match result {
                Ok(ref mut result) => {
                    if let Some(elapsed) = elapsed {
                        result.with_trace_result(TraceResult { duration: elapsed });
                    }
                    monitor
                        .lock()
                        .await
                        .complete_ok(correlation, ty, result.raw_output().clone(), elapsed)
                        .await
                }
                Err(err) => {
                    monitor
                        .lock()
                        .await
                        .complete_err(correlation, ty, err, elapsed)
                        .await
                }
            }
        }
    }
}

#[derive(Clone)]
pub enum TraceConfig {
    #[cfg(feature = "monitor")]
    Enabled(Arc<Mutex<Monitor>>),
    Disabled,
}

impl Debug for TraceConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "monitor")]
            TraceConfig::Enabled(_) => {
                write!(f, "Trace::Enabled")
            }
            TraceConfig::Disabled => {
                write!(f, "Trace::Disabled")
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TraceHandle<'ctx> {
    context: &'ctx EvalContext,
    ty: Arc<Pattern>,
    input: Arc<RuntimeValue>,
    start: Option<Instant>,
}

impl From<EvaluationResult> for (Rationale, Output) {
    fn from(result: EvaluationResult) -> Self {
        (result.rationale().clone(), result.raw_output().clone())
    }
}

impl<'ctx> TraceHandle<'ctx> {
    pub(crate) fn run<'v>(
        self,
        block: Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>>,
    ) -> Pin<Box<dyn Future<Output = Result<EvaluationResult, RuntimeError>> + 'v>>
    where
        'ctx: 'v,
    {
        if self.start.is_some() {
            Box::pin(async move {
                if let Some(correlation) = self.context.correlation().await {
                    self.context
                        .start(correlation, self.input.clone(), self.ty.clone())
                        .await;
                    let mut result = block.await;
                    let elapsed = self.start.map(|e| e.elapsed());
                    self.context
                        .complete(correlation, self.ty.clone(), &mut result, elapsed)
                        .await;
                    result
                } else {
                    block.await
                }
            })
        } else {
            block
        }
    }
}

#[cfg(test)]
pub mod testutil {
    use crate::data::DirectoryDataSource;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::Ephemeral;
    use crate::runtime::EvalContext;
    use crate::runtime::EvaluationResult;
    use crate::value::RuntimeValue;
    use std::path::{Path, PathBuf};

    pub(crate) async fn test_pattern<V>(pattern: &str, value: V) -> EvaluationResult
    where
        V: Into<RuntimeValue>,
    {
        let src = format!("pattern test-pattern = {pattern}");
        let src = Ephemeral::new("test", src);
        evaluate(src, value).await
    }

    /// This function can be used when there are multiple patterns that are
    /// being tested.
    ///
    /// The pattern to be evaulated must be named `test-pattern`.
    pub(crate) async fn test_patterns<V>(patterns: &str, value: V) -> EvaluationResult
    where
        V: Into<RuntimeValue>,
    {
        let src = Ephemeral::new("test", patterns);
        evaluate(src, value).await
    }

    async fn evaluate<V>(src: Ephemeral, value: V) -> EvaluationResult
    where
        V: Into<RuntimeValue>,
    {
        init_logger();
        let mut builder = Builder::new();
        builder.data(DirectoryDataSource::new(test_data_dir().into()));
        builder.build(src.iter()).unwrap();
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::test-pattern", value, EvalContext::default())
            .await;

        result.unwrap()
    }

    pub(crate) fn test_data_dir() -> PathBuf {
        let cargo_manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        cargo_manifest_dir.join("test-data")
    }

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[macro_export]
    macro_rules! assert_satisfied {
        ( $result:expr ) => {
            assert!(
                $result.satisfied(),
                "{}",
                serde_json::to_string_pretty(&crate::runtime::response::Response::new(&$result))
                    .unwrap()
            );
        };
    }

    #[macro_export]
    macro_rules! assert_not_satisfied {
        ( $result:expr ) => {
            assert!(
                !$result.satisfied(),
                "{}",
                serde_json::to_string_pretty(&crate::runtime::response::Response::new(&$result))
                    .unwrap()
            );
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::{Directory, Ephemeral};

    use serde_json::json;
    use std::env;

    #[actix_rt::test]
    async fn ephemeral_sources() {
        let src = Ephemeral::new("foo::bar", "pattern bob");

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let result = builder.finish().await;
        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn link_test_data() {
        let src = Directory::new(env::current_dir().unwrap().join("test-data"));

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let result = builder.finish().await;

        assert!(matches!(result, Ok(_)));
    }

    #[actix_rt::test]
    async fn evaluate_function() {
        let result = testutil::test_pattern(
            r#"
            {
                digest: sigstore::sha256(
                    list::any<{
                        apiVersion: "0.0.1",
                        spec: {
                            signature: {
                                publicKey: {
                                    content: base64::base64(
                                        x509::pem( list::any<{
                                            version: 2,
                                            extensions: list::any<{
                                                subjectAlternativeName: list::any<{
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
            // Single-line comment, yay
            "#,
            json!({
                "digest": "5dd1e2b50b89874fd086da4b61176167ae9e4b434945325326690c8f604d0408"
            }),
        )
        .await;
        assert!(result.satisfied())
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_literals() {
        let pattern = r#"
            jim || bob
            pattern named<name> = {
                name: name
            }
            pattern jim = named<"Jim">
            pattern bob = named<"Bob">
            "#;
        let bob = json!({
                "name": "Bob",
                "age": 52,
        });
        let frank = json!({
                "name": "Frank",
                "age": 66,
        });
        assert!(testutil::test_pattern(pattern, bob).await.satisfied());
        assert!(!testutil::test_pattern(pattern, frank).await.satisfied());
    }

    #[actix_rt::test]
    async fn evaluate_parameterized_types() {
        let pattern = r#"
            jim || bob
            pattern named<name> = {
                name: name
            }
            pattern jim = named<integer>
            pattern bob = named<"Bob">
            "#;
        let bob = json!({
                "name": "Bob",
                "age": 52,
        });
        let jim = json!({
                "name": 42,
                "age": 69,
        });
        assert!(testutil::test_pattern(pattern, bob).await.satisfied());
        assert!(testutil::test_pattern(pattern, jim).await.satisfied());
    }

    #[actix_rt::test]
    async fn evaluate_matches() {
        let pat = r#"
            bob || jim

            pattern bob = {
                name: "Bob",
                age: $(self > 48),
            }

            pattern jim = {
                name: "Jim",
                age: $(self > 52),
            }
            "#;
        let f = |name, age| json!({"name": name, "age": age});
        assert!(testutil::test_pattern(pat, f("Bob", 49)).await.satisfied());
        assert!(testutil::test_pattern(pat, f("Jim", 53)).await.satisfied());
        assert!(!testutil::test_pattern(pat, f("Jim", 49)).await.satisfied());
        assert!(!testutil::test_pattern(pat, f("Bob", 42)).await.satisfied());
    }
}

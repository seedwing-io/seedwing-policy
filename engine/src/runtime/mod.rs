//! Policy evaluation runtime.
//!
//! All policies are parsed and compiled into a `World` used to evaluate policy decisions for different inputs.
use crate::lang::lir::Bindings;
use crate::lang::parser::{Located, ParserError, SourceLocation, SourceSpan};
use crate::lang::Severity;
use crate::runtime::metadata::{PackageMetadata, PatternMetadata, ToMetadata, WorldLike};
use crate::runtime::{cache::SourceCache, rationale::Rationale};
use crate::value::RuntimeValue;
use ariadne::{Label, Report, ReportKind};
use chumsky::error::SimpleReason;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;

pub use crate::core::Example;
pub use crate::lang::lir::Pattern;
pub use response::Response;

pub mod cache;
pub mod config;
pub mod metadata;
pub mod monitor;
pub mod rationale;
pub mod response;
pub mod sources;
pub mod statistics;

mod utils;
pub use utils::is_default;

mod trace;
use crate::runtime::config::ConfigContext;
pub use trace::*;

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
    /// Output equal to input
    Identity,
    /// Output transformed
    Transform(Arc<RuntimeValue>),
}

#[derive(Debug, Clone)]
pub struct EvaluationResult {
    pub(crate) input: Arc<RuntimeValue>,
    pub(crate) ty: Arc<Pattern>,
    pub(crate) rationale: Arc<Rationale>,
    pub(crate) output: Output,
    pub(crate) trace: Option<TraceResult>,
}

impl EvaluationResult {
    pub fn new(
        input: Arc<RuntimeValue>,
        ty: Arc<Pattern>,
        rationale: Arc<Rationale>,
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

    /// Get both the severity and the reason.
    pub fn outcome(&self) -> (Severity, String) {
        // the evaluated severity
        let mut severity = self.rationale.severity();

        let reason;

        if severity > Severity::None {
            // a possible override severity
            let override_severity = self.ty.metadata().reporting.severity;

            if override_severity > Severity::None {
                severity = override_severity;
            }

            reason = self.ty.metadata().reporting.explanation.clone();
        } else {
            reason = None;
        }

        let reason = reason.unwrap_or_else(|| self.rationale.reason());

        (severity, reason)
    }

    /// Get the severity only.
    pub fn severity(&self) -> Severity {
        // the evaluated severity
        let mut severity = self.rationale.severity();

        if severity > Severity::None {
            // a possible override severity
            let override_severity = self.ty.metadata().reporting.severity;

            if override_severity > Severity::None {
                severity = override_severity;
            }
        }

        severity
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

    pub fn output(&self) -> Arc<RuntimeValue> {
        match &self.output {
            Output::Identity => self.input.clone(),
            Output::Transform(inner) => inner.clone(),
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
    #[error("remote client failed: {0}")]
    RemoteClient(#[from] crate::client::Error),
    #[error("recursion limit reached: {0}")]
    RecursionLimit(usize),
}

#[derive(Clone, Debug)]
pub struct World {
    config: ConfigContext,
    types: HashMap<PatternName, usize>,
    type_slots: Vec<Arc<Pattern>>,

    packages: HashMap<PackagePath, PackageMetadata>,
}

impl WorldLike for World {
    fn get_by_slot(&self, slot: usize) -> Option<Arc<Pattern>> {
        World::get_by_slot(self, slot)
    }
}

impl World {
    pub(crate) fn new(
        config: ConfigContext,
        types: HashMap<PatternName, usize>,
        type_slots: Vec<Arc<Pattern>>,
        packages: HashMap<PackagePath, PackageMetadata>,
    ) -> Self {
        Self {
            config,
            types,
            type_slots,
            packages,
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
        self.type_slots.get(slot).cloned()
    }

    pub async fn evaluate_nocopy<P: Into<String>>(
        &self,
        path: P,
        value: Arc<RuntimeValue>,
        mut ctx: EvalContext,
    ) -> Result<EvaluationResult, RuntimeError> {
        ctx.merge_config(&self.config);
        let path = PatternName::from(path.into());
        let slot = self.types.get(&path);
        let ctx = ExecutionContext::new(&ctx);
        if let Some(slot) = slot {
            let ty = self.type_slots[*slot].clone();
            let bindings = Bindings::default();
            ty.evaluate(value.clone(), ctx, &bindings, self).await
        } else {
            Err(RuntimeError::NoSuchPattern(path))
        }
    }

    pub async fn evaluate<P: Into<String>, V: Into<RuntimeValue>>(
        &self,
        path: P,
        value: V,
        ctx: EvalContext,
    ) -> Result<EvaluationResult, RuntimeError> {
        let value = Arc::new(value.into());
        self.evaluate_nocopy(path, value, ctx).await
    }

    pub fn get_package_meta<S: Into<PackagePath>>(&self, name: S) -> Option<PackageMetadata> {
        self.packages.get(&name.into()).cloned()
    }

    pub fn get_pattern_meta<S: Into<PatternName>>(&self, name: S) -> Option<PatternMetadata> {
        let name = name.into();
        if let Some(slot) = self.types.get(&name) {
            let pattern = &self.type_slots[*slot];
            pattern.to_meta(self).ok()
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug, PartialOrd, Ord)]
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

impl<T> From<T> for PatternName
where
    T: AsRef<str>,
{
    fn from(path: T) -> Self {
        let mut segments = path
            .as_ref()
            .split("::")
            .map(|e| e.into())
            .collect::<Vec<String>>();
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

#[derive(PartialEq, Eq, Hash, Clone, Debug, Ord, PartialOrd)]
pub struct PackageName(pub(crate) String);

impl PackageName {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

impl From<&str> for PackageName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for PackageName {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Deref for PackageName {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug, Ord, PartialOrd)]
pub struct PackagePath {
    pub path: Vec<PackageName>,
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

impl From<&String> for PackagePath {
    fn from(value: &String) -> Self {
        value.as_str().into()
    }
}

impl From<Vec<String>> for PackagePath {
    fn from(mut segments: Vec<String>) -> Self {
        let is_absolute = segments.get(0).map(String::is_empty).unwrap_or_default();
        if is_absolute {
            segments = segments[1..].to_vec()
        }

        Self {
            path: segments.into_iter().map(PackageName).collect(),
        }
    }
}

impl From<Vec<Located<PackageName>>> for PackagePath {
    fn from(segments: Vec<Located<PackageName>>) -> Self {
        Self {
            path: segments.into_iter().map(Located::into_inner).collect(),
        }
    }
}

impl Display for PackagePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, name) in self.path.iter().enumerate() {
            if i > 0 {
                f.write_str("::")?;
            }
            f.write_str(&name.0)?;
        }
        Ok(())
    }
}

impl PackagePath {
    pub const fn root() -> Self {
        Self { path: vec![] }
    }

    pub fn from_parts(segments: Vec<&str>) -> Self {
        Self {
            path: segments
                .into_iter()
                .map(|s| PackageName(s.to_string()))
                .collect(),
        }
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

    /// Get the last element of the path
    pub fn name(&self) -> Option<String> {
        self.path.last().map(|s| s.to_string())
    }

    pub fn as_package_str(&self) -> String {
        self.to_string()
    }

    pub fn path(&self) -> &Vec<PackageName> {
        &self.path
    }

    pub fn segments(&self) -> Vec<String> {
        self.path.iter().map(|e| e.0.clone()).collect()
    }

    /// Get the parent path, if there is one.
    ///
    /// If there is more than one segment, remove the last one. Otherwise, return [`None`].
    pub fn parent(&self) -> Option<PackagePath> {
        let len = self.path.len();
        if len > 1 {
            Some(Self {
                path: self.path[0..(len - 1)].to_vec(),
            })
        } else {
            None
        }
    }

    /// Split into base path and name
    pub fn split_name(&self) -> Option<(PackagePath, PackageName)> {
        if !self.path.is_empty() {
            let len = self.path.len();
            Some((
                Self {
                    path: self.path[0..(len - 1)].to_vec(),
                },
                self.path[len - 1].clone(),
            ))
        } else {
            None
        }
    }

    pub fn join(&self, name: impl Into<PackageName>) -> Self {
        let mut path = self.path.clone();
        path.push(name.into());
        Self { path }
    }
}

impl From<SourceLocation> for PackagePath {
    fn from(src: SourceLocation) -> Self {
        let name = src.name().replace('/', "::");
        let segments = name
            .split("::")
            .map(|segment| PackageName(segment.into()))
            .collect();

        Self { path: segments }
    }
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            trace: TraceContext(TraceConfig::Disabled),
            config: ConfigContext::default(),
            options: EvalOptions::new(),
        }
    }
}

/// A context when executing an evaluation step
pub struct ExecutionContext<'c> {
    /// The context of the overall evaluation
    eval: &'c EvalContext,
    /// the recursion level
    remaining_recursions: usize,
}

impl Deref for ExecutionContext<'_> {
    type Target = EvalContext;

    fn deref(&self) -> &Self::Target {
        &self.eval
    }
}

impl<'c> ExecutionContext<'c> {
    pub fn new(eval: &'c EvalContext) -> Self {
        Self {
            eval,
            remaining_recursions: eval.options.max_recursions,
        }
    }

    /// Create a new instance for descending into the evaluation tree.
    ///
    /// This creates a new instance with a decreased count of remaining recursions, or fail
    /// if the counter reached zero.
    ///
    /// **NOTE:** This must be called every time an operation can possibly call another one.
    pub fn push(&self) -> Result<Self, RuntimeError> {
        match self.remaining_recursions == 0 {
            true => Err(RuntimeError::RecursionLimit(
                self.eval.options.max_recursions,
            )),
            false => Ok(Self {
                eval: self.eval,
                remaining_recursions: self.remaining_recursions - 1,
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvalOptions {
    pub max_recursions: usize,
}

impl EvalOptions {
    const DEFAULT_MAX_RECURSIONS: usize = 128;

    /// Create a new instance
    ///
    /// This creates a new instance, possibly consulting configuration from the environment.
    ///
    /// **NOTE:** Using [`EvalOptions::default`] always return the same settings and **not** pull in
    /// information from the environment, or other sources.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        let max_recursions = match std::env::var("SEEDWING_RECURSION_LIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
        {
            Some(max_recursions) if max_recursions > 0 => max_recursions,
            _ => Self::DEFAULT_MAX_RECURSIONS,
        };

        Self { max_recursions }
    }

    /// Create a new instance.
    ///
    /// **NOTE:** On `wasm32`, this falls back to [`EvalOptions::default`].
    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            max_recursions: Self::DEFAULT_MAX_RECURSIONS,
        }
    }
}

#[derive(Debug)]
pub struct EvalContext {
    /// The trace aspect of the context
    pub trace: TraceContext,
    /// The configuration data aspect of the context
    pub config: ConfigContext,
    /// Options for running the evaluation
    pub options: EvalOptions,
}

impl EvalContext {
    pub fn new(trace: TraceConfig, config: ConfigContext, options: EvalOptions) -> Self {
        Self {
            trace: TraceContext(trace),
            config,
            options,
        }
    }

    pub fn new_with_config(config: ConfigContext, options: EvalOptions) -> Self {
        Self {
            trace: TraceContext(TraceConfig::Disabled),
            config,
            options,
        }
    }

    pub fn config(&self) -> &ConfigContext {
        &self.config
    }

    pub fn merge_config(&mut self, defaults: &ConfigContext) {
        self.config.merge_defaults(defaults);
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
    /// The pattern to be evaluated must be named `test-pattern`.
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
        builder.data(DirectoryDataSource::new(test_data_dir()));
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
        ( $result:expr ) => {{
            let __result = $result;
            assert!(
                __result.severity() < $crate::lang::Severity::Error,
                "severity: {}, response: {}",
                __result.severity(),
                serde_json::to_string_pretty(&$crate::runtime::response::Response::new(&__result))
                    .unwrap()
            );
        }};
    }

    #[macro_export]
    macro_rules! assert_not_satisfied {
        ( $result:expr ) => {{
            let __result = $result;
            assert!(
                __result.severity() == $crate::lang::Severity::Error,
                "severity: {}, response: {}",
                __result.severity(),
                serde_json::to_string_pretty(&$crate::runtime::response::Response::new(&__result))
                    .unwrap()
            );
        }};
    }

    /// Run a common test.
    ///
    /// The name of the pattern which will be required is "test".
    pub async fn test_common(
        content: impl Into<String>,
        value: impl Into<RuntimeValue>,
    ) -> EvaluationResult {
        let src = Ephemeral::new("test", content);

        let mut builder = Builder::new();
        builder.build(src.iter()).unwrap();
        let runtime = builder.finish().await.unwrap();

        runtime
            .evaluate("test::test", value, EvalContext::default())
            .await
            .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::builder::Builder;
    use crate::runtime::sources::{Directory, Ephemeral};

    use crate::runtime::metadata::{Documentation, InnerPatternMetadata, SubpackageMetadata};
    use crate::{assert_not_satisfied, assert_satisfied};
    use serde_json::json;
    use std::env;

    #[tokio::test]
    async fn ephemeral_sources() {
        let src = Ephemeral::new("foo::bar", "pattern bob");

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let result = builder.finish().await;
        assert!(matches!(result, Ok(_)));
    }

    #[tokio::test]
    async fn link_test_data() {
        let src = Directory::new(env::current_dir().unwrap().join("test-data"));

        let mut builder = Builder::new();
        let _result = builder.build(src.iter());
        let result = builder.finish().await;

        assert!(matches!(result, Ok(_)));
    }

    #[tokio::test]
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

        assert_satisfied!(result);
    }

    #[tokio::test]
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

        assert_satisfied!(testutil::test_pattern(pattern, bob).await);
        assert_not_satisfied!(testutil::test_pattern(pattern, frank).await);
    }

    #[tokio::test]
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

        assert_satisfied!(testutil::test_pattern(pattern, bob).await);
        assert_satisfied!(testutil::test_pattern(pattern, jim).await);
    }

    #[tokio::test]
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

        assert_satisfied!(testutil::test_pattern(pat, f("Bob", 49)).await);
        assert_satisfied!(testutil::test_pattern(pat, f("Jim", 53)).await);
        assert_not_satisfied!(testutil::test_pattern(pat, f("Jim", 49)).await);
        assert_not_satisfied!(testutil::test_pattern(pat, f("Bob", 42)).await);
    }

    #[tokio::test]
    async fn get_root_package() {
        let mut builder = Builder::new();

        let world = builder.finish().await.unwrap();

        let root = world.get_package_meta("");
        assert!(root.is_some());
        let root = root.unwrap();
        assert_eq!(root.name, "");

        // the root contains a lot, just check if we find something

        assert!(!root.packages.is_empty());
    }

    #[tokio::test]
    async fn get_package() {
        let mut builder = Builder::new();

        let src = Ephemeral::new("foo::bar", "pattern bob");
        builder.build(src.iter()).unwrap();
        let src = Ephemeral::new("foo::baz", "pattern jim");
        builder.build(src.iter()).unwrap();
        let src = Ephemeral::new("foo::baz::crash", "pattern boom");
        builder.build(src.iter()).unwrap();
        let src = Ephemeral::new("foo::baz::crash::boom", "pattern bang");
        builder.build(src.iter()).unwrap();

        let world = builder.finish().await.unwrap();

        let foo = world.get_package_meta("foo");
        assert_eq!(
            foo,
            Some(PackageMetadata {
                name: "foo".to_string(),
                documentation: Documentation::default(),
                packages: vec![
                    SubpackageMetadata {
                        name: "bar".to_string(),
                        documentation: Documentation::default()
                    },
                    SubpackageMetadata {
                        name: "baz".to_string(),
                        documentation: Documentation::default()
                    }
                ],

                patterns: vec![],
            }),
        );

        let foo_bar = world.get_package_meta("foo::bar");
        assert!(foo_bar.is_some());
        assert_eq!(
            foo_bar,
            Some(PackageMetadata {
                name: "foo::bar".to_string(),
                documentation: Documentation::default(),
                packages: vec![],
                patterns: vec![PatternMetadata {
                    name: Some("bob".to_string()),
                    path: Some("foo::bar::bob".to_string()),
                    metadata: Default::default(),
                    parameters: vec![],
                    inner: InnerPatternMetadata::Nothing,
                    examples: vec![],
                }],
            }),
        );
    }

    #[test]
    fn split_name() {
        assert_eq!(PackagePath::root().split_name(), None);
        assert_eq!(PackagePath::from("").split_name(), None);
        assert_eq!(
            PackagePath::from("foo").split_name(),
            Some((PackagePath::root(), "foo".into()))
        );
        assert_eq!(
            PackagePath::from("foo::bar").split_name(),
            Some((PackagePath::from(vec!["foo".to_string()]), "bar".into()))
        );
    }

    #[tokio::test]
    async fn fail_infinite_recursion() {
        let mut builder = Builder::new();
        builder
            .build(Ephemeral::new("test", "pattern foo = foo").iter())
            .unwrap();
        let runtime = builder.finish().await.unwrap();
        let result = runtime
            .evaluate("test::foo", RuntimeValue::Null, EvalContext::default())
            .await;

        assert!(matches!(
            result,
            Err(RuntimeError::RecursionLimit(
                EvalOptions::DEFAULT_MAX_RECURSIONS
            ))
        ));
    }

    #[tokio::test]
    async fn fail_circular_dependency() {
        let mut builder = Builder::new();
        builder
            .build(
                Ephemeral::new(
                    "test",
                    r#"
pattern foo = bar
pattern bar = foo
"#,
                )
                .iter(),
            )
            .unwrap();
        let runtime = builder.finish().await.unwrap();

        let result = runtime
            .evaluate("test::foo", RuntimeValue::Null, EvalContext::default())
            .await;

        assert!(matches!(
            result,
            Err(RuntimeError::RecursionLimit(
                EvalOptions::DEFAULT_MAX_RECURSIONS
            ))
        ));
    }
}

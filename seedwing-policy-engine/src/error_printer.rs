use crate::lang::parser::{ParserError, SourceLocation, SourceSpan};
use crate::runtime::cache::SourceCache;
use crate::runtime::BuildError;
use ariadne::{Cache, Label, Report, ReportKind, Source};
use chumsky::error::SimpleReason;
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::ops::Range;

pub struct ErrorPrinter<'c> {
    cache: &'c SourceCache,
}

impl<'c> ErrorPrinter<'c> {
    pub fn new(cache: &'c SourceCache) -> Self {
        Self { cache }
    }

    pub fn display(&self, errors: &[BuildError]) {
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
                BuildError::ArgumentMismatch(_, _) => {
                    format!("argument mismatch")
                }
                BuildError::TypeNotFound(_, _, name) => {
                    format!("type not found: {}", name)
                }
                BuildError::Parser(_, inner) => match inner.reason() {
                    SimpleReason::Unexpected => {
                        println!("{:?}", inner);
                        format!("unexpected character found {}", inner.found().unwrap())
                    }
                    SimpleReason::Unclosed { span, delimiter } => {
                        format!("unclosed delimiter {}", delimiter)
                    }
                    SimpleReason::Custom(inner) => inner.clone(),
                },
            }))
            .finish();

            // unfortunately we cannot render to a string (stream, or any write) as the writer in
            // consumed in the process.
            report.print(self.cache);
        }
    }
}

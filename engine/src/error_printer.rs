use crate::lang::parser::{SourceLocation, SourceSpan};
use crate::runtime::cache::SourceCache;
use crate::runtime::BuildError;
use ariadne::{Label, Report, ReportKind};
use chumsky::error::SimpleReason;

use std::io;

pub struct ErrorPrinter<'c> {
    cache: &'c SourceCache,
}

impl<'c> ErrorPrinter<'c> {
    pub fn new(cache: &'c SourceCache) -> Self {
        Self { cache }
    }

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
                        println!("{inner:?}");
                        format!("unexpected character found {}", inner.found().unwrap())
                    }
                    SimpleReason::Unclosed { span: _, delimiter } => {
                        format!("unclosed delimiter {delimiter}")
                    }
                    SimpleReason::Custom(inner) => inner.clone(),
                },
            }))
            .finish();

            report.write(self.cache, &mut w);
        }
    }

    pub fn display(&self, errors: &[BuildError]) {
        self.write_to(errors, &mut std::io::stdout().lock())
    }
}

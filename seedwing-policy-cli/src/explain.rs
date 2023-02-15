use seedwing_policy_engine::runtime::{
    rationale::{self, Rationale},
    EvaluationResult, Output,
};
use std::io::Write;
use termcolor::{Ansi, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[allow(dead_code)]
pub fn explain_writer(
    writer: &mut dyn Write,
    result: &EvaluationResult,
    verbosity: usize,
) -> std::io::Result<()> {
    explain_inner(&mut Ansi::new(writer), 0, result, verbosity)
}

// Verbosity levels:
//  0 - no reasoning output
//  1 - show input + rationale for leaf patterns that did not satisfy
//  2 - show input + rationale for all patterns that did not satisfy
//  3 - show input + rationale for all patterns whether or not they satisfied
//  4 - show input + rationale + supporting for all patterns whether or not they satisfied

pub fn explain(result: &EvaluationResult, verbosity: usize) -> std::io::Result<()> {
    if verbosity >= 1 {
        explain_inner(
            &mut StandardStream::stdout(ColorChoice::Auto).lock(),
            0,
            result,
            verbosity,
        )
    } else {
        Ok(())
    }
}

fn explain_inner(
    w: &mut dyn WriteColor,
    indent: usize,
    result: &EvaluationResult,
    verbosity: usize,
) -> std::io::Result<()> {
    const OFFSET: usize = 2;

    if !result.satisfied() {
        w.set_color(ColorSpec::new().set_bold(true))?;
    }

    writeln!(
        w,
        "{:indent$}Type: {}",
        "",
        result
            .ty()
            .name()
            .map(|t| t.to_string())
            .unwrap_or_else(|| "<none>".into())
    )?;
    writeln!(w, "{:indent$}Satisfied: {}", "", result.satisfied())?;
    if verbosity >= 1 || !result.satisfied() {
        writeln!(
            w,
            "{0:indent$}Input:\n{0:indent$}  {1}",
            "",
            serde_json::to_string_pretty(&result.input().as_json())
                .unwrap()
                .replace('\n', &format!("\n{:indent$}", "", indent = indent + OFFSET))
                .trim_end()
        )?;
    }

    if verbosity >= 2 || !result.satisfied() {
        writeln!(w, "{:indent$}Rationale:", "")?;
        let indent = indent + OFFSET;

        if !result.satisfied() {
            w.reset()?;
        }

        match result.rationale() {
            Rationale::Anything => {
                writeln!(w, "{:indent$}anything", "")?;
            }
            Rationale::Nothing => {
                writeln!(w, "{:indent$}nothing", "")?;
            }
            Rationale::Chain(terms) => {
                for r in terms {
                    explain_inner(w, indent + OFFSET, r, verbosity)?;
                }
            }
            Rationale::Object(fields) => {
                for (name, result) in fields {
                    writeln!(w, "{:indent$}field: {name}", "")?;
                    if let Some(inner) = result {
                        explain_inner(w, indent + OFFSET, inner, verbosity)?;
                    } else {
                        writeln!(w, "{:indent$}not present", "")?;
                    }
                }
            }
            Rationale::List(terms) => {
                writeln!(w, "{:indent$}List:", "")?;
                for r in terms {
                    explain_inner(w, indent + OFFSET, r, verbosity)?;
                }
            }
            Rationale::NotAnObject => {
                writeln!(w, "{:indent$}not an object", "")?;
            }
            Rationale::NotAList => {
                writeln!(w, "{:indent$}not a list", "")?;
            }
            Rationale::MissingField(name) => {
                writeln!(w, "{:indent$}missing field: {}", "", name)?;
            }
            Rationale::InvalidArgument(name) => {
                writeln!(w, "{:indent$}invalid argument: {}", "", name)?;
            }
            Rationale::Const(v) => {
                writeln!(w, "{:indent$}const({v})", "")?;
            }
            Rationale::Primordial(v) => {
                writeln!(w, "{:indent$}primordial({v})", "")?;
            }
            Rationale::Expression(v) => {
                writeln!(w, "{:indent$}expression({v})", "")?;
            }
            Rationale::Function(_, _, supporting) => {
                match result.raw_output() {
                    Output::None => {
                        writeln!(w, "{:indent$}Output: <none>", "")?;
                    }
                    Output::Identity => {
                        writeln!(w, "{:indent$}Output: <unchanged>", "")?;
                    }
                    Output::Transform(output) => {
                        writeln!(
                            w,
                            "{0:indent$}Output:\n{0:indent$}  {1}",
                            "",
                            serde_json::to_string_pretty(&output.as_json())
                                .unwrap()
                                .replace(
                                    '\n',
                                    &format!("\n{:indent$}", "", indent = indent + OFFSET)
                                )
                                .trim_end()
                        )?;
                    }
                }

                if verbosity >= 4 {
                    writeln!(w, "{:indent$}Supporting:", "")?;
                    for s in supporting {
                        explain_inner(w, indent + OFFSET, s, verbosity)?;
                    }
                }
            }
            Rationale::Refinement(primary, refinement) => {
                writeln!(w, "{:indent$}primary:", "")?;
                explain_inner(w, indent + OFFSET, primary, verbosity)?;
                if let Some(refinement) = refinement {
                    writeln!(w, "{:indent$}refinement:", "")?;
                    explain_inner(w, indent + OFFSET, refinement, verbosity)?;
                }
            }
        }
    }

    Ok(())
}

use seedwing_policy_engine::runtime::{rationale::Rationale, EvaluationResult, Output};
use std::io::Write;
use termcolor::{Ansi, ColorChoice, ColorSpec, StandardStream, WriteColor};

#[allow(dead_code)]
pub fn explain_writer(writer: &mut dyn Write, result: &EvaluationResult) -> std::io::Result<()> {
    explain_inner(&mut Ansi::new(writer), 0, result)
}

pub fn explain(result: &EvaluationResult) -> std::io::Result<()> {
    explain_inner(
        &mut StandardStream::stdout(ColorChoice::Auto).lock(),
        0,
        result,
    )
}

fn explain_inner(
    w: &mut dyn WriteColor,
    indent: usize,
    result: &EvaluationResult,
) -> std::io::Result<()> {
    const OFFSET: usize = 2;

    if !result.satisfied() {
        w.set_color(ColorSpec::new().set_bold(true))?;
    }

    writeln!(
        w,
        "{:indent$}Pattern: {}",
        "",
        result
            .ty()
            .name()
            .map(|t| t.to_string())
            .unwrap_or_else(|| "<none>".into())
    )?;
    writeln!(w, "{:indent$}Satisfied: {}", "", result.satisfied())?;
    writeln!(
        w,
        "{0:indent$}Value:\n{0:indent$}  {1}",
        "",
        result
            .input()
            .to_string()
            .replace('\n', &format!("\n{:indent$}", "", indent = indent + OFFSET))
            .trim_end()
    )?;

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
                explain_inner(w, indent + OFFSET, r)?;
            }
        }
        Rationale::Object(fields) => {
            for (name, result) in fields {
                writeln!(w, "{:indent$}field: {name}", "")?;
                if let Some(inner) = result {
                    explain_inner(w, indent + OFFSET, inner)?;
                } else {
                    writeln!(w, "{:indent$}not present", "")?;
                }
            }
        }
        Rationale::List(terms) => {
            writeln!(w, "{:indent$}List:", "")?;
            for r in terms {
                explain_inner(w, indent + OFFSET, r)?;
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
                        output
                            .to_string()
                            .replace('\n', &format!("\n{:indent$}", "", indent = indent + OFFSET))
                            .trim_end()
                    )?;
                }
            }

            writeln!(w, "{:indent$}Supporting:", "")?;
            for s in supporting {
                explain_inner(w, indent + OFFSET, s)?;
            }
        }
        Rationale::Refinement(primary, refinement) => {
            writeln!(w, "{:indent$}primary:", "")?;
            explain_inner(w, indent + OFFSET, primary)?;
            if let Some(refinement) = refinement {
                writeln!(w, "{:indent$}refinement:", "")?;
                explain_inner(w, indent + OFFSET, refinement)?;
            }
        }
    }

    Ok(())
}

use crate::cli::Context;
use seedwing_policy_engine::runtime::metadata::PackageMetadata;
use seedwing_policy_engine::runtime::{PackagePath, World};
use std::fs::{create_dir_all, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
#[command(
    about = "Generate documentation",
    args_conflicts_with_subcommands = true
)]
pub struct Docs {
    /// Generate a module for Antora
    #[arg(short = 'a', long = "antora", default_value_t = false)]
    pub antora: bool,
    /// Output directory
    #[arg(short = 'o', long = "output")]
    pub output: PathBuf,
}

impl Docs {
    pub async fn run(self, context: Context) -> anyhow::Result<()> {
        let world = context.world().await?.1;

        Generator {
            world,
            base: self.output,
        }
        .render()
    }
}

struct Generator {
    world: World,
    base: PathBuf,
}

impl Generator {
    fn render(self) -> anyhow::Result<()> {
        self.render_nav()?;
        Ok(())
    }

    fn render_nav(&self) -> anyhow::Result<()> {
        let path = self.base.join("nav.adoc");
        if let Some(parent) = path.parent() {
            create_dir_all(parent)?;
        }

        let mut out = BufWriter::new(File::create(path)?);

        self.walk_packages(PackagePath::root(), &mut |path, _meta| {
            let name = match path.name() {
                Some(name) => name,
                None => return Ok(()),
            };

            let level = path.path().len();
            let xref = path
                .path
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("/");

            writeln!(
                out,
                "{} xref:{xref}/index.adoc[`{name}`]",
                "*".repeat(level),
            )?;

            Ok(())
        })?;

        self.walk_packages(PackagePath::root(), &mut |path, meta| {
            self.render_page(path, meta)
        })?;

        Ok(())
    }

    fn walk_packages<F>(&self, current: PackagePath, f: &mut F) -> anyhow::Result<()>
    where
        F: FnMut(&PackagePath, &PackageMetadata) -> anyhow::Result<()>,
    {
        if let Some(pkg) = self.world.get_package_meta(current.clone()) {
            f(&current, &pkg)?;
            for child in pkg.packages {
                self.walk_packages(current.join(child.name), f)?;
            }
        }

        Ok(())
    }

    fn render_page(&self, path: &PackagePath, meta: &PackageMetadata) -> anyhow::Result<()> {
        let xref = path
            .path
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("/");

        let out = self.base.join("pages").join(xref).join("index.adoc");
        if let Some(parent) = out.parent() {
            create_dir_all(parent)?;
        }

        let mut out = BufWriter::new(File::create(out)?);

        let mut title = path.to_string();
        if title.is_empty() {
            title = "Root".to_string();
        }
        writeln!(out, "= {title}",)?;

        let summary = meta.documentation.summary();
        if !summary.is_empty() {
            writeln!(out, ":description: {summary}")?;
        }

        writeln!(out, ":sectanchors:")?;

        writeln!(out)?;
        writeln!(out, "{}", meta.documentation.summary())?;

        let details = meta.documentation.details();
        if !details.is_empty() {
            writeln!(out)?;
            writeln!(out, "{}", meta.documentation.details())?;
            writeln!(out)?;
        }

        for pattern in &meta.patterns {
            let name = match &pattern.name {
                Some(name) => name,
                None => continue,
            };

            // title

            writeln!(out)?;
            writeln!(out, "[#{name}]")?;
            write!(out, "== `{name}")?;
            if !pattern.parameters.is_empty() {
                write!(out, "<{}>", pattern.parameters.join(", "))?;
            }
            write!(out, "`")?;
            writeln!(out)?;

            // documentation

            writeln!(out)?;
            writeln!(out, "{}", pattern.metadata.documentation)?;

            if !pattern.examples.is_empty() {
                writeln!(out)?;
                writeln!(out, "=== Examples")?;
                writeln!(out)?;

                for ex in &pattern.examples {
                    writeln!(out)?;
                    writeln!(out, "==== {}", ex.summary.as_ref().unwrap_or(&ex.name))?;
                    if let Some(details) = &ex.description {
                        writeln!(out)?;
                        writeln!(out, "{}", details)?;
                    }
                    writeln!(out)?;
                    writeln!(out, "[source,json]")?;
                    writeln!(out, "----")?;
                    writeln!(
                        out,
                        "{}",
                        serde_json::to_string_pretty(&ex.value).unwrap_or_default()
                    )?;
                    writeln!(out, "----")?;
                }
            }
        }

        Ok(())
    }
}

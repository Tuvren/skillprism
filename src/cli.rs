use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use miette::IntoDiagnostic;

use crate::engine::Engine;
use crate::loader::ProjectLoader;
use crate::registry::HarnessRegistry;
use crate::resolver::HarnessResolver;
use crate::router::Router;
use crate::validator::Validator;

#[derive(Parser)]
#[command(name = "skillprism", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(global = true, short = 'v', long = "verbose")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    Build {
        #[arg(long = "target", default_value = "project")]
        target: TargetScope,

        #[arg(long = "diff")]
        diff: bool,

        #[arg(long = "force")]
        force: bool,
    },
    Validate {
        #[arg(default_value = ".")]
        path: String,
    },
    Init {
        #[command(subcommand)]
        kind: InitKind,
    },
}

/// Target scope for where rendered skill files are written.
#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum TargetScope {
    /// Write to the project-local skill directory.
    Project,
    /// Write to the user's home directory.
    User,
    /// Write to a distribution output directory.
    Dist,
}

#[derive(Subcommand)]
enum InitKind {
    Project {
        name: String,

        #[arg(short = 'o', long = "out")]
        out: Option<String>,
    },
    Skill {
        name: String,

        #[arg(short = 't', long = "targets")]
        targets: Option<String>,
    },
}

/// Entry point for the CLI application.
#[allow(clippy::redundant_pub_crate)]
pub(crate) fn run() {
    let cli = Cli::parse();
    if let Err(e) = dispatch(cli) {
        eprintln!("{e:?}");
        std::process::exit(1);
    }
}

fn dispatch(cli: Cli) -> Result<(), miette::Report> {
    match cli.command {
        Command::Build {
            target,
            diff,
            force,
        } => run_build(target, diff, force, cli.verbose),
        Command::Validate { path } => run_validate(&path),
        Command::Init { kind } => run_init(kind),
    }
}

fn run_build(
    target: TargetScope,
    diff: bool,
    force: bool,
    verbose: bool,
) -> Result<(), miette::Report> {
    let project_root = find_project_root()?;
    if verbose {
        eprintln!("[build] project root: {}", project_root.display());
    }

    let mut registry = HarnessRegistry::with_builtins();
    let harnesses_dir = project_root.join("harnesses");
    registry
        .load_user_overrides(&harnesses_dir)
        .into_diagnostic()?;

    let model = ProjectLoader::load(&project_root).into_diagnostic()?;
    if verbose {
        eprintln!("[build] loaded {} skills", model.skills.len());
    }

    let pairs = HarnessResolver::resolve_project(&model, &registry).map_err(|errors| {
        for err in &errors {
            eprintln!("{err:?}");
        }
        miette::miette!("Resolution failed with {} error(s)", errors.len())
    })?;
    if verbose {
        eprintln!("[build] resolved {} skill-harness pairs", pairs.len());
    }

    let outcome = Validator::validate(pairs);
    if !outcome.errors.is_empty() {
        for err in &outcome.errors {
            eprintln!("{err:?}");
        }
        return Err(miette::miette!(
            "Validation failed with {} error(s)",
            outcome.errors.len()
        ));
    }
    if verbose {
        eprintln!("[build] validated {} pairs", outcome.valid.len());
    }

    let mut files_written = 0usize;
    let mut files_unchanged = 0usize;
    let mut files_skipped = 0usize;

    for pair in &outcome.valid {
        let output = Engine::render(pair).into_diagnostic()?;

        if diff {
            let entries = Router::diff(pair, &output, &project_root, target);
            for entry in &entries {
                if entry.diff.stats.is_new_file {
                    println!(
                        "Diff for {}: new file (+{} lines)",
                        entry.path.display(),
                        entry.diff.stats.additions
                    );
                    println!("{}", entry.diff.hunks);
                } else if entry.diff.hunks.is_empty() {
                    println!("{}: no changes", entry.path.display());
                    files_unchanged += 1;
                } else {
                    println!(
                        "Diff for {}: +{}/-{} lines",
                        entry.path.display(),
                        entry.diff.stats.additions,
                        entry.diff.stats.deletions
                    );
                    println!("{}{}", entry.diff.header, entry.diff.hunks);
                }
                if !entry.diff.hunks.is_empty() || entry.diff.stats.is_new_file {
                    files_written += 1;
                }
            }
        } else {
            let result =
                Router::write(pair, &output, &project_root, target, force).into_diagnostic()?;
            files_written += 1 + result.written.sidecar_paths.len();
            if result.written.manifest_path.is_some() {
                files_written += 1;
            }
            files_skipped += result.skipped.len();
        }
    }

    if diff {
        println!("{files_written} file(s) changed, {files_unchanged} file(s) unchanged");
    } else if verbose {
        eprintln!("[build] wrote {files_written} file(s)");
    }

    if files_skipped > 0 {
        eprintln!("{files_skipped} file(s) skipped (use --force to overwrite)");
    }

    Ok(())
}

fn run_validate(path: &str) -> Result<(), miette::Report> {
    let root = PathBuf::from(path);
    let root = if root.is_absolute() {
        root
    } else {
        std::env::current_dir().into_diagnostic()?.join(root)
    };

    let mut registry = HarnessRegistry::with_builtins();
    let harnesses_dir = root.join("harnesses");
    registry
        .load_user_overrides(&harnesses_dir)
        .into_diagnostic()?;

    let model = ProjectLoader::load(&root).into_diagnostic()?;
    let pairs = HarnessResolver::resolve_project(&model, &registry).map_err(|errors| {
        for err in &errors {
            eprintln!("{err:?}");
        }
        miette::miette!("Resolution failed with {} error(s)", errors.len())
    })?;
    let outcome = Validator::validate(pairs);

    if outcome.errors.is_empty() {
        println!("Validation passed ({} skill(s))", outcome.valid.len());
        Ok(())
    } else {
        for err in &outcome.errors {
            eprintln!("{err:?}");
        }
        Err(miette::miette!(
            "Validation failed with {} error(s)",
            outcome.errors.len()
        ))
    }
}

fn run_init(kind: InitKind) -> Result<(), miette::Report> {
    match kind {
        InitKind::Project { name, out } => {
            let dir = out.map_or_else(|| PathBuf::from(&name), PathBuf::from);
            crate::scaffold::project::scaffold_project(&dir, &name).into_diagnostic()?;
            println!("Created project `{name}` in `{}`", dir.display());
            Ok(())
        }
        InitKind::Skill { name, targets } => {
            let root = find_project_root()?;
            let target_harnesses = targets
                .map(|t| {
                    t.split(',')
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            crate::scaffold::skill::scaffold_skill(&root, &name, &target_harnesses)
                .into_diagnostic()?;
            println!("Created skill `{name}`");
            Ok(())
        }
    }
}

fn find_project_root() -> Result<PathBuf, miette::Report> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join("skillprism.yaml").exists() {
            return Ok(dir.to_path_buf());
        }
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            return Err(miette::miette!(
                "No skillprism.yaml found. Run `skillprism init project <name>` to create one, or cd into a skillprism project."
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_target_user() {
        let cli = Cli::try_parse_from(["skillprism", "build", "--target", "user"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Build {
                target: TargetScope::User,
                ..
            }
        ));
    }

    #[test]
    fn build_diff_force() {
        let cli = Cli::try_parse_from(["skillprism", "build", "--diff", "--force"]).unwrap();
        match cli.command {
            Command::Build { diff, force, .. } => {
                assert!(diff);
                assert!(force);
            }
            _ => panic!("expected Build command"),
        }
    }

    #[test]
    fn verbose_validate() {
        let cli = Cli::try_parse_from(["skillprism", "--verbose", "validate"]).unwrap();
        assert!(cli.verbose);
        assert!(matches!(cli.command, Command::Validate { .. }));
    }

    #[test]
    fn validate_default_path() {
        let cli = Cli::try_parse_from(["skillprism", "validate"]).unwrap();
        match cli.command {
            Command::Validate { path } => {
                assert_eq!(path, ".");
            }
            _ => panic!("expected Validate command"),
        }
    }

    #[test]
    fn build_invalid_target() {
        let result = Cli::try_parse_from(["skillprism", "build", "--target", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn build_default_target() {
        let cli = Cli::try_parse_from(["skillprism", "build"]).unwrap();
        match cli.command {
            Command::Build { target, .. } => {
                assert!(matches!(target, TargetScope::Project));
            }
            _ => panic!("expected Build command"),
        }
    }
}

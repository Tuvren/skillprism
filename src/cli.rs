use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{Parser, Subcommand, ValueEnum};
use miette::IntoDiagnostic;

use crate::engine::Engine;
use crate::loader::ProjectLoader;
use crate::registry::HarnessRegistry;
use crate::resolver::HarnessResolver;
use crate::router::{ManifestEntry, Router};
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
    install_signal_handlers();

    let project_root = find_project_root()?;
    if verbose {
        eprintln!("[build] project root: {}", project_root.display());
    }

    let t0 = Instant::now();
    let (model, registry) = load_project(&project_root)?;
    if verbose {
        eprintln!("[{t}] load {} skills", model.skills.len(), t = fmt_duration(t0.elapsed()));
    }

    let t1 = Instant::now();
    let pairs = HarnessResolver::resolve_project(&model, &registry).map_err(|errors| {
        for err in &errors {
            eprintln!("{err:?}");
        }
        miette::miette!("Resolution failed with {} error(s)", errors.len())
    })?;
    if verbose {
        eprintln!("[{t}] resolve {} skill-harness pairs", pairs.len(), t = fmt_duration(t1.elapsed()));
    }

    let t2 = Instant::now();
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
        eprintln!("[{t}] validate {} pairs", outcome.valid.len(), t = fmt_duration(t2.elapsed()));
    }

    let t3 = Instant::now();
    let mut result = BuildResult::default();
    let mut manifest_entries: Vec<ManifestEntry> = Vec::new();
    let mut skip_all = false;

    for pair in &outcome.valid {
        let t_render = Instant::now();
        let output = Engine::render(pair).into_diagnostic()?;
        let render_time = fmt_duration(t_render.elapsed());

        if let Some(entry) = Engine::render_manifest_entry(pair) {
            if let Some(path) = crate::router::resolve_manifest_path(
                &project_root, &pair.harness, target,
            ) {
                let path = path.into_diagnostic()?;
                manifest_entries.push(ManifestEntry { path, content: entry });
            }
        }

        let pair_name = format!("{} \u{2192} {}", pair.skill.name, &pair.harness.id);
        if verbose {
            eprintln!("  [{render_time}] render {pair_name}");
        }

        if diff {
            let entries = Router::diff(pair, &output, &project_root, target);
            for entry in &entries {
                print_diff_entry(entry, &mut result);
            }
        } else {
            let t_write = Instant::now();
            let write_result =
                Router::write(pair, &output, &project_root, target, force, &mut skip_all).into_diagnostic()?;
            let write_time = fmt_duration(t_write.elapsed());
            let skill_skipped = write_result
                .skipped
                .contains(&write_result.written.skill_path.to_string_lossy().to_string());
            if !skill_skipped {
                result.changed += 1;
            }
            result.changed += write_result.written.sidecar_paths.len();
            result.skipped += write_result.skipped.len();
            if verbose {
                eprintln!("  [{write_time}] write {pair_name}");
            }
        }
    }

    if verbose {
        eprintln!("[{t}] render + write {} skills", outcome.valid.len(), t = fmt_duration(t3.elapsed()));
    }

    handle_manifests(diff, &manifest_entries, target, force, &mut skip_all, &mut result)?;

    if diff {
        println!("{} file(s) changed, {} file(s) unchanged", result.changed, result.unchanged);
    } else if verbose {
        eprintln!("[build] wrote {} file(s)", result.changed);
    }
    if result.skipped > 0 {
        if skip_all {
            eprintln!("{} file(s) skipped", result.skipped);
        } else {
            eprintln!("{} file(s) skipped (use --force to overwrite)", result.skipped);
        }
    }

    Ok(())
}

fn load_project(project_root: &Path) -> Result<(crate::types::ProjectModel, HarnessRegistry), miette::Report> {
    let mut registry = HarnessRegistry::with_builtins();
    let harnesses_dir = project_root.join("harnesses");
    registry
        .load_user_overrides(&harnesses_dir)
        .into_diagnostic()?;

    let model = ProjectLoader::load(project_root).into_diagnostic()?;
    Ok((model, registry))
}

fn handle_manifests(
    diff: bool,
    manifest_entries: &[ManifestEntry],
    target: TargetScope,
    force: bool,
    skip_all: &mut bool,
    result: &mut BuildResult,
) -> Result<(), miette::Report> {
    if diff {
        for entry in &Router::diff_manifests(manifest_entries) {
            print_diff_entry(entry, result);
        }
    } else if !manifest_entries.is_empty() {
        let mut manifest_skipped = Vec::new();
        let written = Router::write_aggregated_manifests(manifest_entries, target, force, skip_all, &mut manifest_skipped)
            .into_diagnostic()?;
        result.changed += written.len();
        result.skipped += manifest_skipped.len();
    }
    Ok(())
}

#[derive(Default)]
struct BuildResult {
    changed: usize,
    unchanged: usize,
    skipped: usize,
}

fn print_diff_entry(entry: &crate::router::DiffEntry, result: &mut BuildResult) {
    if entry.diff.stats.is_new_file {
        println!(
            "Diff for {}: new file (+{} lines)",
            entry.path.display(),
            entry.diff.stats.additions
        );
        println!("{}", entry.diff.hunks);
        result.changed += 1;
    } else if entry.diff.hunks.is_empty() {
        println!("{}: no changes", entry.path.display());
        result.unchanged += 1;
    } else {
        println!(
            "Diff for {}: +{}/-{} lines",
            entry.path.display(),
            entry.diff.stats.additions,
            entry.diff.stats.deletions
        );
        println!("{}{}", entry.diff.header, entry.diff.hunks);
        result.changed += 1;
    }
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

fn install_signal_handlers() {
    let result = ctrlc::set_handler(|| {
        eprintln!("\nSIGINT received — abandoning in-progress writes");
        std::process::exit(130);
    });

    if let Err(e) = result {
        eprintln!("Warning: failed to install SIGINT handler: {e}");
    }

    #[cfg(unix)]
    {
        extern "C" fn sigterm_handler(_: i32) {
            eprintln!("SIGTERM received — abandoning in-progress writes");
            std::process::exit(143);
        }

        const SIGTERM: i32 = 15;
        unsafe {
            let _ = signal(SIGTERM, sigterm_handler as *const () as usize);
        }
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn signal(sig: i32, handler: usize) -> usize;
}

fn fmt_duration(d: std::time::Duration) -> String {
    let secs = d.as_secs_f64();
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else {
        format!("{secs:.1}s")
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

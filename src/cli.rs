// Copyright 2026 Oscar Yáñez Cisterna (@SkrOYC)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

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

    /// Enable verbose progress output
    #[arg(global = true, short = 'v', long = "verbose")]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Compile templates and write harness-specific output files
    Build {
        #[arg(long = "target", default_value = "project")]
        target: TargetScope,

        /// Show a diff of changes without writing files
        #[arg(long = "diff", visible_alias = "dry-run")]
        diff: bool,

        /// Overwrite existing files without confirmation
        #[arg(long = "force")]
        force: bool,
    },
    /// Validate skill project without writing output
    Validate {
        /// Path to the project root directory
        #[arg(default_value = ".")]
        path: String,
    },
    /// Scaffold a new project, skill, or harness definition
    Init {
        #[command(subcommand)]
        kind: InitKind,
    },
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellKind,
    },
}

/// Shell to generate completion scripts for.
#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Bash,
    Fish,
    Zsh,
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
    /// Scaffold a full skillprism project
    Project {
        /// Project name
        name: String,

        /// Output directory (defaults to ./<name>)
        #[arg(short = 'o', long = "out")]
        out: Option<String>,

        /// Comma-separated list of harness IDs (default: claude, opencode)
        #[arg(short = 'H', long = "harnesses")]
        harnesses: Option<String>,
    },
    /// Scaffold a single skill into an existing project
    Skill {
        /// Skill name
        name: String,

        /// Comma-separated list of target harnesses (default: all built-in)
        #[arg(short = 'H', long = "harnesses")]
        harnesses: Option<String>,
    },
    /// Scaffold a new custom harness definition in harnesses/
    Harness {
        /// Harness name (used as the YAML filename and harness ID)
        name: String,
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
        Command::Completions { shell } => run_completions(shell),
    }
}

#[allow(clippy::too_many_lines)]
// reason: build pipeline orchestration — each step is justified; refactor deferred to Epic G.
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
        eprintln!(
            "[{t}] load {} skills",
            model.skills.len(),
            t = fmt_duration(t0.elapsed())
        );
    }

    let t1 = Instant::now();
    let pairs = HarnessResolver::resolve_project(&model, &registry).map_err(|errors| {
        for err in &errors {
            eprintln!("{err:?}");
        }
        miette::miette!("Resolution failed with {} error(s)", errors.len())
    })?;
    if verbose {
        eprintln!(
            "[{t}] resolve {} skill-harness pairs",
            pairs.len(),
            t = fmt_duration(t1.elapsed())
        );
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
        eprintln!(
            "[{t}] validate {} pairs",
            outcome.valid.len(),
            t = fmt_duration(t2.elapsed())
        );
    }

    if verbose {
        log_verbose_variables(&outcome.valid);
    }

    check_collisions(&outcome.valid, &project_root, target)?;

    let t3 = Instant::now();
    let mut result = BuildResult::default();
    let mut manifest_entries: Vec<ManifestEntry> = Vec::new();
    let mut skip_all = false;

    for pair in &outcome.valid {
        let t_render = Instant::now();
        let output = Engine::render(pair).into_diagnostic()?;
        let render_time = fmt_duration(t_render.elapsed());

        if let Some(entry) = Engine::render_manifest_entry(pair).into_diagnostic()? {
            if let Some(path) =
                crate::router::resolve_manifest_path(&project_root, &pair.harness, target)
            {
                let path = path.into_diagnostic()?;
                manifest_entries.push(ManifestEntry {
                    path,
                    content: entry,
                });
            }
        }

        let pair_name = format!("{} \u{2192} {}", pair.skill.name, &pair.harness.id);
        if verbose {
            eprintln!("  [{render_time}] render {pair_name}");
        }

        if diff {
            let entries = Router::diff(pair, &output, &project_root, target).into_diagnostic()?;
            for entry in &entries {
                print_diff_entry(entry, &mut result);
            }
        } else {
            let t_write = Instant::now();
            let write_result =
                Router::write(pair, &output, &project_root, target, force, &mut skip_all)
                    .into_diagnostic()?;
            let write_time = fmt_duration(t_write.elapsed());
            let skill_skipped = write_result.skipped.contains(
                &write_result
                    .written
                    .skill_path
                    .to_string_lossy()
                    .to_string(),
            );
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
        eprintln!(
            "[{t}] render + write {} skills",
            outcome.valid.len(),
            t = fmt_duration(t3.elapsed())
        );
    }

    handle_manifests(diff, &manifest_entries, force, &mut skip_all, &mut result)?;

    if diff {
        println!(
            "{} file(s) changed, {} file(s) unchanged",
            result.changed, result.unchanged
        );
    } else if verbose {
        eprintln!("[build] wrote {} file(s)", result.changed);
    }
    if result.skipped > 0 {
        if skip_all {
            eprintln!("{} file(s) skipped", result.skipped);
        } else {
            eprintln!(
                "{} file(s) skipped (use --force to overwrite)",
                result.skipped
            );
        }
    }

    Ok(())
}

fn load_project(
    project_root: &Path,
) -> Result<(crate::types::ProjectModel, HarnessRegistry), miette::Report> {
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
        let written = Router::write_aggregated_manifests(
            manifest_entries,
            force,
            skip_all,
            &mut manifest_skipped,
        )
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

fn run_completions(shell: ShellKind) -> Result<(), miette::Report> {
    let mut cmd = Cli::command();
    let clap_shell = match shell {
        ShellKind::Bash => clap_complete::Shell::Bash,
        ShellKind::Fish => clap_complete::Shell::Fish,
        ShellKind::Zsh => clap_complete::Shell::Zsh,
    };
    let cmd_name = cmd.get_name().to_string();
    let mut stdout = std::io::stdout().lock();
    clap_complete::generate(clap_shell, &mut cmd, cmd_name, &mut stdout);
    stdout.flush().into_diagnostic()?;
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

fn parse_harness_list(opt: Option<String>) -> Vec<String> {
    opt.map(|h| {
        h.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

fn run_init(kind: InitKind) -> Result<(), miette::Report> {
    match kind {
        InitKind::Project {
            name,
            out,
            harnesses,
        } => {
            let dir = out.map_or_else(|| PathBuf::from(&name), PathBuf::from);
            let selected = parse_harness_list(harnesses);
            crate::scaffold::project::scaffold_project(&dir, &name, &selected).into_diagnostic()?;
            println!("Created project `{name}` in `{}`", dir.display());
            Ok(())
        }
        InitKind::Skill { name, harnesses } => {
            let root = find_project_root()?;
            let selected = parse_harness_list(harnesses);
            crate::scaffold::skill::scaffold_skill(&root, &name, &selected).into_diagnostic()?;
            println!("Created skill `{name}`");
            Ok(())
        }
        InitKind::Harness { name } => {
            let root = find_project_root()?;
            crate::scaffold::harness::scaffold_harness(&root, &name).into_diagnostic()?;
            println!("Created harness `{name}`");
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
        // SAFETY: raw signal() is used because adding libc as a direct dependency
        // is not warranted for a single call. On Linux, signal() resets the handler
        // to SIG_DFL after firing (one-shot). This is safe because the handler
        // only calls _exit(143), so the process terminates on the first SIGTERM
        // regardless. If multi-signal handling is needed in the future, migrate
        // to sigaction() via the libc crate.
        #[allow(clippy::used_underscore_items)]
        extern "C" fn sigterm_handler(_: i32) {
            unsafe {
                _exit(143);
            }
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
    fn _exit(status: i32);
}

fn check_collisions(
    valid: &[crate::resolver::ResolvedPair],
    project_root: &Path,
    target: TargetScope,
) -> Result<(), miette::Report> {
    if let Err(errors) = Router::detect_collisions(valid, project_root, target) {
        for err in &errors {
            eprintln!("{err:?}");
        }
        return Err(miette::miette!(
            "Build aborted: {} path collision(s) detected",
            errors.len()
        ));
    }
    Ok(())
}

fn log_verbose_variables(valid: &[crate::resolver::ResolvedPair]) {
    for pair in valid {
        let pair_name = format!("{} \u{2192} {}", pair.skill.name, &pair.harness.id);
        if pair.skill.variables.is_empty() {
            eprintln!("  {pair_name}: (no variables)");
        } else {
            let vars: Vec<String> = pair
                .skill
                .variables
                .iter()
                .map(|(k, v)| format!("{k}={v:?}"))
                .collect();
            eprintln!("  {pair_name}: {}", vars.join(", "));
        }
    }
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
    fn completions_bash_includes_subcommands() {
        let mut cmd = Cli::command();
        let cmd_name = cmd.get_name().to_string();
        let mut buf = Vec::new();
        clap_complete::generate(clap_complete::Shell::Bash, &mut cmd, cmd_name, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("build"),
            "bash completions should include build"
        );
        assert!(
            output.contains("validate"),
            "bash completions should include validate"
        );
        assert!(
            output.contains("init"),
            "bash completions should include init"
        );
    }

    #[test]
    fn completions_parse_bash() {
        let cli = Cli::try_parse_from(["skillprism", "completions", "bash"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Completions {
                shell: ShellKind::Bash
            }
        ));
    }

    #[test]
    fn completions_parse_fish() {
        let cli = Cli::try_parse_from(["skillprism", "completions", "fish"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Completions {
                shell: ShellKind::Fish
            }
        ));
    }

    #[test]
    fn completions_parse_zsh() {
        let cli = Cli::try_parse_from(["skillprism", "completions", "zsh"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Completions {
                shell: ShellKind::Zsh
            }
        ));
    }

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

    #[test]
    fn dry_run_is_alias_for_diff() {
        let cli_diff = Cli::try_parse_from(["skillprism", "build", "--diff"]).unwrap();
        let cli_dry_run = Cli::try_parse_from(["skillprism", "build", "--dry-run"]).unwrap();
        match (cli_diff.command, cli_dry_run.command) {
            (Command::Build { diff: d1, .. }, Command::Build { diff: d2, .. }) => {
                assert!(d1, "--diff should set diff=true");
                assert!(d2, "--dry-run should also set diff=true");
            }
            _ => panic!("expected Build command for both"),
        }
    }
}

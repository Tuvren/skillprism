// CLI Command Tree
//
// This file documents the CLI interface using Rust clap derive syntax
// as the native contract format. The actual implementation lives in
// src/cli.rs and is generated from these same derive macros.

use clap::{Parser, Subcommand, ValueEnum};

/// Build-time compiler that transforms canonical skill sources into
/// harness-specific agent files.
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
    /// Compile templates and write harness-specific output
    Build {
        /// Target scope for output
        ///   project  → write to project-level agent paths (default)
        ///   user     → write to user-level (global) agent paths
        ///   dist     → write to ./dist/ for inspection
        #[arg(long = "target", default_value = "project")]
        target: TargetScope,

        /// Show diff against currently installed files without writing
        #[arg(long = "diff")]
        diff: bool,

        /// Overwrite existing files without confirmation
        #[arg(long = "force")]
        force: bool,
    },

    /// Validate skill project without writing output
    Validate {
        /// Optional path to project root (defaults to cwd)
        #[arg(default_value = ".")]
        path: String,
    },

    /// Scaffold a new project or skill
    Init {
        /// Type of scaffold
        #[command(subcommand)]
        kind: InitKind,
    },
}

#[derive(ValueEnum, Clone)]
enum TargetScope {
    Project,
    User,
    Dist,
}

#[derive(Subcommand)]
enum InitKind {
    /// Scaffold a full skillprism project
    Project {
        /// Project name
        name: String,

        /// Output directory (defaults to ./{name})
        #[arg(short = 'o', long = "out")]
        out: Option<String>,

        /// Comma-separated list of harness IDs (default: claude, opencode)
        #[arg(short = 'H', long = "harnesses")]
        harnesses: Option<String>,
    },
    /// Scaffold a single skill into an existing project
    Skill {
        /// Skill name (used for directory and SKILL.md title)
        name: String,

        /// Target harnesses (comma-separated, default: all built-in)
        #[arg(short = 'H', long = "harnesses")]
        harnesses: Option<String>,
    },
    /// Scaffold a new custom harness definition in harnesses/
    Harness {
        /// Harness name (used as the YAML filename and harness ID)
        name: String,
    },
}

// Exit codes:
//   0 — Success
//   1 — Validation error, render failure, or I/O error
//   2 — CLI argument error (clap handles this automatically)

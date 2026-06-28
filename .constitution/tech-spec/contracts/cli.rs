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
    /// Compile templates and write harness-specific output files
    Build {
        /// Target scope for output
        ///   project  → write to project-level agent paths (default)
        ///   user     → write to user-level (global) agent paths
        ///   dist     → write to ./dist/ for inspection
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
#[derive(ValueEnum, Clone, Copy)]
enum ShellKind {
    /// Generate completions for Bash
    Bash,
    /// Generate completions for Fish
    Fish,
    /// Generate completions for Zsh
    Zsh,
}

#[derive(ValueEnum, Clone, Copy)]
enum TargetScope {
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

// Exit codes:
//   0 — Success
//   1 — Validation error, render failure, or I/O error
//   2 — CLI argument error (clap handles this automatically)

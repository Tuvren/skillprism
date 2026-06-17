use clap::{Parser, Subcommand, ValueEnum};

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

#[derive(ValueEnum, Clone)]
enum TargetScope {
    Project,
    User,
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

#[allow(clippy::redundant_pub_crate)]
pub(crate) fn run() {
    let _cli = Cli::parse();
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

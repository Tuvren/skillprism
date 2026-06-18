use miette::Diagnostic;
use thiserror::Error;

/// Errors related to project loading and configuration.
#[derive(Debug, Diagnostic, Error)]
pub enum ProjectError {
    /// Failed to read a configuration file from disk.
    #[error("Failed to read project config: {path}")]
    #[diagnostic(help("Check that the file exists and is readable"))]
    ConfigRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// A YAML file contains invalid syntax.
    #[error("Invalid YAML in {path}:{line}")]
    #[diagnostic(help("{message}"))]
    YamlParse {
        path: String,
        line: usize,
        message: String,
    },

    /// A required field is missing from the configuration.
    #[error("Missing required field in {path}")]
    #[diagnostic(help("{message}"))]
    #[allow(dead_code)]
    MissingField { path: String, message: String },

    /// The skillprism.yaml project config file was not found.
    #[error("Project configuration not found: {path}")]
    #[diagnostic(help("Create a skillprism.yaml file in the project root"))]
    ConfigNotFound { path: String },

    /// The named harness does not exist in the registry.
    #[error("Unknown harness: {name}")]
    #[diagnostic(help("{message}"))]
    UnknownHarness { name: String, message: String },
}

#![allow(dead_code)]

use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum ProjectError {
    #[error("Failed to read project config: {path}")]
    ConfigRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid YAML in {path}:{line}")]
    #[diagnostic(help("{message}"))]
    YamlParse {
        path: String,
        line: usize,
        message: String,
    },

    #[error("Missing required field in {path}")]
    #[diagnostic(help("{message}"))]
    MissingField {
        path: String,
        message: String,
    },

    #[error("Project configuration not found: {path}")]
    #[diagnostic(help("Create a skillprism.yaml file in the project root"))]
    ConfigNotFound {
        path: String,
    },
}

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

    /// The skillprism.yaml project config file was not found.
    #[error("Project configuration not found: {path}")]
    #[diagnostic(help("Create a skillprism.yaml file in the project root"))]
    ConfigNotFound { path: String },

    /// The named harness does not exist in the registry.
    #[error("Unknown harness: {name}")]
    #[diagnostic(help("{message}"))]
    UnknownHarness { name: String, message: String },
}

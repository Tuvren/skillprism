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

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use miette::{Diagnostic, NamedSource, SourceSpan};
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
    #[error("YAML does not parse in {path}{location}: {message}")]
    YamlSyntax {
        path: String,
        location: YamlLocation,
        message: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{message}")]
        span: Option<SourceSpan>,
    },

    /// Valid YAML does not match the configuration schema.
    #[error("Invalid config in {path}{location}: {message}")]
    ConfigSchema {
        path: String,
        location: YamlLocation,
        message: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{message}")]
        span: Option<SourceSpan>,
    },

    /// The skillprism.yaml project config file was not found.
    #[error("Project configuration not found: {path}")]
    #[diagnostic(help("Create a skillprism.yaml file in the project root"))]
    ConfigNotFound { path: String },

    /// The named harness does not exist in the registry.
    #[error("Unknown harness: {name}")]
    #[diagnostic(help("{message}"))]
    UnknownHarness { name: String, message: String },

    /// A skill directory contains both SKILL.md.j2 and SKILL.md.
    #[error("Ambiguous template in {dir}: both SKILL.md.j2 and SKILL.md exist")]
    #[diagnostic(help(
        "A skill directory may have only one template file. Keep SKILL.md.j2 if it \
         needs the .j2 extension for tooling, or SKILL.md if you'd rather have your \
         editor treat it as plain Markdown — then delete the other one."
    ))]
    AmbiguousTemplate { dir: String },
}

/// A parser-reported position, or a file-level diagnostic without a position.
#[derive(Debug)]
pub enum YamlLocation {
    File,
    Position { line: usize, column: usize },
}

impl fmt::Display for YamlLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => Ok(()),
            Self::Position { line, column } => write!(f, " at line {line} column {column}"),
        }
    }
}

impl ProjectError {
    pub fn yaml_syntax(path: &Path, content: &str, error: &yaml_serde::Error) -> Self {
        let (location, span) = yaml_position(content, error.location());
        let path = path.to_string_lossy().into_owned();
        Self::YamlSyntax {
            src: Arc::new(NamedSource::new(path.clone(), content.to_owned())),
            path,
            location,
            message: error.to_string(),
            span,
        }
    }

    pub fn config_schema(
        path: &Path,
        content: &str,
        mut message: String,
        position: Option<yaml_serde::Location>,
    ) -> Self {
        let (location, span) = yaml_position(content, position);
        // The headline already prints this position; keep the reason concise.
        let suffix = location.to_string();
        if let Some(reason) = message.strip_suffix(&suffix) {
            message.truncate(reason.len());
        }
        let path = path.to_string_lossy().into_owned();
        Self::ConfigSchema {
            src: Arc::new(NamedSource::new(path.clone(), content.to_owned())),
            path,
            location,
            message,
            span,
        }
    }
}

fn yaml_position(
    content: &str,
    position: Option<yaml_serde::Location>,
) -> (YamlLocation, Option<SourceSpan>) {
    let Some(position) = position else {
        return (YamlLocation::File, None);
    };
    let line = position.line();
    let column = position.column();
    // yaml_serde 0.10 reports byte indices, including UTF-8 and CRLF widths.
    let offset = position.index();
    let span = content.get(offset..).map(|token| {
        let length = match token.find(|c: char| c.is_whitespace() || ":,[]{}".contains(c)) {
            Some(0) => token.chars().next().map_or(0, char::len_utf8),
            Some(length) => length,
            None => token.len(),
        };
        (offset, length).into()
    });
    (YamlLocation::Position { line, column }, span)
}

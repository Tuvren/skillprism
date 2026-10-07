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
    #[diagnostic(help("Fix the YAML syntax at the reported location"))]
    YamlSyntax {
        path: String,
        location: YamlLocation,
        message: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("here")]
        span: Option<SourceSpan>,
    },

    /// Valid YAML does not match the configuration schema.
    #[error("Invalid config in {path}{location}: {message}")]
    #[diagnostic(help("Check field names and value types against {kind}"))]
    ConfigSchema {
        kind: ConfigKind,
        path: String,
        location: YamlLocation,
        message: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("here")]
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

/// The schema whose documentation can help repair a configuration error.
#[derive(Debug, Clone, Copy)]
pub enum ConfigKind {
    Project,
    Skill,
    Harness,
}

impl fmt::Display for ConfigKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Project => "https://tuvren.github.io/skillprism/docs/quickstart/",
            Self::Skill => "https://tuvren.github.io/skillprism/docs/skill-yaml/",
            Self::Harness => "https://tuvren.github.io/skillprism/docs/harnesses/",
        })
    }
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
        let message = without_position(error.to_string(), &location);
        Self::YamlSyntax {
            src: Arc::new(NamedSource::new(path.clone(), content.to_owned())),
            path,
            location,
            message,
            span,
        }
    }

    pub fn config_schema(
        kind: ConfigKind,
        path: &Path,
        content: &str,
        mut message: String,
        position: Option<yaml_serde::Location>,
    ) -> Self {
        let (location, span) = yaml_position(content, position);
        message = without_position(message, &location);
        // A missing field has no source token; its enclosing mapping is misleading.
        let (location, span) =
            if message.starts_with("missing field `") || message.contains(": missing field `") {
                (YamlLocation::File, None)
            } else {
                (location, span)
            };
        let path = path.to_string_lossy().into_owned();
        Self::ConfigSchema {
            kind,
            src: Arc::new(NamedSource::new(path.clone(), content.to_owned())),
            path,
            location,
            message,
            span,
        }
    }
}

fn without_position(message: String, location: &YamlLocation) -> String {
    let position = location.to_string();
    if position.is_empty() {
        return message;
    }
    // Parser context can follow the primary position; retain distinct context marks.
    let mut reason = String::with_capacity(message.len());
    let mut start = 0;
    for (offset, text) in message.match_indices(&position) {
        let end = offset + text.len();
        if !message[end..].starts_with(|c: char| c.is_ascii_digit()) {
            reason.push_str(&message[start..offset]);
            start = end;
        }
    }
    reason.push_str(&message[start..]);
    reason
}

fn quoted_length(token: &str) -> Option<usize> {
    let quote = token.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let mut chars = token.char_indices().skip(1).peekable();
    while let Some((offset, c)) = chars.next() {
        if quote == '"' && c == '\\' {
            chars.next();
        } else if c == quote {
            if quote == '\'' && chars.peek().is_some_and(|(_, next)| *next == '\'') {
                chars.next();
            } else {
                return Some(offset + c.len_utf8());
            }
        }
    }
    None
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
        let length = quoted_length(token).unwrap_or_else(|| {
            match token.find(|c: char| c.is_whitespace() || ":,[]{}".contains(c)) {
                Some(0) => token.chars().next().map_or(0, char::len_utf8),
                Some(length) => length,
                None => token.len(),
            }
        });
        (offset, length).into()
    });
    (YamlLocation::Position { line, column }, span)
}

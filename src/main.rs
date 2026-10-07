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

#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cli;
mod distribution;
mod engine;
mod loader;
mod registry;
mod resolver;
mod router;
mod scaffold;
mod state;
mod types;
mod validator;

fn plain_diagnostics(stderr_is_terminal: bool, no_color: Option<&std::ffi::OsStr>) -> bool {
    // Match miette's supports-color detection: NO_COLOR disables color when present except "0".
    !stderr_is_terminal || no_color.is_some_and(|value| value != "0")
}

fn diagnostic_options(plain: bool) -> miette::MietteHandlerOpts {
    let options = miette::MietteHandlerOpts::new().wrap_lines(false);
    if plain {
        options.color(false).unicode(false).terminal_links(false)
    } else {
        options
    }
}

fn main() {
    use std::io::IsTerminal;

    let no_color = std::env::var_os("NO_COLOR");
    let plain = plain_diagnostics(std::io::stderr().is_terminal(), no_color.as_deref());
    miette::set_hook(Box::new(move |_| {
        Box::new(diagnostic_options(plain).build())
    }))
    .expect("the diagnostic hook is installed once at startup");

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "__generate_man" {
        if let Err(e) = cli::generate_man_page() {
            eprintln!("{e:?}");
            std::process::exit(1);
        }
        return;
    }
    cli::run();
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fmt;
    use std::path::Path;

    use miette::ReportHandler;

    use super::{diagnostic_options, plain_diagnostics};
    use crate::types::ProjectError;

    struct RenderedDiagnostic {
        handler: miette::MietteHandler,
        diagnostic: ProjectError,
    }

    impl fmt::Debug for RenderedDiagnostic {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.handler.debug(&self.diagnostic, formatter)
        }
    }

    #[test]
    fn diagnostic_help_urls_stay_intact_in_terminal_and_plain_modes() {
        for plain in [false, true] {
            let diagnostic = ProjectError::config_schema(
                Path::new("skillprism.yaml"),
                "name: my-skills\n",
                "unknown field `name`".to_owned(),
                None,
            );
            let handler = diagnostic_options(plain)
                .force_graphical(true)
                .width(80)
                .build();
            let rendered = format!(
                "{:?}",
                RenderedDiagnostic {
                    handler,
                    diagnostic
                }
            );
            for url in [
                "https://tuvren.github.io/skillprism/docs/quickstart/",
                "https://tuvren.github.io/skillprism/docs/skill-yaml/",
            ] {
                assert!(rendered.contains(url), "plain={plain}: {rendered}");
            }
            if plain {
                assert!(rendered.is_ascii(), "{rendered}");
                assert!(!rendered.contains('\u{1b}'), "{rendered}");
            }
        }
    }

    #[test]
    fn no_color_selects_plain_diagnostics_when_present_except_zero() {
        assert!(!plain_diagnostics(true, None));
        assert!(!plain_diagnostics(true, Some(OsStr::new("0"))));
        for value in ["", "1", " "] {
            assert!(plain_diagnostics(true, Some(OsStr::new(value))));
        }
        for no_color in [
            None,
            Some(OsStr::new("")),
            Some(OsStr::new("0")),
            Some(OsStr::new("1")),
        ] {
            assert!(plain_diagnostics(false, no_color));
        }
    }
}

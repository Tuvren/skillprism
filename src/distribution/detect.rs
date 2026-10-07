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

//! Auto-detection of installed agent harnesses.
//!
//! Probes common agent installation paths to determine which harnesses the
//! user has installed. Used as contextual information (e.g. hints in the
//! interactive `add` prompt), not as a default selection.

use std::path::{Path, PathBuf};

/// A known agent with its detection path and harness ID.
struct AgentProbe {
    /// Harness ID (e.g. "claude", "opencode")
    harness_id: &'static str,
    /// Path relative to `$HOME` to probe (e.g. ".claude")
    probe_path: &'static str,
}

/// Known agents to probe, ordered by popularity.
const AGENTS: &[AgentProbe] = &[
    AgentProbe {
        harness_id: "claude",
        probe_path: ".claude",
    },
    AgentProbe {
        harness_id: "opencode",
        probe_path: ".config/opencode",
    },
    AgentProbe {
        harness_id: "codex",
        probe_path: ".codex",
    },
    AgentProbe {
        harness_id: "factory",
        probe_path: ".factory",
    },
    AgentProbe {
        harness_id: "pi",
        probe_path: ".pi",
    },
];

/// Directory probed for `OpenCode`, matching the user-scope installer.
///
/// `$XDG_CONFIG_HOME/opencode` when that variable is set and non-empty, otherwise
/// `$HOME/.config/opencode`.
fn opencode_config_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("opencode"));
        }
    }
    let home = std::env::var("HOME").ok()?;
    if home.is_empty() {
        return None;
    }
    Some(PathBuf::from(home).join(".config/opencode"))
}

/// Detects which agents are installed by probing common agent paths.
///
/// Returns a list of harness IDs for agents that are detected on the system.
/// `OpenCode` uses the same `$XDG_CONFIG_HOME` base as user-scope installs.
/// Other agents are probed under `$HOME`. Returns an empty vec when no agent
/// directory exists.
pub fn detect_installed_agents() -> Vec<String> {
    let home = std::env::var("HOME")
        .ok()
        .filter(|home| !home.is_empty())
        .map(PathBuf::from);
    let mut found = home
        .as_ref()
        .map(|home| detect_from_home(home))
        .unwrap_or_default();
    // `detect_from_home` only sees `$HOME/.config/opencode`. Replace that hit
    // with the installer base so a custom `XDG_CONFIG_HOME` is what counts.
    found.retain(|id| id != "opencode");
    if opencode_config_dir().is_some_and(|path| path.exists()) {
        let index = usize::from(found.first().is_some_and(|id| id == "claude"));
        found.insert(index, "opencode".to_string());
    }
    found
}

/// Internal: probe from a given home directory (testable without env mocks).
fn detect_from_home(home: &Path) -> Vec<String> {
    AGENTS
        .iter()
        .filter(|agent| home.join(agent.probe_path).exists())
        .map(|agent| agent.harness_id.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_nothing_when_no_paths_exist() {
        let dir = tempfile::tempdir().unwrap();
        let detected = detect_from_home(dir.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn detects_single_agent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let detected = detect_from_home(dir.path());
        assert_eq!(detected, vec!["claude"]);
    }

    #[test]
    fn detects_multiple_agents() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        std::fs::create_dir_all(dir.path().join(".config/opencode")).unwrap();
        std::fs::create_dir_all(dir.path().join(".factory")).unwrap();
        let detected = detect_from_home(dir.path());
        assert!(detected.contains(&"claude".to_string()));
        assert!(detected.contains(&"opencode".to_string()));
        assert!(detected.contains(&"factory".to_string()));
        assert!(!detected.contains(&"pi".to_string()));
    }

    #[test]
    fn public_api_returns_empty_when_home_has_no_agents() {
        let _lock = crate::router::paths::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let empty_home = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", empty_home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::remove("XDG_CONFIG_HOME");
        assert!(detect_installed_agents().is_empty());
    }

    #[test]
    fn detects_opencode_from_xdg_config_home() {
        let _lock = crate::router::paths::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = tempfile::tempdir().unwrap();
        let xdg = tempfile::tempdir().unwrap();
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        let _xdg = crate::router::paths::tests::EnvGuard::set("XDG_CONFIG_HOME", xdg.path());

        std::fs::create_dir_all(home.path().join(".config/opencode")).unwrap();
        assert!(
            !detect_installed_agents().contains(&"opencode".to_string()),
            "OpenCode detection must follow XDG_CONFIG_HOME, not $HOME/.config"
        );

        std::fs::create_dir_all(xdg.path().join("opencode")).unwrap();
        assert_eq!(detect_installed_agents(), vec!["opencode".to_string()]);
    }

    #[test]
    fn detects_opencode_under_home_config_when_xdg_is_unset() {
        let _lock = crate::router::paths::tests::ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let home = tempfile::tempdir().unwrap();
        let _xdg = crate::router::paths::tests::EnvGuard::remove("XDG_CONFIG_HOME");
        let _home = crate::router::paths::tests::EnvGuard::set("HOME", home.path());
        std::fs::create_dir_all(home.path().join(".config/opencode")).unwrap();
        assert_eq!(detect_installed_agents(), vec!["opencode".to_string()]);
    }
}

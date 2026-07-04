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

/// Detects which agents are installed by probing common agent paths.
///
/// Returns a list of harness IDs for agents that are detected on the system.
/// Returns an empty vec if `$HOME` is not set or no agents are detected.
pub fn detect_installed_agents() -> Vec<String> {
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => return Vec::new(),
    };
    detect_from_home(&home)
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

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(suffix: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("skillprism-detect-{suffix}"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn detects_nothing_when_no_paths_exist() {
        let dir = TempDir::new("none");
        let detected = detect_from_home(dir.path());
        assert!(detected.is_empty());
    }

    #[test]
    fn detects_single_agent() {
        let dir = TempDir::new("single");
        std::fs::create_dir_all(dir.path().join(".claude")).unwrap();
        let detected = detect_from_home(dir.path());
        assert_eq!(detected, vec!["claude"]);
    }

    #[test]
    fn detects_multiple_agents() {
        let dir = TempDir::new("multi");
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
        let old_home = std::env::var_os("HOME");
        let empty_home = std::env::temp_dir().join("skillprism-empty-home-test");
        std::fs::create_dir_all(&empty_home).unwrap();
        // SAFETY: env var mutation is isolated by the test runner's default
        // process-per-test-thread model and restored below.
        unsafe { std::env::set_var("HOME", &empty_home) };
        assert!(detect_installed_agents().is_empty());
        if let Some(h) = old_home {
            // SAFETY: restoring the original HOME after the assertion.
            unsafe { std::env::set_var("HOME", h) };
        } else {
            unsafe { std::env::remove_var("HOME") };
        }
    }
}

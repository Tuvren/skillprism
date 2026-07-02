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

//! Agent Skills spec compliance checks (<https://agentskills.io/specification>).
//!
//! The spec defines hard constraints on `name`, `description`, and `compatibility`.
//! Each harness may define its own caps (`name_max_length`, `description_max_length`)
//! that can exceed the spec's portable caps — e.g. Claude Code allows descriptions up
//! to 1536 characters vs. the spec's 1024. Policy (per project decision):
//!
//! - **Hard error** when a value exceeds the *target harness* cap, or violates a
//!   universal spec rule (name format, name==directory, empty description).
//! - **Warning** when a value is over the spec's portable cap but still within the
//!   target harness's cap (e.g. a 1200-char description for Claude). The skill builds,
//!   but won't be portable to harnesses with stricter caps.

use crate::registry::HarnessDefinition;
use crate::types::SkillModel;

/// Spec portable caps — a skill within these builds everywhere without warnings.
const SPEC_NAME_MAX: usize = 64;
const SPEC_DESCRIPTION_MAX: usize = 1024;
const SPEC_COMPATIBILITY_MAX: usize = 500;

/// A hard spec or harness-cap violation that fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    /// Which constraint was violated.
    pub kind: SpecErrorKind,
    /// Human-readable detail for the diagnostic help text.
    pub detail: String,
}

/// The kind of spec/harness constraint that was violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecErrorKind {
    /// `name` doesn't match `^[a-z0-9]+(-[a-z0-9]+)*$` (lowercase, digits, hyphens;
    /// no leading/trailing/consecutive hyphens; max 64 per spec).
    InvalidName,
    /// `name` in skill.yaml doesn't match the parent directory name.
    NameDirectoryMismatch,
    /// `description` is empty — the spec requires at least one character.
    EmptyDescription,
    /// `description` exceeds the target harness's `description_max_length`.
    DescriptionExceedsHarness { len: usize, max: usize },
    /// `name` exceeds the target harness's `name_max_length`.
    NameExceedsHarness { len: usize, max: usize },
    /// `compatibility` exceeds the spec's 500-character limit.
    CompatibilityTooLong { len: usize },
    /// `compatibility` is present but empty; spec requires 1-500 characters.
    CompatibilityEmpty,
}

/// A non-fatal portability warning — the value is within the harness cap but over the
/// spec's portable cap, so the skill may not build cleanly for stricter harnesses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecWarning {
    /// Human-readable warning message.
    pub message: String,
}

/// Checks a skill against the Agent Skills spec and the target harness's caps.
///
/// Returns `(errors, warnings)`. Errors are hard failures; warnings are portability
/// nudges that don't block the build.
pub fn check_spec(
    skill: &SkillModel,
    harness: &HarnessDefinition,
) -> (Vec<SpecError>, Vec<SpecWarning>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let name = &skill.name;
    let harness_name = &harness.name;
    let harness_id = &harness.id;

    // --- name format (universal spec rule, always a hard error) ---
    if !is_valid_skill_name(name) {
        errors.push(SpecError {
            kind: SpecErrorKind::InvalidName,
            detail: format!(
                "`{name}` must be 1-64 characters, lowercase letters/digits/hyphens only, \
                 with no leading, trailing, or consecutive hyphens (spec: ^[a-z0-9]+(-[a-z0-9]+)*$)"
            ),
        });
    }

    // --- name == directory name (universal spec rule, always a hard error) ---
    if name != &skill.directory_name {
        errors.push(SpecError {
            kind: SpecErrorKind::NameDirectoryMismatch,
            detail: format!(
                "skill.yaml `name: {name}` does not match the skill's directory name \
                 `{dir}` — the Agent Skills spec requires them to be identical",
                dir = skill.directory_name
            ),
        });
    }

    // --- name length: error over harness cap, warn over spec cap (if within harness) ---
    let name_max = harness.capabilities.name_max_length;
    let name_len = name.chars().count();
    if name_len > name_max {
        errors.push(SpecError {
            kind: SpecErrorKind::NameExceedsHarness { len: name_len, max: name_max },
            detail: format!(
                "name is {name_len} characters but {harness_name} ({harness_id}) allows at most {name_max}"
            ),
        });
    } else if name_len > SPEC_NAME_MAX {
        // Only reachable when harness cap > spec cap (e.g. codex allows 100).
        warnings.push(SpecWarning {
            message: format!(
                "name is {name_len} characters — over the spec's portable cap of {SPEC_NAME_MAX} \
                 but within {harness_name}'s cap of {name_max}. The skill builds for \
                 {harness_id} but may not be portable to harnesses with stricter limits."
            ),
        });
    }

    // --- description: error if empty, error over harness cap, warn over spec cap ---
    let desc = &skill.description;
    if desc.is_empty() {
        errors.push(SpecError {
            kind: SpecErrorKind::EmptyDescription,
            detail: "description is empty — the Agent Skills spec requires a non-empty \
                     description that says what the skill does and when to use it"
                .to_string(),
        });
    } else {
        let desc_max = harness.capabilities.description_max_length;
        let desc_len = desc.chars().count();
        if desc_len > desc_max {
            errors.push(SpecError {
                kind: SpecErrorKind::DescriptionExceedsHarness {
                    len: desc_len,
                    max: desc_max,
                },
                detail: format!(
                    "description is {desc_len} characters but {harness_name} ({harness_id}) \
                     allows at most {desc_max}"
                ),
            });
        } else if desc_len > SPEC_DESCRIPTION_MAX {
            // Only reachable when harness cap > spec cap (e.g. claude allows 1536).
            warnings.push(SpecWarning {
                message: format!(
                    "description is {desc_len} characters — over the spec's portable cap of \
                     {SPEC_DESCRIPTION_MAX} but within {harness_name}'s cap of {desc_max}. The \
                     skill builds for {harness_id} but may not be portable to harnesses with \
                     stricter limits."
                ),
            });
        }
    }

    // --- compatibility length (spec cap only, no harness-specific cap) ---
    if let Some(compat) = skill.compatibility.as_ref() {
        let compat_len = compat.chars().count();
        if compat_len == 0 {
            errors.push(SpecError {
                kind: SpecErrorKind::CompatibilityEmpty,
                detail: format!(
                    "compatibility is present but empty — the Agent Skills spec requires it \
                     to be 1-{SPEC_COMPATIBILITY_MAX} characters when provided"
                ),
            });
        } else if compat_len > SPEC_COMPATIBILITY_MAX {
            errors.push(SpecError {
                kind: SpecErrorKind::CompatibilityTooLong { len: compat_len },
                detail: format!(
                    "compatibility is {compat_len} characters — the spec limit is \
                     {SPEC_COMPATIBILITY_MAX}. Shorten it or move details to a reference file."
                ),
            });
        }
    }

    (errors, warnings)
}

/// Validates a skill name's *format* (characters and hyphen rules) against the spec's
/// `^[a-z0-9]+(-[a-z0-9]+)*$` pattern: non-empty, lowercase letters + digits + hyphens
/// only, no leading/trailing/consecutive hyphens. Length is checked separately against
/// the harness cap (hard error) and spec cap (warning), so this function does NOT
/// enforce the spec's 64-char limit — a harness may allow longer names (e.g. codex: 100).
fn is_valid_skill_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut prev_was_hyphen = true; // start can't be a hyphen
    for c in name.chars() {
        match c {
            'a'..='z' | '0'..='9' => prev_was_hyphen = false,
            '-' => {
                if prev_was_hyphen {
                    return false; // leading or consecutive hyphen
                }
                prev_was_hyphen = true;
            }
            _ => return false, // uppercase, underscores, unicode, etc.
        }
    }
    !prev_was_hyphen // can't end with a hyphen
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::HarnessRegistry;
    use crate::resolver::tests::test_skill;

    fn claude() -> HarnessDefinition {
        HarnessRegistry::with_builtins().resolve("claude").unwrap()
    }
    fn codex() -> HarnessDefinition {
        HarnessRegistry::with_builtins().resolve("codex").unwrap()
    }
    fn opencode() -> HarnessDefinition {
        HarnessRegistry::with_builtins()
            .resolve("opencode")
            .unwrap()
    }

    #[test]
    fn valid_name_passes() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "Does things.".to_string();
        let (errors, warnings) = check_spec(&skill, &claude());
        assert!(errors.is_empty(), "errors: {errors:?}");
        assert!(warnings.is_empty(), "warnings: {warnings:?}");
    }

    #[test]
    fn uppercase_name_rejected() {
        let mut skill = test_skill("My-Agent", vec![]);
        skill.directory_name = "My-Agent".to_string();
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &claude());
        assert!(errors.iter().any(|e| e.kind == SpecErrorKind::InvalidName));
    }

    #[test]
    fn leading_hyphen_rejected() {
        let mut skill = test_skill("-agent", vec![]);
        skill.directory_name = "-agent".to_string();
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &claude());
        assert!(errors.iter().any(|e| e.kind == SpecErrorKind::InvalidName));
    }

    #[test]
    fn trailing_hyphen_rejected() {
        let mut skill = test_skill("agent-", vec![]);
        skill.directory_name = "agent-".to_string();
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &claude());
        assert!(errors.iter().any(|e| e.kind == SpecErrorKind::InvalidName));
    }

    #[test]
    fn consecutive_hyphens_rejected() {
        let mut skill = test_skill("my--agent", vec![]);
        skill.directory_name = "my--agent".to_string();
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &claude());
        assert!(errors.iter().any(|e| e.kind == SpecErrorKind::InvalidName));
    }

    #[test]
    fn underscore_in_name_rejected() {
        let mut skill = test_skill("my_agent", vec![]);
        skill.directory_name = "my_agent".to_string();
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &claude());
        assert!(errors.iter().any(|e| e.kind == SpecErrorKind::InvalidName));
    }

    #[test]
    fn name_directory_mismatch_rejected() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.directory_name = "my-skill".to_string();
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &claude());
        assert!(
            errors
                .iter()
                .any(|e| e.kind == SpecErrorKind::NameDirectoryMismatch)
        );
    }

    #[test]
    fn empty_description_rejected() {
        let skill = test_skill("my-agent", vec![]);
        let (errors, _) = check_spec(&skill, &claude());
        assert!(
            errors
                .iter()
                .any(|e| e.kind == SpecErrorKind::EmptyDescription)
        );
    }

    #[test]
    fn description_within_spec_passes_everywhere() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "x".repeat(1024);
        let (errors, warnings) = check_spec(&skill, &claude());
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
        let (errors, warnings) = check_spec(&skill, &codex());
        assert!(
            errors
                .iter()
                .any(|e| matches!(e.kind, SpecErrorKind::DescriptionExceedsHarness { .. }))
        );
        assert!(warnings.is_empty());
    }

    #[test]
    fn description_over_spec_but_within_claude_cap_warns() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "x".repeat(1200); // 1024 < 1200 < 1536
        let (errors, warnings) = check_spec(&skill, &claude());
        assert!(
            errors.is_empty(),
            "within claude cap — should not error: {errors:?}"
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("portable cap"));
    }

    #[test]
    fn description_over_claude_cap_errors() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "x".repeat(1537); // > 1536
        let (errors, _) = check_spec(&skill, &claude());
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            SpecErrorKind::DescriptionExceedsHarness {
                len: 1537,
                max: 1536
            }
        )));
    }

    #[test]
    fn description_over_codex_cap_errors_no_warn_band() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "x".repeat(600); // > 500 (codex cap), < 1024 (spec)
        let (errors, warnings) = check_spec(&skill, &codex());
        // codex cap (500) < spec cap (1024), so anything over 500 is an error — no warn band.
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            SpecErrorKind::DescriptionExceedsHarness { len: 600, max: 500 }
        )));
        assert!(warnings.is_empty());
    }

    #[test]
    fn description_at_codex_cap_passes() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "x".repeat(500);
        let (errors, warnings) = check_spec(&skill, &codex());
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn name_over_spec_but_within_codex_cap_warns() {
        let name = "a".repeat(80);
        let mut skill = test_skill(&name, vec![]);
        skill.directory_name = name;
        skill.description = "Does things.".to_string();
        let (errors, warnings) = check_spec(&skill, &codex());
        // codex allows name up to 100; spec is 64. 80 is over spec but within codex.
        assert!(
            errors.is_empty(),
            "within codex cap — should not error: {errors:?}"
        );
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn name_over_codex_cap_errors() {
        let name = "a".repeat(101);
        let mut skill = test_skill(&name, vec![]);
        skill.directory_name = name;
        skill.description = "Does things.".to_string();
        let (errors, _) = check_spec(&skill, &codex());
        assert!(errors.iter().any(|e| matches!(
            e.kind,
            SpecErrorKind::NameExceedsHarness { len: 101, max: 100 }
        )));
    }

    #[test]
    fn compatibility_within_limit_passes() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "Does things.".to_string();
        skill.compatibility = Some("x".repeat(500));
        let (errors, _) = check_spec(&skill, &opencode());
        assert!(errors.is_empty());
    }

    #[test]
    fn compatibility_over_limit_rejected() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "Does things.".to_string();
        skill.compatibility = Some("x".repeat(501));
        let (errors, _) = check_spec(&skill, &opencode());
        assert!(
            errors
                .iter()
                .any(|e| matches!(e.kind, SpecErrorKind::CompatibilityTooLong { len: 501 }))
        );
    }

    #[test]
    fn no_compatibility_field_is_fine() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "Does things.".to_string();
        skill.compatibility = None;
        let (errors, _) = check_spec(&skill, &opencode());
        assert!(errors.is_empty());
    }

    #[test]
    fn empty_compatibility_rejected() {
        let mut skill = test_skill("my-agent", vec![]);
        skill.description = "Does things.".to_string();
        skill.compatibility = Some(String::new());
        let (errors, _) = check_spec(&skill, &opencode());
        assert!(
            errors
                .iter()
                .any(|e| e.kind == SpecErrorKind::CompatibilityEmpty),
            "empty compatibility should be rejected: {errors:?}"
        );
    }

    #[test]
    fn description_uses_char_count_not_byte_count() {
        let mut skill = test_skill("my-agent", vec![]);
        // Each 'é' is 2 bytes in UTF-8 but 1 char. 1024 chars of 'é' = 2048 bytes.
        // The cap is in characters, so 1024 chars should pass even though it's 2048 bytes.
        skill.description = "é".repeat(1024);
        let (errors, warnings) = check_spec(&skill, &opencode());
        assert!(
            errors.is_empty(),
            "1024 chars should pass regardless of byte count: {errors:?}"
        );
        assert!(warnings.is_empty());
    }
}

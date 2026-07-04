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

//! System-wide state tracking layer for installed skills.
//!
//! The state file lives at `~/.config/skillprism/installed.yaml` (or
//! `$XDG_CONFIG_HOME/skillprism/installed.yaml` when set). The directory is
//! created with mode `0o700` and the file with mode `0o600`. Every mutation
//! reads the entire file, computes the new state in memory, and rewrites the
//! entire file via a single temp-rename operation.
//!
//! # Concurrency note
//!
//! v1 of the state layer does **not** support concurrent `add` calls in the
//! same state directory. Two simultaneous writers will read the same baseline,
//! compute independent new states, and the second rename will silently clobber
//! the first. Serialize `skillprism add` invocations (e.g., one CI job, one
//! human terminal). A `flock` follow-up is deferred to a future epic.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current schema version of `installed.yaml`.
const STATE_VERSION: u32 = 1;

/// Directory permissions for the state directory: owner read/write/execute only.
const STATE_DIR_MODE: u32 = 0o700;

/// File permissions for `installed.yaml`: owner read/write only.
const STATE_FILE_MODE: u32 = 0o600;

/// Errors that can occur when reading or writing the installation state.
#[derive(Debug, Diagnostic, Error)]
pub enum StateError {
    /// The user's home directory could not be determined.
    #[error("Could not determine home directory")]
    #[diagnostic(help(
        "Set the HOME environment variable, or set XDG_CONFIG_HOME to an existing directory."
    ))]
    MissingHome,

    /// An I/O error occurred while accessing the state file or directory.
    #[error("State file I/O error: {0}")]
    Io(#[from] io::Error),

    /// The state file could not be parsed as YAML.
    #[error("Failed to parse state file `{path}`: {detail}")]
    #[diagnostic(help("Delete or repair the file, then retry."))]
    YamlParse { path: String, detail: String },

    /// The state file has an unsupported schema version.
    #[error("Unsupported state file version: {version}")]
    #[diagnostic(help("Delete the state file to start fresh, or upgrade skillprism."))]
    UnsupportedVersion { version: u32 },
}

/// Where a skill was installed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallScope {
    /// Project-local install (e.g., `.claude/skills/` inside the project).
    Project,
    /// User-global install (e.g., `~/.claude/skills/`).
    User,
}

/// The source type used to fetch a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    /// GitHub-hosted repository.
    GitHub,
    /// GitLab-hosted repository (including self-hosted).
    GitLab,
    /// Generic git repository.
    Git,
    /// Local filesystem path.
    Local,
    /// Discovered via `.well-known/agent-skills/index.json`.
    #[serde(rename = "wellknown")]
    WellKnown,
}

/// The format a skill was installed as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillFormat {
    /// A skillprism-format skill with `skill.yaml` declaring `skillprism: '1'`.
    Skillprism,
    /// A plain-format skill copied as-is.
    Plain,
}

/// A single per-file record stored for change detection on `update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledFile {
    /// Relative or absolute path where the file was installed.
    pub path: String,
    /// `sha256:<hex>` hash of the file content.
    pub hash: String,
}

/// A single installed skill record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSkill {
    /// Skill name (kebab-case, directory-derived or declared).
    pub name: String,
    /// The exact source string the user passed to `add`.
    pub source: String,
    /// Normalized URL the source resolved to.
    pub source_url: String,
    /// Classifier for the source host/protocol.
    pub source_type: SourceType,
    /// Git ref (branch/tag/SHA) at install time; `None` for local sources.
    pub r#ref: Option<String>,
    /// Resolved upstream SHA the `ref` pointed to at install/update time.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub resolved_ref: Option<String>,
    /// Subpath within the source that contained the skill; `None` if root.
    pub skill_path: Option<String>,
    /// Absolute project root where the skill was installed; `None` for user scope
    /// or when the root could not be determined.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub project_root: Option<String>,
    /// Install scope: project or user.
    pub scope: InstallScope,
    /// Harnesses the skill was rendered/copied to.
    pub harnesses: Vec<String>,
    /// Format declared by the skill manifest.
    pub format: SkillFormat,
    /// ISO 8601 timestamp of first install.
    pub installed_at: String,
    /// ISO 8601 timestamp of last update.
    pub updated_at: String,
    /// Per-file records for change detection.
    pub files: Vec<InstalledFile>,
}

/// Top-level state document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledState {
    /// Schema version.
    pub version: u32,
    /// Installed skill records, kept sorted alphabetically by `name`.
    pub skills: Vec<InstalledSkill>,
}

impl InstalledState {
    /// Returns an empty state document at the current schema version.
    pub const fn empty() -> Self {
        Self {
            version: STATE_VERSION,
            skills: Vec::new(),
        }
    }

    /// Returns the record for the named skill, if present.
    pub fn get(&self, name: &str) -> Option<&InstalledSkill> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Removes the record for the named skill, returning whether it existed.
    pub fn remove(&mut self, name: &str) -> bool {
        let len = self.skills.len();
        self.skills.retain(|s| s.name != name);
        self.skills.len() != len
    }

    /// Inserts a new record or replaces an existing one, then re-sorts by name.
    pub fn upsert(&mut self, skill: InstalledSkill) {
        if let Some(pos) = self.skills.iter().position(|s| s.name == skill.name) {
            self.skills[pos] = skill;
        } else {
            self.skills.push(skill);
        }
        self.skills.sort_by(|a, b| a.name.cmp(&b.name));
    }
}

/// A handle to the installation state file.
///
/// Opening the store initializes the state directory and file if they do not
/// exist, and loads the current document. Mutations are not persisted until
/// [`StateStore::save`] is called.
#[derive(Debug)]
pub struct StateStore {
    path: PathBuf,
    state: InstalledState,
}

impl StateStore {
    /// Opens the state store, creating the directory and empty file if needed.
    pub fn open() -> Result<Self, StateError> {
        Self::open_at(&state_dir()?)
    }

    /// Opens the state store at an explicit directory.
    ///
    /// This is primarily useful for tests and for tooling that needs to manage
    /// an alternative state location without mutating environment variables.
    pub fn open_at(dir: &Path) -> Result<Self, StateError> {
        fs::create_dir_all(dir)?;
        #[cfg(unix)]
        set_dir_mode(dir, STATE_DIR_MODE)?;

        let path = dir.join("installed.yaml");
        let state = if path.exists() {
            Self::read(&path)?
        } else {
            let empty = InstalledState::empty();
            Self::atomic_write(&path, &empty)?;
            empty
        };

        Ok(Self { path, state })
    }

    /// Path to the state file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Immutable view of the loaded state.
    pub const fn state(&self) -> &InstalledState {
        &self.state
    }

    /// Mutable view of the loaded state.
    pub const fn state_mut(&mut self) -> &mut InstalledState {
        &mut self.state
    }

    /// Returns all installed skills.
    pub fn skills(&self) -> &[InstalledSkill] {
        &self.state.skills
    }

    /// Returns the record for the named skill, if present.
    pub fn get(&self, name: &str) -> Option<&InstalledSkill> {
        self.state.get(name)
    }

    /// Inserts or replaces a skill record in memory.
    pub fn upsert(&mut self, skill: InstalledSkill) {
        self.state.upsert(skill);
    }

    /// Removes a skill record in memory, returning whether it existed.
    pub fn remove(&mut self, name: &str) -> bool {
        self.state.remove(name)
    }

    /// Persists the current in-memory state atomically.
    pub fn save(&self) -> Result<(), StateError> {
        Self::atomic_write(&self.path, &self.state)
    }

    fn read(path: &Path) -> Result<InstalledState, StateError> {
        let content = fs::read_to_string(path)?;
        let mut state: InstalledState =
            yaml_serde::from_str(&content).map_err(|e| StateError::YamlParse {
                path: path.to_string_lossy().to_string(),
                detail: e.to_string(),
            })?;

        if state.version != STATE_VERSION {
            return Err(StateError::UnsupportedVersion {
                version: state.version,
            });
        }

        state.skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(state)
    }

    fn atomic_write(path: &Path, state: &InstalledState) -> Result<(), StateError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let yaml = yaml_serde::to_string(state).map_err(|e| StateError::YamlParse {
            path: path.to_string_lossy().to_string(),
            detail: e.to_string(),
        })?;

        let tmp_name = format!(
            ".tmp-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            path.file_name()
                .map_or_else(|| "state".to_string(), |n| n.to_string_lossy().to_string())
        );
        let tmp_path = path.with_file_name(tmp_name);
        {
            #[cfg(unix)]
            {
                use std::fs::OpenOptions;
                use std::os::unix::fs::OpenOptionsExt;
                let mut file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .mode(STATE_FILE_MODE)
                    .open(&tmp_path)?;
                file.write_all(yaml.as_bytes())?;
                file.sync_all()?;
            }
            #[cfg(not(unix))]
            {
                let mut file = fs::File::create(&tmp_path)?;
                file.write_all(yaml.as_bytes())?;
                file.sync_all()?;
            }
        }

        #[cfg(unix)]
        set_file_mode(&tmp_path, STATE_FILE_MODE)?;

        fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

/// Resolves the state directory using `$XDG_CONFIG_HOME` with a `~/.config`
/// fallback.
pub fn state_dir() -> Result<PathBuf, StateError> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("skillprism"));
        }
    }

    let home = home_dir()?;
    Ok(home.join(".config/skillprism"))
}

fn home_dir() -> Result<PathBuf, StateError> {
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() => Ok(PathBuf::from(h)),
        _ => Err(StateError::MissingHome),
    }
}

#[cfg(unix)]
fn set_dir_mode(dir: &Path, mode: u32) -> io::Result<()> {
    set_mode(dir, mode)
}

#[cfg(unix)]
fn set_file_mode(path: &Path, mode: u32) -> io::Result<()> {
    set_mode(path, mode)
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions)
}

/// Returns the current time as an RFC 3339 / ISO 8601 string.
pub fn now_rfc3339() -> String {
    system_time_to_rfc3339(SystemTime::now())
}

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let duration = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();

    // Convert seconds to UTC date components.
    let days = secs / 86_400;
    let (year, month, day) = days_to_ymd(days);
    let day_secs = secs % 86_400;
    let hour = day_secs / 3_600;
    let minute = (day_secs % 3_600) / 60;
    let second = day_secs % 60;

    if nanos == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
    } else {
        let micros = nanos / 1_000;
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{micros:06}Z")
    }
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970_u64;
    loop {
        let year_days = if is_leap_year(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }

    let month_lengths = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1_u64;
    for &length in &month_lengths {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }

    (year, month, days + 1)
}

const fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialises tests that touch the real state directory path. Tests that
    // need an isolated directory use `StateStore::open_at` instead.
    static STATE_LOCK: Mutex<()> = Mutex::new(());

    fn temp_state_dir(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("skillprism_test")
            .join("state")
            .join(name)
    }

    fn with_temp_store<F>(name: &str, f: F)
    where
        F: FnOnce(StateStore),
    {
        let _lock = STATE_LOCK.lock().unwrap();
        let dir = temp_state_dir(name);
        let _ = fs::remove_dir_all(&dir);
        let store = StateStore::open_at(&dir).unwrap();
        f(store);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_state_dir_with_mode_700() {
        with_temp_store("dir_mode", |store| {
            let dir = store.path().parent().unwrap();
            assert!(dir.exists());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(dir).unwrap().permissions().mode();
                assert_eq!(mode & 0o777, 0o700);
            }
        });
    }

    #[test]
    fn creates_empty_state_file() {
        with_temp_store("empty_file", |store| {
            assert!(store.path().exists());
            assert_eq!(store.state().version, STATE_VERSION);
            assert!(store.skills().is_empty());
            let content = fs::read_to_string(store.path()).unwrap();
            assert_eq!(content.trim(), "version: 1\nskills: []");
        });
    }

    #[test]
    fn state_file_has_mode_600() {
        with_temp_store("file_mode", |store| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(store.path()).unwrap().permissions().mode();
                assert_eq!(mode & 0o777, 0o600);
            }
        });
    }

    #[test]
    fn state_file_mode_600_ignores_stale_tmp() {
        with_temp_store("file_mode_existing_tmp", |mut store| {
            // Pre-create a stale temp file with liberal permissions. The writer
            // now uses a unique temp name, so this file is ignored, but the
            // final state file must still end up as 0o600.
            let tmp = store.path().with_extension("tmp");
            fs::write(&tmp, b"stale").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&tmp).unwrap().permissions();
                perms.set_mode(0o644);
                fs::set_permissions(&tmp, perms).unwrap();
            }

            store.upsert(dummy_skill("one"));
            store.save().unwrap();

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(store.path()).unwrap().permissions().mode();
                assert_eq!(mode & 0o777, 0o600);
            }
        });
    }

    #[test]
    fn upsert_sorts_skills_alphabetically() {
        with_temp_store("upsert_sort", |mut store| {
            store.upsert(dummy_skill("charlie"));
            store.upsert(dummy_skill("alpha"));
            store.upsert(dummy_skill("bravo"));
            store.save().unwrap();

            let names: Vec<_> = store.skills().iter().map(|s| s.name.clone()).collect();
            assert_eq!(names, vec!["alpha", "bravo", "charlie"]);
        });
    }

    #[test]
    fn remove_deletes_record() {
        with_temp_store("remove_record", |mut store| {
            store.upsert(dummy_skill("keep"));
            store.upsert(dummy_skill("drop"));
            store.save().unwrap();

            assert!(store.remove("drop"));
            assert!(!store.remove("missing"));
            assert!(store.get("keep").is_some());
            assert!(store.get("drop").is_none());
        });
    }

    #[test]
    fn roundtrip_preserves_full_record() {
        with_temp_store("roundtrip", |mut store| {
            let original = InstalledSkill {
                name: "my-skill".to_string(),
                source: "owner/repo".to_string(),
                source_url: "https://github.com/owner/repo.git".to_string(),
                source_type: SourceType::GitHub,
                r#ref: Some("abc123".to_string()),
                resolved_ref: Some("abc123".to_string()),
                skill_path: Some("skills/pdf".to_string()),
                project_root: Some("/home/user/project".to_string()),
                scope: InstallScope::Project,
                harnesses: vec!["claude".to_string(), "opencode".to_string()],
                format: SkillFormat::Skillprism,
                installed_at: "2026-07-02T14:23:45Z".to_string(),
                updated_at: "2026-07-02T14:23:45Z".to_string(),
                files: vec![InstalledFile {
                    path: ".claude/skills/my-skill/SKILL.md".to_string(),
                    hash: "sha256:abc123".to_string(),
                }],
            };
            store.upsert(original.clone());
            store.save().unwrap();

            let reloaded = StateStore::open_at(store.path().parent().unwrap()).unwrap();
            assert_eq!(reloaded.skills().len(), 1);
            assert_eq!(reloaded.skills()[0], original);
        });
    }

    #[test]
    fn xdg_config_home_overrides_default() {
        let _lock = STATE_LOCK.lock().unwrap();
        let dir = temp_state_dir("xdg_override");
        let _ = fs::remove_dir_all(&dir);
        let xdg = dir.join("xdg_config");
        let prev = std::env::var("XDG_CONFIG_HOME").ok();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &xdg);
        }

        assert_eq!(state_dir().unwrap(), xdg.join("skillprism"));

        if let Some(prev) = prev {
            unsafe { std::env::set_var("XDG_CONFIG_HOME", prev) };
        } else {
            unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_to_home_when_xdg_unset() {
        let _lock = STATE_LOCK.lock().unwrap();
        let dir = temp_state_dir("home_fallback");
        let _ = fs::remove_dir_all(&dir);
        let home = dir.join("home");
        fs::create_dir_all(&home).unwrap();

        let prev_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let prev_home = std::env::var("HOME").ok();
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::set_var("HOME", &home);
        }

        assert_eq!(state_dir().unwrap(), home.join(".config/skillprism"));

        if let Some(xdg) = prev_xdg {
            unsafe { std::env::set_var("XDG_CONFIG_HOME", xdg) };
        } else {
            unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        }
        if let Some(h) = prev_home {
            unsafe { std::env::set_var("HOME", h) };
        } else {
            unsafe { std::env::remove_var("HOME") };
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_is_atomic() {
        with_temp_store("atomic", |mut store| {
            store.upsert(dummy_skill("one"));
            store.save().unwrap();
            // The writer uses unique `.tmp-<pid>-<nanos>-<name>` files and
            // renames them into place; no temp file should remain.
            let dir = store.path().parent().unwrap();
            let leftover: Vec<_> = fs::read_dir(dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
                .collect();
            assert!(leftover.is_empty(), "temp files should be renamed away");
        });
    }

    #[test]
    fn unsupported_version_rejected() {
        with_temp_store("bad_version", |store| {
            let dir = store.path().parent().unwrap();
            fs::create_dir_all(dir).unwrap();
            fs::write(dir.join("installed.yaml"), "version: 99\nskills: []\n").unwrap();
            let err = StateStore::open_at(dir).unwrap_err();
            assert!(matches!(
                err,
                StateError::UnsupportedVersion { version: 99 }
            ));
        });
    }

    #[test]
    fn rfc3339_format_is_valid() {
        let s = system_time_to_rfc3339(SystemTime::UNIX_EPOCH);
        assert_eq!(s, "1970-01-01T00:00:00Z");
    }

    fn dummy_skill(name: &str) -> InstalledSkill {
        InstalledSkill {
            name: name.to_string(),
            source: format!("owner/{name}"),
            source_url: "https://github.com/owner/repo.git".to_string(),
            source_type: SourceType::GitHub,
            r#ref: Some("main".to_string()),
            resolved_ref: None,
            skill_path: None,
            project_root: None,
            scope: InstallScope::Project,
            harnesses: vec!["claude".to_string()],
            format: SkillFormat::Skillprism,
            installed_at: now_rfc3339(),
            updated_at: now_rfc3339(),
            files: vec![InstalledFile {
                path: format!(".claude/skills/{name}/SKILL.md"),
                hash: "sha256:abc".to_string(),
            }],
        }
    }
}

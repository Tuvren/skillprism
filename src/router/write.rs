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

use std::fs;
use std::io;
use std::path::Path;

/// Atomically writes content to a file by writing to a temp file then renaming.
pub fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    atomic_write_bytes(path, content.as_bytes())
}

/// Atomically writes bytes to a file by writing to a temp file then renaming.
pub fn atomic_write_bytes(path: &Path, content: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Copies asset directories (every direct subdirectory of a skill's own directory,
/// regardless of name) to the skill output directory.
pub fn copy_assets(source_dirs: &[impl AsRef<Path>], target_dir: &Path) -> io::Result<()> {
    for src in source_dirs {
        let src = src.as_ref();
        if !src.exists() {
            continue;
        }

        let dir_name = src.file_name().expect("asset dir should have a name");

        let dst = target_dir.join(dir_name);
        copy_dir_recursive(src, &dst)?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("source is not a directory: {}", src.display()),
        ));
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let metadata = fs::symlink_metadata(&src_path)?;

        if metadata.is_symlink() {
            let target = fs::read_link(&src_path)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&target, &dst_path)?;
            #[cfg(not(unix))]
            fs::copy(&src_path, &dst_path)?;
        } else if metadata.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_creates_file() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("atomic_write");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let target = dir.join("output.md");
        atomic_write(&target, "Hello, World!").unwrap();
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "Hello, World!");

        let tmp_file = target.with_extension("tmp");
        assert!(!tmp_file.exists(), "tmp file should be cleaned up");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("atomic_write_nested");
        let _ = fs::remove_dir_all(&dir);

        let target = dir.join("deep/nested/output.md");
        atomic_write(&target, "nested").unwrap();
        assert!(target.exists());
        assert_eq!(fs::read_to_string(&target).unwrap(), "nested");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_assets_replicates_directories() {
        let dir = std::env::temp_dir()
            .join("skillprism_test")
            .join("copy_assets");
        let _ = fs::remove_dir_all(&dir);

        let src = dir.join("source");
        let refs = src.join("references");
        fs::create_dir_all(&refs).unwrap();
        fs::write(refs.join("guide.md"), "# Guide").unwrap();
        fs::write(refs.join("config.json"), "{}").unwrap();

        let scripts = src.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::write(scripts.join("build.sh"), "#!/bin/bash").unwrap();

        let dst = dir.join("output/skill-name");
        copy_assets(&[src.join("references"), src.join("scripts")], &dst).unwrap();

        assert!(dst.join("references").exists());
        assert!(dst.join("references/guide.md").exists());
        assert!(dst.join("references/config.json").exists());
        assert!(dst.join("scripts").exists());
        assert!(dst.join("scripts/build.sh").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}

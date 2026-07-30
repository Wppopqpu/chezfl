use std::path::Path;
use std::time::SystemTime;

/// Check if a path exists and is a regular file.
pub fn is_file(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    Ok(path.as_ref().metadata().map(|m| m.is_file()).unwrap_or(false))
}

/// Check if a path exists and is a directory.
pub fn is_dir(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    Ok(path.as_ref().metadata().map(|m| m.is_dir()).unwrap_or(false))
}

/// Check if a path is a symlink (broken or not).
pub fn is_symlink(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    Ok(path
        .as_ref()
        .symlink_metadata()
        .map(|m| m.is_symlink())
        .unwrap_or(false))
}

/// Check if a path exists and is executable (any file type).
///
/// Uses the Unix executable permission bit (`mode & 0o111`).
/// On non-Unix platforms, always returns `false`.
#[cfg(unix)]
pub fn is_runnable(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    Ok(path
        .as_ref()
        .metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false))
}

/// Check if a path exists (any kind: file, dir, symlink, etc.).
pub fn exists(path: impl AsRef<Path>) -> anyhow::Result<bool> {
    Ok(path.as_ref().exists())
}

/// Read a file's contents into a `String`.
pub fn read_to_string(path: impl AsRef<Path>) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(path.as_ref())?)
}

/// Write a string to a file, creating parent directories if needed.
pub fn write(path: impl AsRef<Path>, content: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(std::fs::write(path.as_ref(), content)?)
}

/// Copy a file from `src` to `dst`, creating parent directories if needed.
pub fn copy(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    if let Some(parent) = dst.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(std::fs::copy(src.as_ref(), dst.as_ref()).map(|_| ())?)
}

/// Remove a file or empty directory.
pub fn remove(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    if path.is_dir() {
        Ok(std::fs::remove_dir(path)?)
    } else {
        Ok(std::fs::remove_file(path)?)
    }
}

/// Recursively remove a file or directory.
pub fn remove_all(path: impl AsRef<Path>) -> anyhow::Result<()> {
    Ok(std::fs::remove_dir_all(path.as_ref())?)
}

/// Create a directory and all parents (like `mkdir -p`).
pub fn create_dir(path: impl AsRef<Path>) -> anyhow::Result<()> {
    Ok(std::fs::create_dir_all(path.as_ref())?)
}

/// Create a symbolic link `src → dst`.
#[cfg(unix)]
pub fn symlink(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    if let Some(parent) = dst.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(std::os::unix::fs::symlink(src.as_ref(), dst.as_ref())?)
}

/// Get the modification time of a file as seconds since Unix epoch.
///
/// Returns `None` if the path does not exist or cannot be accessed
/// (e.g. permission denied on a parent directory).
pub fn mtime(path: impl AsRef<Path>) -> anyhow::Result<Option<u64>> {
    match path.as_ref().metadata() {
        Ok(meta) => {
            let d = meta.modified()?.duration_since(SystemTime::UNIX_EPOCH)?;
            Ok(Some(d.as_secs()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Check whether a target is up-to-date with respect to its prerequisites,
/// using the same logic as `make`: the target needs a rebuild when it does
/// not exist, or when any prerequisite is newer (has a later mtime).
///
/// Missing prerequisites are treated as "needs rebuild" (returns `false`),
/// since the build result cannot be verified without them.
///
/// # Example
///
/// ```rust
/// use chezfl::tools::fs::{write, mtime, up_to_date};
/// let tmp = std::env::temp_dir();
/// let tgt = tmp.join("target");
/// let pre = tmp.join("prereq");
/// write(&tgt, "built").unwrap();
/// write(&pre, "source").unwrap();
/// // target is newer than prereq → up-to-date
/// assert!(up_to_date(&tgt, &[&pre]).unwrap());
/// ```
pub fn up_to_date(target: impl AsRef<Path>, prerequisites: &[impl AsRef<Path>]) -> anyhow::Result<bool> {
    let target_mtime = match mtime(&target)? {
        Some(t) => t,
        None => return Ok(false),
    };

    for prereq in prerequisites {
        match mtime(prereq)? {
            Some(m) if m <= target_mtime => continue,
            _ => return Ok(false),
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_file() {
        let f = std::env::temp_dir().join("chezfl_fs_test_is_file");
        std::fs::write(&f, "hello").unwrap();
        assert!(is_file(&f).unwrap());
        assert!(!is_file(std::env::temp_dir()).unwrap());
        assert!(!is_file("/nonexistent_path_chezfl").unwrap());
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn test_is_dir() {
        assert!(is_dir(std::env::temp_dir()).unwrap());
        let f = std::env::temp_dir().join("chezfl_fs_test_is_dir_file");
        std::fs::write(&f, "x").unwrap();
        assert!(!is_dir(&f).unwrap());
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn test_symlink() {
        let tmp = std::env::temp_dir();
        let target = tmp.join("chezfl_fs_test_symlink_target");
        let link = tmp.join("chezfl_fs_test_symlink_link");
        std::fs::write(&target, "content").unwrap();
        symlink(&target, &link).unwrap();
        assert!(is_symlink(&link).unwrap());
        assert!(is_file(&link).unwrap());
        assert!(exists(&link).unwrap());
        std::fs::remove_file(&link).unwrap();
        std::fs::remove_file(&target).unwrap();
    }

    #[test]
    fn test_is_runnable() {
        let tmp = std::env::temp_dir();
        let f = tmp.join("chezfl_fs_test_runnable");
        std::fs::write(&f, "#!/bin/sh").unwrap();
        assert!(!is_runnable(&f).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(is_runnable(&f).unwrap());
        }
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn test_exists() {
        let f = std::env::temp_dir().join("chezfl_fs_test_exists");
        assert!(!exists(&f).unwrap());
        std::fs::write(&f, "x").unwrap();
        assert!(exists(&f).unwrap());
        assert!(is_file(&f).unwrap());
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn test_read_write() {
        let f = std::env::temp_dir().join("chezfl_fs_test_read_write");
        write(&f, "hello world").unwrap();
        let content = read_to_string(&f).unwrap();
        assert_eq!(content, "hello world");
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn test_write_creates_parent_dirs() {
        let f = std::env::temp_dir()
            .join("chezfl_fs_test_write_dir")
            .join("nested")
            .join("file.txt");
        write(&f, "nested content").unwrap();
        assert!(is_file(&f).unwrap());
        assert_eq!(read_to_string(&f).unwrap(), "nested content");
        std::fs::remove_dir_all(f.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn test_copy() {
        let src = std::env::temp_dir().join("chezfl_fs_test_copy_src");
        let dst = std::env::temp_dir().join("chezfl_fs_test_copy_dst");
        write(&src, "copy me").unwrap();
        copy(&src, &dst).unwrap();
        assert_eq!(read_to_string(&dst).unwrap(), "copy me");
        std::fs::remove_file(&src).unwrap();
        std::fs::remove_file(&dst).unwrap();
    }

    #[test]
    fn test_copy_creates_parent_dirs() {
        let src = std::env::temp_dir().join("chezfl_fs_test_copy_parent_src");
        let dst = std::env::temp_dir()
            .join("chezfl_fs_test_copy_parent_dir")
            .join("sub")
            .join("file.txt");
        write(&src, "x").unwrap();
        copy(&src, &dst).unwrap();
        assert!(is_file(&dst).unwrap());
        std::fs::remove_file(&src).unwrap();
        std::fs::remove_dir_all(
            dst.parent().unwrap().parent().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_remove_file() {
        let f = std::env::temp_dir().join("chezfl_fs_test_remove_file");
        write(&f, "x").unwrap();
        assert!(exists(&f).unwrap());
        remove(&f).unwrap();
        assert!(!exists(&f).unwrap());
    }

    #[test]
    fn test_remove_dir() {
        let d = std::env::temp_dir().join("chezfl_fs_test_remove_dir");
        create_dir(&d).unwrap();
        assert!(is_dir(&d).unwrap());
        remove(&d).unwrap();
        assert!(!exists(&d).unwrap());
    }

    #[test]
    fn test_remove_all() {
        let d = std::env::temp_dir()
            .join("chezfl_fs_test_remove_all");
        let f = d.join("sub").join("file.txt");
        write(&f, "x").unwrap();
        assert!(is_file(&f).unwrap());
        remove_all(&d).unwrap();
        assert!(!exists(&d).unwrap());
    }

    #[test]
    fn test_create_dir() {
        let d = std::env::temp_dir()
            .join("chezfl_fs_test_create_dir")
            .join("a")
            .join("b");
        create_dir(&d).unwrap();
        assert!(is_dir(&d).unwrap());
        std::fs::remove_dir_all(
            d.parent().unwrap().parent().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_mtime() {
        let f = std::env::temp_dir().join("chezfl_fs_test_mtime");
        assert!(mtime(&f).unwrap().is_none());
        write(&f, "content").unwrap();
        assert!(mtime(&f).unwrap().is_some());
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn test_up_to_date_no_target() {
        let tmp = std::env::temp_dir();
        let tgt = tmp.join("chezfl_fs_test_uptodate_target");
        let pre = tmp.join("chezfl_fs_test_uptodate_prereq");
        write(&pre, "src").unwrap();
        assert!(!up_to_date(&tgt, &[&pre]).unwrap());
        std::fs::remove_file(&pre).unwrap();
    }

    #[test]
    fn test_up_to_date_target_newer() {
        let tmp = std::env::temp_dir();
        let tgt = tmp.join("chezfl_fs_test_uptodate_newer_target");
        let pre = tmp.join("chezfl_fs_test_uptodate_newer_prereq");
        write(&pre, "src").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
        write(&tgt, "built").unwrap();
        assert!(up_to_date(&tgt, &[&pre]).unwrap());
        std::fs::remove_file(&tgt).unwrap();
        std::fs::remove_file(&pre).unwrap();
    }

    #[test]
    fn test_up_to_date_prereq_newer() {
        let tmp = std::env::temp_dir();
        let tgt = tmp.join("chezfl_fs_test_uptodate_prereq_newer_target");
        let pre = tmp.join("chezfl_fs_test_uptodate_prereq_newer_prereq");
        write(&tgt, "built").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(2));
        write(&pre, "src").unwrap();
        assert!(!up_to_date(&tgt, &[&pre]).unwrap());
        std::fs::remove_file(&tgt).unwrap();
        std::fs::remove_file(&pre).unwrap();
    }

    #[test]
    fn test_up_to_date_missing_prereq() {
        let tmp = std::env::temp_dir();
        let tgt = tmp.join("chezfl_fs_test_uptodate_missing_prereq_target");
        let pre = tmp.join("chezfl_fs_test_uptodate_missing_prereq_prereq");
        write(&tgt, "built").unwrap();
        assert!(!up_to_date(&tgt, &[&pre]).unwrap());
        std::fs::remove_file(&tgt).unwrap();
    }
}

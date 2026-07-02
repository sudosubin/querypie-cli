use anyhow::{anyhow, Context, Result};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::paths;

const LOCK_TIMEOUT: Duration = Duration::from_secs(60);
const STALE_AFTER: Duration = Duration::from_secs(600);
const RETRY_INTERVAL: Duration = Duration::from_millis(100);

pub struct HostLock {
    path: PathBuf,
    _file: File,
}

impl HostLock {
    pub fn acquire(host: &str) -> Result<Self> {
        acquire_path(paths::host_lock_file(host))
    }
}

impl Drop for HostLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn acquire_path(path: PathBuf) -> Result<HostLock> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let started = SystemTime::now();
    loop {
        match create_lock_file(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "pid={}", std::process::id());
                return Ok(HostLock { path, _file: file });
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                remove_stale_lock(&path);
                if started.elapsed().unwrap_or_default() > LOCK_TIMEOUT {
                    return Err(anyhow!("timed out waiting for lock {}", path.display()));
                }
                thread::sleep(RETRY_INTERVAL);
            }
            Err(err) => {
                return Err(err).with_context(|| format!("create lock {}", path.display()));
            }
        }
    }
}

fn create_lock_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().write(true).create_new(true).open(path)
}

fn remove_stale_lock(path: &Path) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    let Ok(modified) = metadata.modified() else {
        return;
    };
    if modified.elapsed().unwrap_or_default() > STALE_AFTER {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_path_uses_host_file_name() {
        // given
        let host = "https://querypie.example.com/";

        // when
        let path = paths::host_lock_file(host);

        // then
        assert!(path.ends_with("querypie.example.com.lock"));
    }
}

// 文件锁：`.herdr-worker/.lock`，create_new 原子争抢 + 超时等待 + 陈旧破坏。
// 单用户本地场景的朴素实现（无 fs2 依赖）；所有写者（TUI 写回 / act / sync）共用。

use anyhow::{bail, Result};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const STALE_AFTER: Duration = Duration::from_secs(30);

pub struct BoardLock {
    path: PathBuf,
}

impl BoardLock {
    pub fn acquire(root: &Path, timeout: Duration) -> Result<Self> {
        let path = root.join(".lock");
        let start = SystemTime::now();
        loop {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(_f) => return Ok(BoardLock { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    break_stale(&path);
                    if start.elapsed()? >= timeout {
                        bail!("板文件锁超时（{}）：{}", path.display(), timeout.as_millis());
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
}

fn break_stale(path: &Path) {
    if let Ok(meta) = fs::metadata(path) {
        if let Ok(modified) = meta.modified() {
            if modified
                .elapsed()
                .map(|e| e > STALE_AFTER)
                .unwrap_or(false)
            {
                let _ = fs::remove_file(path);
            }
        }
    }
}

impl Drop for BoardLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// 供测试与未来 act/sync 复用：在锁内执行 f
pub fn with_lock<T>(root: &Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
    let _lock = BoardLock::acquire(root, Duration::from_secs(5))?;
    f()
}

#[allow(dead_code)]
pub fn touch(path: &Path) {
    let _ = File::create(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_is_exclusive_and_released() {
        let dir = std::env::temp_dir().join(format!("herdr-worker-lock-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let a = BoardLock::acquire(&dir, Duration::from_millis(100)).unwrap();
        // 持锁期间第二个获取者应当超时
        assert!(BoardLock::acquire(&dir, Duration::from_millis(150)).is_err());
        drop(a);
        // 释放后立即可取
        assert!(BoardLock::acquire(&dir, Duration::from_millis(100)).is_ok());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn with_lock_serializes() {
        let dir = std::env::temp_dir().join(format!("herdr-worker-wl-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let n = with_lock(&dir, || Ok(41)).unwrap();
        assert_eq!(n, 41);
        fs::remove_dir_all(&dir).unwrap();
    }
}

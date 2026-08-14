use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 单次运行的日志会话，位于 `<base>/logs/<unix-timestamp>/`
pub struct Session {
    harness: Mutex<File>,
    gui: Mutex<File>,
}

fn timestamp(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn open_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

impl Session {
    pub fn create(base: &Path, pid: u32) -> io::Result<Self> {
        let timestamp = timestamp(SystemTime::now());
        let dir = base.join("logs").join(timestamp.to_string());
        fs::create_dir_all(&dir)?;
        let session = Self {
            harness: Mutex::new(open_file(&dir.join("harness.log"))?),
            gui: Mutex::new(open_file(&dir.join("gui.log"))?),
        };
        session.log_gui(&format!("session started (pid {pid}) at {timestamp}"));
        Ok(session)
    }

    /// harness 进程的原始输出，原样落盘，不做脱敏。
    pub fn log_harness(&self, line: &str) {
        let mut f = self.harness.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "{line}");
    }

    pub fn log_gui(&self, line: &str) {
        let timestamp = timestamp(SystemTime::now());
        let mut f = self.gui.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "[{timestamp}] {line}");
    }
}

/// 删除目录名 timestamp 早于 `older_than` 的日志会话。无法解析的目录不会被删除。
pub fn cleanup_old(base: &Path, older_than: Duration) -> io::Result<usize> {
    let logs_dir = base.join("logs");
    let cutoff = timestamp(SystemTime::now()).saturating_sub(older_than.as_secs());
    let mut removed = 0;
    let mut first_err = None;
    if let Ok(entries) = fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().is_ok_and(|file_type| file_type.is_dir()) {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Ok(created_at) = name.parse::<u64>() else {
                continue;
            };
            if created_at >= cutoff {
                continue;
            }
            match fs::remove_dir_all(entry.path()) {
                Ok(()) => removed += 1,
                Err(error) => {
                    let _ = first_err.get_or_insert(error);
                }
            }
        }
    }
    if let Some(error) = first_err {
        return Err(error);
    }
    Ok(removed)
}

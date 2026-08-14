use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use log::{LevelFilter, Metadata, Record};

/// 单次运行的日志会话，位于 `<base>/logs/<unix-timestamp>/`
pub struct Session {
    harness: Mutex<File>,
    gui: Mutex<File>,
}

/// 全局 logger：应用日志统一走 `log` crate 门面。
/// 格式 `time [level] target: message`，落盘到当前会话 gui.log，
/// debug 构建同时输出到 stderr 便于开发调试。
struct SessionLogger;

static ACTIVE_SESSION: OnceLock<Mutex<Option<Arc<Session>>>> = OnceLock::new();
static SESSION_LOGGER: SessionLogger = SessionLogger;

fn timestamp(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn open_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

impl Session {
    pub fn create(base: &Path) -> io::Result<Self> {
        let timestamp = timestamp(SystemTime::now());
        let dir = base.join("logs").join(timestamp.to_string());
        fs::create_dir_all(&dir)?;
        let session = Self {
            harness: Mutex::new(open_file(&dir.join("harness.log"))?),
            gui: Mutex::new(open_file(&dir.join("gui.log"))?),
        };
        Ok(session)
    }

    /// harness 进程的原始输出，原样落盘，不做脱敏。
    pub fn log_harness(&self, line: &str) {
        let mut f = self.harness.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "{line}");
    }

    /// logger 格式化后的标准日志行，直接落盘。
    fn write_gui(&self, line: &str) {
        let mut f = self.gui.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "{line}");
    }
}

/// 初始化全局 logger。重复调用安全（`set_logger` 仅首次生效）。
pub fn init_logging() {
    let _ = log::set_logger(&SESSION_LOGGER);
    log::set_max_level(if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    });
}

/// 注册当前会话，此后所有 `log::*` 调用写入该会话的 gui.log。
pub fn attach_session(session: Arc<Session>) {
    let mut slot = ACTIVE_SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    *slot = Some(session);
}

impl log::Log for SessionLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format!(
            "{} [{}] {}: {}",
            chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z"),
            record.level(),
            record.target(),
            record.args()
        );
        #[cfg(debug_assertions)]
        eprintln!("{line}");
        let session = ACTIVE_SESSION
            .get()
            .and_then(|m| m.lock().ok())
            .and_then(|slot| slot.clone());
        if let Some(session) = session {
            session.write_gui(&line);
        }
    }

    fn flush(&self) {}
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
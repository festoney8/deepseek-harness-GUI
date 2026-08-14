use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

/// 单次运行的日志会话，位于 <base>/logs/YYYY-MM-DD_HH-mm-ss-<pid>/
pub struct Session {
    pub dir: PathBuf,
    harness: Mutex<File>,
    gui: Mutex<File>,
}

/// 民用日期换算（Howard Hinnant 算法，UTC）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn utc_ymd_hms(now: SystemTime) -> (String, String) {
    let secs = now.duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let (y, mo, d) = civil_from_days(secs.div_euclid(86_400));
    let tod = secs.rem_euclid(86_400);
    let (h, mi, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    (format!("{y:04}-{mo:02}-{d:02}"), format!("{h:02}:{mi:02}:{s:02}"))
}

fn open_file(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

impl Session {
    pub fn create(base: &Path, pid: u32) -> io::Result<Self> {
        let now = SystemTime::now();
        let (date, time) = utc_ymd_hms(now);
        let dir = base.join("logs").join(format!("{date}_{}", time.replace(':', "-")));
        fs::create_dir_all(&dir)?;
        let session = Self {
            dir: dir.clone(),
            harness: Mutex::new(open_file(&dir.join("harness.log"))?),
            gui: Mutex::new(open_file(&dir.join("gui.log"))?),
        };
        session.log_gui(&format!("session started (pid {pid}) at {date} {time}"));
        Ok(session)
    }

    /// harness 进程的原始输出，原样落盘，不做脱敏。
    pub fn log_harness(&self, line: &str) {
        let mut f = self.harness.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "{line}");
    }

    pub fn log_gui(&self, line: &str) {
        let (_, time) = utc_ymd_hms(SystemTime::now());
        let mut f = self.gui.lock().unwrap_or_else(|e| e.into_inner());
        let _ = writeln!(f, "[{time}] {line}");
    }
}

/// 删除 logs 目录下早于 `older_than` 的会话目录。失败仅告警，由调用方决定是否阻断。
pub fn cleanup_old(base: &Path, older_than: Duration) -> io::Result<usize> {
    let logs_dir = base.join("logs");
    let cutoff = SystemTime::now() - older_than;
    let mut removed = 0;
    let mut first_err = None;
    if let Ok(entries) = fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            if !entry.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            match entry.metadata().and_then(|m| m.modified()) {
                Ok(modified) if modified < cutoff => match fs::remove_dir_all(entry.path()) {
                    Ok(()) => removed += 1,
                    Err(e) => {
                        let _ = first_err.get_or_insert(e);
                    }
                },
                _ => {}
            }
        }
    }
    if let Some(e) = first_err {
        return Err(e);
    }
    Ok(removed)
}
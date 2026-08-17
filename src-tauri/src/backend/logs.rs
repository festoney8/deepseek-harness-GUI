use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::BackendError;

/// 保存本次应用启动所使用日志目录的共享状态。
#[derive(Debug, Clone)]
pub(crate) struct LogState {
    /// 本次启动日志目录的绝对路径。
    pub session_dir: PathBuf,
}

/// 在应用日志目录下创建本次启动的独立日志目录。
pub(crate) fn create_session_log_dir(app_log_dir: &Path) -> Result<PathBuf, BackendError> {
    todo!()
}

/// 使用系统文件管理器打开本次启动日志目录。
pub(crate) async fn open_logs(app: &AppHandle, state: &LogState) -> Result<(), BackendError> {
    todo!()
}

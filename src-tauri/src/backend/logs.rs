use std::path::{Path, PathBuf};

use tauri::AppHandle;

use super::BackendError;

/// 创建只输出到终端的 Tauri 日志插件。
pub(crate) fn create_logger() -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::new().targets([tauri_plugin_log::Target::new(
        tauri_plugin_log::TargetKind::Stdout,
    )])
}

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

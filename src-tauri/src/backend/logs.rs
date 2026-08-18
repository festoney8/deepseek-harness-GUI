use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use log::error;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use super::BackendError;

/// 创建写往终端和本次启动日志目录的 Tauri 日志插件。
pub(crate) fn create_logger(session_dir: &Path) -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::new().targets([
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
        tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Folder {
            path: session_dir.to_path_buf(),
            file_name: Some(String::from("app")),
        }),
    ])
}

/// 保存本次应用启动对应的日志目录。
pub(crate) struct LogState {
    /// 本次启动日志目录的绝对路径。
    pub session_dir: PathBuf,
}

/// 在应用日志目录下创建本次启动的独立日志目录。
pub(crate) fn create_session_log_dir(app_log_dir: &Path) -> Result<PathBuf, BackendError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| BackendError::LogDirCreateFailed)?
        .as_secs();
    let session_dir = app_log_dir.join(timestamp.to_string());
    std::fs::create_dir_all(&session_dir).map_err(|source| {
        error!("failed to create session log dir {session_dir:?}: {source:?}");
        BackendError::LogDirCreateFailed
    })?;
    Ok(session_dir)
}

/// 使用系统文件管理器打开本次启动日志目录。
pub(crate) async fn open_logs(app: &AppHandle, state: &LogState) -> Result<(), BackendError> {
    let path = state.session_dir.to_string_lossy().into_owned();
    app.opener().open_path(path, None::<&str>).map_err(|source| {
        error!(
            "failed to open session log dir {:?}: {source:?}",
            state.session_dir
        );
        BackendError::OpenLogsFailed
    })
}
use tauri::{AppHandle, State};

use crate::backend::{self, BackendError, HarnessState, LogState};

/// 前后端 IPC 边界使用的稳定结构化错误。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IpcError {
    /// 供前端分支处理的稳定错误码。
    pub code: String,
    /// 可直接展示给用户的错误消息。
    pub message: String,
}

impl IpcError {
    /// 将内部业务错误转换为统一的 IPC 错误。
    fn from_backend(error: BackendError) -> Self {
        todo!()
    }
}

/// 启动本地 DSH 服务。
#[tauri::command]
pub(crate) async fn start_dsh(
    port: u16,
    app: AppHandle,
    state: State<'_, HarnessState>,
) -> Result<String, IpcError> {
    todo!()
}

/// 停止当前受控的 DSH 服务。
#[tauri::command]
pub(crate) async fn stop_dsh(state: State<'_, HarnessState>) -> Result<(), IpcError> {
    todo!()
}

/// 探测并连接远程 DSH 服务。
#[tauri::command]
pub(crate) async fn connect_remote(host: String, port: u16) -> Result<String, IpcError> {
    todo!()
}

/// 打开本次应用启动对应的日志目录。
#[tauri::command]
pub(crate) async fn open_logs(app: AppHandle, state: State<'_, LogState>) -> Result<(), IpcError> {
    todo!()
}

/// 隐藏主窗口到系统托盘。
#[tauri::command]
pub(crate) async fn hide_to_tray(app: AppHandle) -> Result<(), IpcError> {
    todo!()
}

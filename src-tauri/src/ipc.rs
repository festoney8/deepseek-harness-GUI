use tauri::{AppHandle, State};

use crate::backend::{self, BackendError, HarnessState, LogState};

/// 前后端 IPC 边界使用的稳定结构化错误
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IpcError {
    /// 供前端分支处理的稳定错误码
    pub code: String,
    /// 可直接展示给用户的错误消息
    pub message: String,
}

impl IpcError {
    /// 将内部业务错误转换为统一的 IPC 错误
    fn from_backend(error: BackendError) -> Self {
        let (code, message) = match &error {
            // 内部错误完整链写入日志，不向前端暴露底层细节
            BackendError::InvalidTimeout | BackendError::Platform(_) => {
                log::error!("ipc internal error: {error:?}");
                ("internal_error", String::from("内部错误，请查看日志"))
            }
            BackendError::Tray(_) => {
                log::error!("ipc tray error: {error:?}");
                ("tray_error", error.to_string())
            }
            BackendError::InvalidHost => ("invalid_host", error.to_string()),
            BackendError::InvalidProtocol => ("invalid_protocol", error.to_string()),
            BackendError::InvalidPort => ("invalid_port", error.to_string()),
            BackendError::ServiceUnavailable => ("service_unavailable", error.to_string()),
            BackendError::PortOccupied => ("port_occupied", error.to_string()),
            BackendError::OperationInProgress => ("operation_in_progress", error.to_string()),
            BackendError::DshAlreadyRunning => ("dsh_already_running", error.to_string()),
            BackendError::ProcessNotRunning => ("process_not_running", error.to_string()),
            BackendError::DshSpawnFailed => ("dsh_spawn_failed", error.to_string()),
            BackendError::DshStartTimeout => ("dsh_start_timeout", error.to_string()),
            BackendError::DshExitedEarly => ("dsh_exited_early", error.to_string()),
            BackendError::OpenLogsFailed => ("open_logs_failed", error.to_string()),
            BackendError::LogDirCreateFailed => ("log_dir_create_failed", error.to_string()),
            BackendError::WindowResourceMissing => ("window_resource_missing", error.to_string()),
        };

        IpcError {
            code: code.to_string(),
            message,
        }
    }
}

/// 启动本地 DSH 服务
#[tauri::command]
pub(crate) async fn start_dsh(
    port: u16,
    app: AppHandle,
    state: State<'_, HarnessState>,
) -> Result<String, IpcError> {
    backend::start_dsh(port, &app, &state)
        .await
        .map_err(IpcError::from_backend)
}

/// 停止当前受控的 DSH 服务
#[tauri::command]
pub(crate) async fn stop_dsh(state: State<'_, HarnessState>) -> Result<(), IpcError> {
    backend::stop_dsh(&state)
        .await
        .map_err(IpcError::from_backend)
}

/// 探测并连接远程 DSH 服务
#[tauri::command]
pub(crate) async fn connect_remote(
    protocol: String,
    host: String,
    port: u16,
) -> Result<String, IpcError> {
    backend::connect_remote(protocol, host, port)
        .await
        .map_err(IpcError::from_backend)
}

/// 打开本次应用启动对应的日志目录
#[tauri::command]
pub(crate) async fn open_logs(app: AppHandle, state: State<'_, LogState>) -> Result<(), IpcError> {
    backend::open_logs(&app, &state)
        .await
        .map_err(IpcError::from_backend)
}

/// 隐藏主窗口到系统托盘
#[tauri::command]
pub(crate) async fn hide_to_tray(app: AppHandle) -> Result<(), IpcError> {
    backend::hide_to_tray(&app).map_err(IpcError::from_backend)
}
use std::sync::{atomic::AtomicBool, Arc};

use tauri::AppHandle;

use super::{BackendError, HarnessState};

/// 防止托盘退出流程被重复触发的共享状态。
#[derive(Debug)]
pub(crate) struct ExitState {
    /// 标记应用是否已经进入退出流程。
    pub exiting: AtomicBool,
}

/// 注册系统托盘菜单及窗口生命周期事件。
pub(crate) fn register_tray(
    app: &AppHandle,
    harness_state: Arc<HarnessState>,
    exit_state: Arc<ExitState>,
) -> Result<(), BackendError> {
    todo!()
}

/// 隐藏主窗口并保持应用后台运行。
pub(crate) fn hide_to_tray(app: &AppHandle) -> Result<(), BackendError> {
    todo!()
}

/// 显示主窗口并将其恢复到前台。
pub(crate) fn show_main_window(app: &AppHandle) -> Result<(), BackendError> {
    todo!()
}

/// 停止受控进程后退出 Tauri 应用。
pub(crate) async fn quit_app(
    app: AppHandle,
    harness_state: Arc<HarnessState>,
    exit_state: Arc<ExitState>,
) -> Result<(), BackendError> {
    todo!()
}

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use log::error;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, WindowEvent, WebviewWindow,
};

use super::{stop_dsh, BackendError, HarnessPhase, HarnessState};

/// 防止托盘退出流程被重复触发的共享状态。
pub(crate) struct ExitState {
    /// 标记应用是否已经进入退出流程。
    pub exiting: AtomicBool,
}

impl std::fmt::Debug for ExitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExitState")
            .field("exiting", &self.exiting.load(Ordering::Relaxed))
            .finish()
    }
}

/// 注册系统托盘菜单及窗口生命周期事件。
pub(crate) fn register_tray(
    app: &AppHandle,
    harness_state: Arc<HarnessState>,
    exit_state: Arc<ExitState>,
) -> Result<(), BackendError> {
    let show_item = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or(BackendError::WindowResourceMissing)?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("DeepSeek Harness GUI")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                if let Err(error) = show_main_window(app) {
                    error!("tray show failed: {error:?}");
                }
            }
            "quit" => {
                let app = app.clone();
                let harness_state = Arc::clone(&harness_state);
                let exit_state = Arc::clone(&exit_state);
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = quit_app(app, harness_state, exit_state).await {
                        error!("tray quit failed: {error:?}");
                    }
                });
            }
            _ => {}
        })
        .build(app)?;

    let close_app = app.clone();
    if let Some(window) = app.get_webview_window("main") {
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                if let Err(error) = hide_to_tray(&close_app) {
                    error!("close to tray failed: {error:?}");
                }
            }
        });
    }

    Ok(())
}

/// 隐藏主窗口并保持应用后台运行。
pub(crate) fn hide_to_tray(app: &AppHandle) -> Result<(), BackendError> {
    main_window(app)?.hide()?;
    Ok(())
}

/// 显示主窗口并将其恢复到前台。
pub(crate) fn show_main_window(app: &AppHandle) -> Result<(), BackendError> {
    let window = main_window(app)?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}

/// 停止受控进程后退出 Tauri 应用。
///
/// 重复调用会被 `ExitState` 幂等化；若 DSH 仍在运行，先执行内部停止并清理进程树，
/// 无论清理结果如何都结束应用，失败只记录日志。
pub(crate) async fn quit_app(
    app: AppHandle,
    harness_state: Arc<HarnessState>,
    exit_state: Arc<ExitState>,
) -> Result<(), BackendError> {
    if exit_state.exiting.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let has_dsh = {
        let lifecycle = harness_state.lifecycle.read().await;
        lifecycle.process.is_some() || lifecycle.phase != HarnessPhase::Stopped
    };
    if has_dsh {
        if let Err(error) = stop_dsh(&harness_state).await {
            error!("quit: stop dsh failed: {error:?}");
        }
    }

    app.exit(0);
    Ok(())
}

/// 返回主窗口，缺失时报告统一的窗口资源错误。
fn main_window(app: &AppHandle) -> Result<WebviewWindow, BackendError> {
    app.get_webview_window("main")
        .ok_or(BackendError::WindowResourceMissing)
}
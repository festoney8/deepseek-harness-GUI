//! webview ↔ Rust 通讯接口清单。
//!
//! Commands（前端 invoke → Rust，均为薄包装转发 Supervisor）：
//! - get_state() -> Snapshot            全量快照（前端 initRuntime 初始化基准拉取）
//! - check_env()                        检测 node/npm 版本
//! - check_version()                    远端（官方源+镜像源）与本地 dsh 版本查询
//! - install_dsh(mirror: bool)          安装/更新 dsh（false 官方源，true 镜像源）
//! - start_server()                     启动 harness（自动选空闲端口并轮询就绪）
//! - open_log_dir()                     用默认文件管理器打开本次运行日志目录
//! - exit_app()                         终止 harness 进程树后退出应用
//! - hide_to_tray()                     隐藏主窗口（关窗进托盘流程的窗口隐藏动作）
//!
//! Events（Rust emit → 前端 listen）：
//! - runtime-state: Snapshot            状态变更推送
//! - close-requested: ()                窗口关闭拦截通知

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::protocol::Snapshot;
use crate::runtime::{shutdown, Supervisor};

/// 全量快照：前端 initRuntime 的初始化基准拉取（事件为增量推送，必须有基准值）。
#[tauri::command]
pub fn get_state(sup: tauri::State<'_, Supervisor>) -> Snapshot {
    sup.snapshot()
}

/// 检测 node/npm 版本，刷新快照的 node/npm 格子。
#[tauri::command]
pub fn check_env(sup: tauri::State<'_, Supervisor>) {
    sup.check_env();
}

/// 查询远端（官方源+镜像源）与本地 dsh 版本，刷新版本格子。
#[tauri::command]
pub fn check_version(sup: tauri::State<'_, Supervisor>) {
    sup.check_version();
}

/// 安装/更新 dsh：mirror 为 true 时使用镜像 registry。
#[tauri::command]
pub fn install_dsh(sup: tauri::State<'_, Supervisor>, mirror: bool) {
    sup.install(mirror);
}

/// 启动 harness：自动选择空闲端口并轮询 HTTP 就绪。
#[tauri::command]
pub fn start_server(sup: tauri::State<'_, Supervisor>) {
    sup.start();
}

/// 用系统默认文件管理器打开本次运行的日志目录（harness.log + gui.log）。
#[tauri::command]
pub fn open_log_dir(app: AppHandle, sup: tauri::State<'_, Supervisor>) {
    if let Some(dir) = sup.log_dir() {
        let _ = app.opener().open_path(dir.display().to_string(), None::<&str>);
    }
}

/// 终止 harness 进程树后退出应用（唯一关闭入口）。
#[tauri::command]
pub fn exit_app(app: AppHandle) {
    shutdown(&app);
}

/// 隐藏主窗口（关窗进托盘流程的窗口隐藏动作）。
#[tauri::command]
pub fn hide_to_tray(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}
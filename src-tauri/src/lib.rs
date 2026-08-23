mod backend;
mod ipc;
mod platform;

use std::sync::{atomic::AtomicBool, Arc};

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// 启动 Tauri 应用并完成后端初始化
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 修复 GUI 应用 PATH，必须在任何子进程启动之前
    let path_fix = fix_path_env::fix();

    let exit_state = Arc::new(backend::ExitState {
        exiting: AtomicBool::new(false),
    });

    tauri::Builder::default()
        // 单例插件必须注册为第一个插件;再次启动时唤起已有实例的主窗口
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = backend::show_main_window(app) {
                log::error!("single instance show failed: {error:?}");
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(backend::create_harness_state())
        .manage(backend::create_webview_state())
        .invoke_handler(tauri::generate_handler![
            ipc::start_dsh,
            ipc::stop_dsh,
            ipc::connect_remote,
            ipc::open_logs,
            ipc::hide_to_tray,
            ipc::create_webview_with_url,
            ipc::activate_webview_tab,
            ipc::close_webview_tab,
            ipc::hide_all_webview_tabs,
        ])
        .setup(move |app| {
            // 解析平台标准日志目录，并创建本次启动的独立会话目录
            let app_log_dir = app.path().app_log_dir()?;
            // 启动时清理 7 天之前的会话日志目录
            backend::cleanup_old_logs(&app_log_dir, 7);
            let session_dir = backend::create_session_log_dir(&app_log_dir)?;
            // 日志插件只注册一次，文件 target 指向会话目录
            app.handle()
                .plugin(backend::create_logger(&session_dir).build())?;
            // 日志插件就绪后补记 PATH 修复失败
            if let Err(error) = path_fix {
                log::error!("PATH fix failed: {error}");
            }
            app.manage(backend::LogState { session_dir });

            // 托盘事件闭包需要与 IPC 状态共享同一组共享句柄
            let harness_state = {
                let state = app.state::<backend::HarnessState>();
                Arc::new(backend::HarnessState {
                    operation: state.operation.clone(),
                    lifecycle: state.lifecycle.clone(),
                })
            };
            let exit_state = Arc::clone(&exit_state);
            // 主窗口必须先于托盘注册存在，托盘依赖 get_window("main")
            build_main_window(app)?;
            let webview_state = app.state::<backend::WebviewState>().inner().clone();
            backend::register_resize_handler(app.handle(), webview_state)?;
            backend::register_tray(app.handle(), harness_state, exit_state)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 主窗口由 Rust 创建以挂载 on_download（config 定义的窗口无法挂载）
///
/// on_download 用于拦截 webview 及其中 iframe 的下载操作，让用户选择保存位置
fn build_main_window(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DeepSeek Harness")
        .inner_size(1200.0, 800.0)
        .min_inner_size(1200.0, 800.0)
        .decorations(false)
        .center()
        .resizable(true)
        .maximizable(true)
        .disable_drag_drop_handler()
        .on_download(backend::handle_download)
        .build()?;
    Ok(())
}

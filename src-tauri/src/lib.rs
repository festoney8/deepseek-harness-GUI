mod backend;
mod ipc;
mod platform;

use std::sync::{atomic::AtomicBool, Arc};

use tauri::{webview::DownloadEvent, Manager, WebviewUrl, WebviewWindowBuilder};

/// 启动 Tauri 应用并完成后端初始化
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 修复 GUI 应用 PATH，必须在任何子进程启动之前
    let path_fix = fix_path_env::fix();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(backend::create_harness_state())
        .invoke_handler(tauri::generate_handler![
            ipc::start_dsh,
            ipc::stop_dsh,
            ipc::connect_remote,
            ipc::open_logs,
            ipc::hide_to_tray,
        ])
        .setup(|app| {
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
            let exit_state = Arc::new(backend::ExitState {
                exiting: AtomicBool::new(false),
            });
            // 主窗口必须先于托盘注册存在，托盘依赖 get_webview_window("main")
            build_main_window(app)?;
            backend::register_tray(app.handle(), harness_state, exit_state)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 从下载 URL 推断默认文件名（路径最后一段 percent-decode），失败时回退为 "download"
fn default_file_name(url: &tauri::Url) -> String {
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|name| !name.is_empty())
        .map(|name| {
            percent_encoding::percent_decode_str(name)
                .decode_utf8_lossy()
                .into_owned()
        })
        .unwrap_or_else(|| "download".into())
}

/// 主窗口由 Rust 创建以挂载 on_download（config 定义的窗口无法挂载）
///
/// on_download 用于拦截 webview 及其中 iframe 的下载操作，让用户选择保存位置
fn build_main_window(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DeepSeek Harness")
        .inner_size(1200.0, 800.0)
        .min_inner_size(1200.0, 800.0)
        .center()
        .resizable(true)
        .maximizable(true)
        .zoom_hotkeys_enabled(true)
        .disable_drag_drop_handler()
        .on_download(|webview, event| match event {
            DownloadEvent::Requested { url, destination } => {
                match rfd::FileDialog::new()
                    .set_parent(&webview.window())
                    .set_file_name(default_file_name(&url))
                    .save_file()
                {
                    Some(path) => {
                        *destination = path;
                        true
                    }
                    None => {
                        log::debug!("download cancelled by user: {url}");
                        false // 用户取消 → 取消下载
                    }
                }
            }
            DownloadEvent::Finished { url, path, success } => {
                log::info!("download finished: url={url}, path={path:?}, success={success}");
                true
            }
            _ => true,
        })
        .build()?;
    Ok(())
}

mod ipc;
mod logs;
mod os;
mod protocol;
mod runtime;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::DownloadEvent,
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use ipc::{check_env, check_version, exit_app, get_output, get_state, hide_to_tray, install_dsh, start_server};
use protocol::EVENT_CLOSE_REQUESTED;
use runtime::{show_main, shutdown, Supervisor};

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
fn build_main_window(app: &tauri::App) -> tauri::Result<()> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("DeepSeek Harness")
        .inner_size(1200.0, 800.0)
        .min_inner_size(1200.0, 800.0)
        .center()
        .decorations(false)
        .shadow(true)
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

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::with_id("main-tray")
        .icon(
            app.default_window_icon()
                .expect("configured app icon")
                .clone(),
        )
        .tooltip("DeepSeek Harness")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main(app),
            "quit" => shutdown(app),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    os::fix_env_path();
    // Workaround for WebKitGTK DMABUF renderer crashes on NVIDIA/Wayland
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    logs::init_logging();
    log::info!("starting deepseek-harness-gui");
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log::info!("second instance detected, showing main window");
            show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(Supervisor::new())
        .setup(|app| {
            let sup = app.state::<Supervisor>();
            let base = app
                .path()
                .app_local_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("deepseek-harness-gui"));
            sup.spawn_worker(app.handle().clone(), base);
            build_main_window(app)?;
            build_tray(app)?;
            log::info!("main window and tray ready");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                log::debug!("close requested for window {}", window.label());
                api.prevent_close();
                let _ = window.emit(EVENT_CLOSE_REQUESTED, ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            get_output,
            check_env,
            check_version,
            install_dsh,
            start_server,
            exit_app,
            hide_to_tray
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

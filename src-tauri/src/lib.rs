mod logs;
mod runtime;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

use runtime::{show_main, shutdown, Snapshot, Supervisor};

#[tauri::command]
fn get_state(sup: tauri::State<'_, Supervisor>) -> Snapshot {
    sup.snapshot()
}

#[tauri::command]
fn retry_start(sup: tauri::State<'_, Supervisor>) {
    sup.retry();
}

#[tauri::command]
fn cancel_start(sup: tauri::State<'_, Supervisor>) {
    sup.cancel();
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    shutdown(&app);
}

#[tauri::command]
fn hide_to_tray(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn open_log_dir(sup: tauri::State<'_, Supervisor>) -> Result<(), String> {
    match sup.session_dir() {
        Some(dir) => tauri_plugin_opener::open_path(dir, None::<&str>).map_err(|e| e.to_string()),
        None => Err("日志目录尚未创建".into()),
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().expect("configured app icon").clone())
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
                show_main(&tray.app_handle());
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
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(&app);
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
            build_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.emit("close-requested", ());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            retry_start,
            cancel_start,
            exit_app,
            hide_to_tray,
            open_log_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
mod backend;
mod ipc;
mod platform;

/// 启动 Tauri 应用并完成后端初始化。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(backend::create_logger().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

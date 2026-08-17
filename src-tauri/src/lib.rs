mod backend;
mod ipc;
mod platform;

/// 启动 Tauri 应用并完成后端初始化。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    todo!()
}

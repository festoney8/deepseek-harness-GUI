#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// 启动桌面应用入口。
fn main() {
    deepseek_harness_gui_lib::run();
}

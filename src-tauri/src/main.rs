#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// 启动桌面应用入口
fn main() {
    let _ = fix_path_env::fix(); // fix path

    deepseek_harness_gui_lib::run();
}

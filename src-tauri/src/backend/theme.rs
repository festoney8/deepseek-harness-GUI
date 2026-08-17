use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::RwLock;

use super::BackendError;

/// 缓存最近一次成功解析主题值的共享状态。
#[derive(Debug)]
pub(crate) struct ThemeState {
    /// 当前有效主题；尚未读取成功时为空。
    pub current: RwLock<Option<String>>,
}

/// 创建当前主题共享缓存。
pub(crate) fn create_theme_state() -> ThemeState {
    todo!()
}

/// 启动主题配置文件轮询与文件监听任务。
pub(crate) async fn start_theme_watcher(
    app: AppHandle,
    state: Arc<ThemeState>,
) -> Result<(), BackendError> {
    todo!()
}

/// 获取最近一次成功解析的主题值。
pub(crate) async fn get_curr_theme(state: &ThemeState) -> Result<String, BackendError> {
    todo!()
}

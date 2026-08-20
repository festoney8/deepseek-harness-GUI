use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tauri::{
    webview::{DownloadEvent, Webview},
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};

use super::BackendError;

const URL_WINDOW_TITLE: &str = "DeepSeek Harness";
const URL_WINDOW_WIDTH: f64 = 1200.0;
const URL_WINDOW_HEIGHT: f64 = 800.0;

#[derive(Debug, Default)]
struct UrlWindowRegistry {
    next_id: u64,
    windows: HashMap<String, String>,
}

/// 维护规范化 URL 到动态 Webview 窗口的映射。
///
/// 创建锁覆盖查找、显示和创建全过程，确保并发请求同一个 URL 时只会创建一个窗口。
#[derive(Debug, Clone)]
pub(crate) struct UrlWindowState {
    registry: Arc<Mutex<UrlWindowRegistry>>,
}

/// 创建动态 URL 窗口的共享状态。
pub(crate) fn create_url_window_state() -> UrlWindowState {
    UrlWindowState {
        registry: Arc::new(Mutex::new(UrlWindowRegistry::default())),
    }
}

/// 为 Webview 统一处理下载请求，并使用系统文件保存对话框选择目标路径。
pub(crate) fn handle_download<R: Runtime>(webview: Webview<R>, event: DownloadEvent<'_>) -> bool {
    match event {
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
                    false
                }
            }
        }
        DownloadEvent::Finished { url, path, success } => {
            log::info!("download finished: url={url}, path={path:?}, success={success}");
            true
        }
        _ => true,
    }
}

/// 创建或显示一个直接加载外部 HTTP(S) URL 的 Webview 窗口。
pub(crate) fn create_window_with_url(
    app: &AppHandle,
    url: String,
    state: &UrlWindowState,
) -> Result<(), BackendError> {
    let parsed_url = parse_window_url(&url)?;
    let url_key = parsed_url.as_str().to_owned();
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;

    if let Some(label) = registry.windows.get(&url_key).cloned() {
        if let Some(window) = app.get_webview_window(&label) {
            window.unminimize().map_err(BackendError::Window)?;
            window.show().map_err(BackendError::Window)?;
            window.set_focus().map_err(BackendError::Window)?;
            return Ok(());
        }
        registry.windows.remove(&url_key);
    }

    let label = next_window_label(&mut registry);
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(parsed_url))
        .title(URL_WINDOW_TITLE)
        .inner_size(URL_WINDOW_WIDTH, URL_WINDOW_HEIGHT)
        .min_inner_size(URL_WINDOW_WIDTH, URL_WINDOW_HEIGHT)
        .center()
        .resizable(true)
        .maximizable(true)
        .zoom_hotkeys_enabled(true)
        .disable_drag_drop_handler()
        .on_download(handle_download)
        .build()
        .map_err(BackendError::Window)?;

    let registry_for_destroy = Arc::clone(&state.registry);
    let key_for_destroy = url_key.clone();
    let label_for_destroy = label.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            if let Ok(mut registry) = registry_for_destroy.lock() {
                if registry.windows.get(&key_for_destroy) == Some(&label_for_destroy) {
                    registry.windows.remove(&key_for_destroy);
                }
            }
        }
    });

    registry.windows.insert(url_key, label);
    Ok(())
}

fn parse_window_url(input: &str) -> Result<tauri::Url, BackendError> {
    let parsed = input
        .parse::<tauri::Url>()
        .map_err(|_| BackendError::InvalidWindowUrl)?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(BackendError::InvalidWindowUrl);
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(BackendError::InvalidWindowUrl);
    }
    if parsed.host().is_none() {
        return Err(BackendError::InvalidWindowUrl);
    }

    Ok(parsed)
}

fn next_window_label(registry: &mut UrlWindowRegistry) -> String {
    let id = registry.next_id;
    registry.next_id = registry.next_id.wrapping_add(1);
    format!("url-window-{id}")
}

/// 从下载 URL 推断默认文件名（路径最后一段 percent-decode），失败时回退为 "download"。
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

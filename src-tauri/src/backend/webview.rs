use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tauri::{
    webview::{DownloadEvent, NewWindowResponse, Webview, WebviewBuilder},
    AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;

use super::BackendError;

const CHILD_WEBVIEW_TOP: f64 = 36.0;

#[derive(Debug, Clone)]
struct ChildTab {
    label: String,
    url: String,
}

#[derive(Debug, Default)]
struct UrlWindowRegistry {
    next_id: u64,
    tabs: HashMap<String, ChildTab>,
}

/// 可由 IPC 返回给前端的 child WebView 标签信息。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebviewTab {
    pub label: String,
    pub url: String,
    pub display_name: String,
    pub reused: bool,
}

/// 维护 DSH origin 到 child WebView 标签的映射。
#[derive(Debug, Clone)]
pub(crate) struct UrlWindowState {
    registry: Arc<Mutex<UrlWindowRegistry>>,
}

#[derive(Debug, Clone)]
struct WebviewOrigin {
    scheme: String,
    host: String,
    port: Option<u16>,
}

impl WebviewOrigin {
    fn from_url(url: &tauri::Url) -> Result<Self, BackendError> {
        Ok(Self {
            scheme: url.scheme().to_owned(),
            host: url
                .host_str()
                .ok_or(BackendError::InvalidWindowUrl)?
                .to_owned(),
            port: url.port_or_known_default(),
        })
    }

    fn matches(&self, url: &tauri::Url) -> bool {
        self.scheme == url.scheme()
            && self.host == url.host_str().unwrap_or_default()
            && self.port == url.port_or_known_default()
    }

    fn key(&self) -> String {
        format!("{}://{}:{:?}", self.scheme, self.host, self.port)
    }
}

/// 创建动态 child WebView 的共享状态。
pub(crate) fn create_url_window_state() -> UrlWindowState {
    UrlWindowState {
        registry: Arc::new(Mutex::new(UrlWindowRegistry::default())),
    }
}

/// 为主窗口和所有 child WebView 统一处理下载请求。
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

/// 创建或激活一个直接加载外部 HTTP(S) URL 的 child WebView。
pub(crate) fn create_window_with_url(
    app: &AppHandle,
    url: String,
    state: &UrlWindowState,
) -> Result<WebviewTab, BackendError> {
    let parsed_url = parse_window_url(&url)?;
    let origin = WebviewOrigin::from_url(&parsed_url)?;
    let origin_key = origin.key();
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;

    if let Some(tab) = registry.tabs.get(&origin_key).cloned() {
        if app.get_webview(&tab.label).is_some() {
            activate_webview(app, &registry, &tab.label)?;
            return Ok(tab_info(&tab, true));
        }
        registry.tabs.remove(&origin_key);
    }

    let label = next_window_label(&mut registry);
    let parent = app
        .get_window("main")
        .ok_or(BackendError::WindowResourceMissing)?;
    let size = parent
        .inner_size()
        .map_err(BackendError::Window)?
        .to_logical::<f64>(parent.scale_factor().map_err(BackendError::Window)?);
    let content_height = (size.height - CHILD_WEBVIEW_TOP).max(1.0);
    let builder = build_child_webview(label.clone(), parsed_url.clone(), origin, app.clone());

    parent
        .add_child(
            builder,
            LogicalPosition::new(0.0, CHILD_WEBVIEW_TOP),
            LogicalSize::new(size.width, content_height),
        )
        .map_err(BackendError::Window)?;

    let tab = ChildTab {
        label,
        url: parsed_url.to_string(),
    };
    registry.tabs.insert(origin_key, tab.clone());
    activate_webview(app, &registry, &tab.label)?;
    Ok(tab_info(&tab, false))
}

/// 激活一个 child WebView，并隐藏同一主窗口中的其他 child WebView。
pub(crate) fn activate_window_tab(
    app: &AppHandle,
    label: String,
    state: &UrlWindowState,
) -> Result<(), BackendError> {
    let registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    if !registry.tabs.values().any(|tab| tab.label == label) {
        return Err(BackendError::ChildWebviewNotFound);
    }
    activate_webview(app, &registry, &label)
}

/// 关闭一个 child WebView 并移除其注册信息。
pub(crate) fn close_window_tab(
    app: &AppHandle,
    label: String,
    state: &UrlWindowState,
) -> Result<(), BackendError> {
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    let key = registry
        .tabs
        .iter()
        .find(|(_, tab)| tab.label == label)
        .map(|(key, _)| key.clone())
        .ok_or(BackendError::ChildWebviewNotFound)?;
    let webview = app
        .get_webview(&label)
        .ok_or(BackendError::ChildWebviewNotFound)?;
    webview.close().map_err(BackendError::Window)?;
    registry.tabs.remove(&key);
    Ok(())
}

/// 隐藏所有 child WebView。
pub(crate) fn hide_all_window_tabs(
    app: &AppHandle,
    state: &UrlWindowState,
) -> Result<(), BackendError> {
    let registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    for tab in registry.tabs.values() {
        if let Some(webview) = app.get_webview(&tab.label) {
            webview.hide().map_err(BackendError::Window)?;
        }
    }
    Ok(())
}

/// 注册主窗口 resize 监听，保持 child WebView 使用固定的顶部偏移。
pub(crate) fn register_resize_handler(
    app: &AppHandle,
    state: UrlWindowState,
) -> Result<(), BackendError> {
    let window = app
        .get_window("main")
        .ok_or(BackendError::WindowResourceMissing)?;
    let app_handle = app.clone();
    window.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::Resized { .. } | WindowEvent::ScaleFactorChanged { .. }
        ) {
            if let Err(error) = resize_child_webviews(&app_handle, &state) {
                log::error!("resize child webviews failed: {error:?}");
            }
        }
    });
    Ok(())
}

fn resize_child_webviews(app: &AppHandle, state: &UrlWindowState) -> Result<(), BackendError> {
    let window = app
        .get_window("main")
        .ok_or(BackendError::WindowResourceMissing)?;
    let scale_factor = window.scale_factor().map_err(BackendError::Window)?;
    let size = window
        .inner_size()
        .map_err(BackendError::Window)?
        .to_logical::<f64>(scale_factor);
    let content_height = (size.height - CHILD_WEBVIEW_TOP).max(1.0);
    let registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;

    for tab in registry.tabs.values() {
        if let Some(webview) = app.get_webview(&tab.label) {
            webview
                .set_position(LogicalPosition::new(0.0, CHILD_WEBVIEW_TOP))
                .map_err(BackendError::Window)?;
            webview
                .set_size(LogicalSize::new(size.width, content_height))
                .map_err(BackendError::Window)?;
        }
    }
    Ok(())
}

fn build_child_webview(
    label: String,
    url: tauri::Url,
    origin: WebviewOrigin,
    app: AppHandle,
) -> WebviewBuilder<tauri::Wry> {
    let navigation_app = app.clone();
    let new_window_app = app;
    WebviewBuilder::new(label, WebviewUrl::External(url))
        .on_navigation(move |target| {
            if origin.matches(target) {
                return true;
            }
            if let Err(error) = navigation_app
                .opener()
                .open_url(target.as_str(), None::<&str>)
            {
                log::error!("open external navigation failed: url={target}, error={error}");
            }
            false
        })
        .on_new_window(move |target, _| {
            if let Err(error) = new_window_app
                .opener()
                .open_url(target.as_str(), None::<&str>)
            {
                log::error!("open external new window failed: url={target}, error={error}");
            }
            NewWindowResponse::Deny
        })
        .on_download(handle_download)
        .disable_drag_drop_handler()
        .zoom_hotkeys_enabled(true)
}

fn activate_webview(
    app: &AppHandle,
    registry: &UrlWindowRegistry,
    label: &str,
) -> Result<(), BackendError> {
    let target = app
        .get_webview(label)
        .ok_or(BackendError::ChildWebviewNotFound)?;
    for tab in registry.tabs.values() {
        if let Some(webview) = app.get_webview(&tab.label) {
            if tab.label == label {
                webview.show().map_err(BackendError::Window)?;
            } else {
                webview.hide().map_err(BackendError::Window)?;
            }
        }
    }
    target.set_focus().map_err(BackendError::Window)
}

fn tab_info(tab: &ChildTab, reused: bool) -> WebviewTab {
    let parsed = tab.url.parse::<tauri::Url>().ok();
    let display_name = parsed
        .as_ref()
        .and_then(|url| {
            let host = url.host_str()?;
            let port = url.port_or_known_default()?;
            Some(format!("{host}:{port}"))
        })
        .unwrap_or_else(|| tab.url.clone());
    WebviewTab {
        label: tab.label.clone(),
        url: tab.url.clone(),
        display_name,
        reused,
    }
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

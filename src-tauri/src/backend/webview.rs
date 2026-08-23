use std::{
    collections::{HashMap, HashSet},
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
struct WebviewRegistry {
    next_id: u64,
    tabs: HashMap<WebviewOrigin, ChildTab>,
    creating: HashSet<WebviewOrigin>,
}

/// 可由 IPC 返回给前端的 child WebView 标签信息。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WebviewTab {
    pub label: String,
    pub url: String,
    pub display_name: String,
}

/// 维护 DSH origin 到 child WebView 标签的映射。
///
/// `CHILD_WEBVIEW_TOP` 必须与 `AppNavbar.vue` 的 `h-9`（36px）保持一致。
#[derive(Debug, Clone)]
pub(crate) struct WebviewState {
    registry: Arc<Mutex<WebviewRegistry>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct WebviewOrigin {
    scheme: String,
    host: String,
    port: u16,
}

impl WebviewOrigin {
    fn from_url(url: &tauri::Url) -> Result<Self, BackendError> {
        Ok(Self {
            scheme: url.scheme().to_owned(),
            host: url
                .host_str()
                .ok_or(BackendError::InvalidWebviewUrl)?
                .to_owned(),
            port: url
                .port_or_known_default()
                .ok_or(BackendError::InvalidWebviewUrl)?,
        })
    }

    fn matches(&self, url: &tauri::Url) -> bool {
        self.scheme == url.scheme()
            && self.host == url.host_str().unwrap_or_default()
            && self.port == url.port_or_known_default().unwrap_or_default()
    }
}

/// 创建动态 child WebView 的共享状态。
pub(crate) fn create_webview_state() -> WebviewState {
    WebviewState {
        registry: Arc::new(Mutex::new(WebviewRegistry::default())),
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
pub(crate) fn create_webview_with_url(
    app: &AppHandle,
    url: String,
    state: &WebviewState,
) -> Result<WebviewTab, BackendError> {
    let parsed_url = parse_webview_url(&url)?;
    let origin = WebviewOrigin::from_url(&parsed_url)?;
    let origin_key = origin.clone();

    prune_stale_tabs(app, state)?;

    if let Some(tab) = find_tab(state, &origin_key)? {
        if app.get_webview(&tab.label).is_some() {
            let labels = tab_labels(state)?;
            activate_webview(app, &labels, &tab.label)?;
            log::debug!("webview tab exists, activated: label={}", tab.label);
            return Ok(tab_info(&tab));
        }
        remove_tab(state, &origin_key)?;
    }

    let label = reserve_tab(state, origin_key.clone())?;
    let parent = app.get_window("main").ok_or_else(|| {
        release_reserved_tab(state, &origin_key);
        BackendError::WindowResourceMissing
    })?;
    let (position, size) = match child_webview_bounds(&parent) {
        Ok(bounds) => bounds,
        Err(error) => {
            release_reserved_tab(state, &origin_key);
            return Err(error);
        }
    };
    let builder = build_child_webview(label.clone(), parsed_url.clone(), origin, app.clone());

    if let Err(error) = parent.add_child(builder, position, size) {
        release_reserved_tab(state, &origin_key);
        return Err(BackendError::Window(error));
    }

    let tab = ChildTab {
        label,
        url: parsed_url.to_string(),
    };
    finish_reserved_tab(state, origin_key, tab.clone())?;

    let labels = tab_labels(state)?;
    activate_webview(app, &labels, &tab.label)?;
    log::info!("webview tab created: label={}, url={}", tab.label, tab.url);
    Ok(tab_info(&tab))
}

/// 激活一个 child WebView，并隐藏同一主窗口中的其他 child WebView。
pub(crate) fn activate_webview_tab(
    app: &AppHandle,
    label: String,
    state: &WebviewState,
) -> Result<(), BackendError> {
    prune_stale_tabs(app, state)?;
    let labels = tab_labels(state)?;
    if !labels.iter().any(|item| item == &label) {
        return Err(BackendError::ChildWebviewNotFound);
    }
    log::debug!("webview tab activated: label={label}");
    activate_webview(app, &labels, &label)
}

/// 关闭一个 child WebView 并移除其注册信息。
pub(crate) fn close_webview_tab(
    app: &AppHandle,
    label: String,
    state: &WebviewState,
) -> Result<(), BackendError> {
    prune_stale_tabs(app, state)?;
    let key = find_tab_key(state, &label)?;
    let webview = app
        .get_webview(&label)
        .ok_or(BackendError::ChildWebviewNotFound)?;
    webview.close().map_err(BackendError::Window)?;
    remove_tab(state, &key)?;
    log::info!("webview tab closed: label={label}");
    Ok(())
}

/// 隐藏所有 child WebView。
pub(crate) fn hide_all_webview_tabs(
    app: &AppHandle,
    state: &WebviewState,
) -> Result<(), BackendError> {
    prune_stale_tabs(app, state)?;
    for label in tab_labels(state)? {
        if let Some(webview) = app.get_webview(&label) {
            webview.hide().map_err(BackendError::Window)?;
        }
    }
    Ok(())
}

/// 注册主窗口 resize 监听，保持 child WebView 使用固定的顶部偏移。
pub(crate) fn register_resize_handler(
    app: &AppHandle,
    state: WebviewState,
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

fn resize_child_webviews(app: &AppHandle, state: &WebviewState) -> Result<(), BackendError> {
    prune_stale_tabs(app, state)?;
    let window = app
        .get_window("main")
        .ok_or(BackendError::WindowResourceMissing)?;
    let (position, size) = child_webview_bounds(&window)?;

    for label in tab_labels(state)? {
        if let Some(webview) = app.get_webview(&label) {
            webview
                .set_position(position)
                .map_err(BackendError::Window)?;
            webview.set_size(size).map_err(BackendError::Window)?;
        }
    }
    Ok(())
}

fn child_webview_bounds<R: Runtime>(
    window: &tauri::Window<R>,
) -> Result<(LogicalPosition<f64>, LogicalSize<f64>), BackendError> {
    let scale_factor = window.scale_factor().map_err(BackendError::Window)?;
    let size = window
        .inner_size()
        .map_err(BackendError::Window)?
        .to_logical::<f64>(scale_factor);
    let content_height = (size.height - CHILD_WEBVIEW_TOP).max(1.0);
    Ok((
        LogicalPosition::new(0.0, CHILD_WEBVIEW_TOP),
        LogicalSize::new(size.width, content_height),
    ))
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

fn activate_webview(app: &AppHandle, labels: &[String], label: &str) -> Result<(), BackendError> {
    let target = app
        .get_webview(label)
        .ok_or(BackendError::ChildWebviewNotFound)?;
    for item in labels {
        if let Some(webview) = app.get_webview(item) {
            if item == label {
                webview.show().map_err(BackendError::Window)?;
            } else {
                webview.hide().map_err(BackendError::Window)?;
            }
        }
    }
    target.set_focus().map_err(BackendError::Window)
}

fn tab_info(tab: &ChildTab) -> WebviewTab {
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
    }
}

fn find_tab(
    state: &WebviewState,
    origin: &WebviewOrigin,
) -> Result<Option<ChildTab>, BackendError> {
    let registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    Ok(registry.tabs.get(origin).cloned())
}

fn find_tab_key(state: &WebviewState, label: &str) -> Result<WebviewOrigin, BackendError> {
    let registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    registry
        .tabs
        .iter()
        .find(|(_, tab)| tab.label == label)
        .map(|(origin, _)| origin.clone())
        .ok_or(BackendError::ChildWebviewNotFound)
}

fn tab_labels(state: &WebviewState) -> Result<Vec<String>, BackendError> {
    let registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    Ok(registry
        .tabs
        .values()
        .map(|tab| tab.label.clone())
        .collect())
}

fn reserve_tab(state: &WebviewState, origin: WebviewOrigin) -> Result<String, BackendError> {
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    if registry.tabs.contains_key(&origin) || !registry.creating.insert(origin.clone()) {
        return Err(BackendError::OperationInProgress);
    }
    Ok(next_window_label(&mut registry))
}

fn finish_reserved_tab(
    state: &WebviewState,
    origin: WebviewOrigin,
    tab: ChildTab,
) -> Result<(), BackendError> {
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    registry.creating.remove(&origin);
    registry.tabs.insert(origin, tab);
    Ok(())
}

fn release_reserved_tab(state: &WebviewState, origin: &WebviewOrigin) {
    if let Ok(mut registry) = state.registry.lock() {
        registry.creating.remove(origin);
    }
}

fn remove_tab(state: &WebviewState, origin: &WebviewOrigin) -> Result<(), BackendError> {
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    registry.tabs.remove(origin);
    Ok(())
}

fn prune_stale_tabs(app: &AppHandle, state: &WebviewState) -> Result<(), BackendError> {
    let labels = tab_labels(state)?;
    let stale_labels: HashSet<String> = labels
        .into_iter()
        .filter(|label| app.get_webview(label).is_none())
        .collect();
    if stale_labels.is_empty() {
        return Ok(());
    }

    let mut registry = state
        .registry
        .lock()
        .map_err(|_| BackendError::WindowStatePoisoned)?;
    registry
        .tabs
        .retain(|_, tab| !stale_labels.contains(&tab.label));
    log::warn!("pruned stale webview tabs: {stale_labels:?}");
    Ok(())
}

fn parse_webview_url(input: &str) -> Result<tauri::Url, BackendError> {
    let parsed = input
        .parse::<tauri::Url>()
        .map_err(|_| BackendError::InvalidWebviewUrl)?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(BackendError::InvalidWebviewUrl);
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(BackendError::InvalidWebviewUrl);
    }
    if parsed.host().is_none() {
        return Err(BackendError::InvalidWebviewUrl);
    }

    Ok(parsed)
}

fn next_window_label(registry: &mut WebviewRegistry) -> String {
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

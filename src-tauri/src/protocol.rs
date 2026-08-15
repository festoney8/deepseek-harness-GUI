//! 协议契约层：webview 与 Rust 通讯的类型与事件名常量。
//!
//! `Snapshot` 经 serde 序列化为 JSON 后通过 IPC 传给前端，
//! 前端 `useRuntime.ts` 的 `RuntimeSnapshot` 是它的 TypeScript 镜像。

use serde::Serialize;

/// 运行阶段：状态页与前端渲染的依据
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[default]
    Idle,
    Installing,
    Starting,
    Ready,
    Failed,
}

/// 三格面板 + 终端的完整状态快照（前端看到的 JSON 结构）
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub phase: Phase,
    pub port: Option<u16>,
    pub detail: String,
    pub elapsed: Option<u64>,
    /// 格子一：node / npm 版本（None = 未检测到）
    pub node: Option<String>,
    pub npm: Option<String>,
    /// 格子二：远端 / 本地 dsh 版本（local None = 未安装或安装不完整；remote None = 获取失败）
    pub remote: Option<String>,
    /// 镜像源查询到的远端版本（None = 获取失败，仅用于展示）
    pub remote_mirror: Option<String>,
    pub local: Option<String>,
    pub version_error: bool,
    /// 版本检查是否已完成（false = 尚未检查，前端显示"检查中"）
    pub version_checked: bool,
}

/// 状态变更推送事件
pub const EVENT_RUNTIME_STATE: &str = "runtime-state";
/// 窗口关闭拦截通知（前端据此弹出托盘/退出确认）
pub const EVENT_CLOSE_REQUESTED: &str = "close-requested";
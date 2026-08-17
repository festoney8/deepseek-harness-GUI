use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::{Mutex, RwLock};

use crate::platform::ManagedProcess;

use super::BackendError;

/// DSH 单实例生命周期的当前阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPhase {
    /// 当前没有受控 DSH 实例。
    Stopped,
    /// 正在创建进程并等待服务就绪。
    Starting,
    /// DSH 已经完成就绪探测并保持运行。
    Running,
    /// 正在终止进程树并回收资源。
    Stopping,
}

/// 当前 DSH 实例及其进程身份信息。
#[derive(Debug)]
pub(crate) struct HarnessLifecycle {
    /// 生命周期当前所处的阶段。
    pub phase: HarnessPhase,
    /// 用于区分新旧进程实例的递增标识。
    pub generation: u64,
    /// 当前受控进程的共享控制句柄。
    pub process: Option<ManagedProcess>,
}

/// 协调 DSH 单实例生命周期变更与状态读取的共享状态。
#[derive(Debug)]
pub(crate) struct HarnessState {
    /// 保证启动和停止操作不会同时执行的异步互斥锁。
    pub operation: Mutex<()>,
    /// 允许后台监控与业务操作访问的生命周期状态。
    pub lifecycle: RwLock<HarnessLifecycle>,
}

/// 创建 DSH 生命周期共享状态。
pub(crate) fn create_harness_state() -> HarnessState {
    todo!()
}

/// 启动单实例 DSH 服务并等待 WebUI 就绪。
pub(crate) async fn start_dsh(
    port: u16,
    app: &AppHandle,
    state: &HarnessState,
) -> Result<String, BackendError> {
    todo!()
}

/// 终止当前 DSH 进程树并清理生命周期状态。
pub(crate) async fn stop_dsh(state: &HarnessState) -> Result<(), BackendError> {
    todo!()
}

/// 监控 DSH 进程退出并发送前端事件。
pub(crate) async fn monitor_dsh_exit(
    app: AppHandle,
    state: Arc<HarnessState>,
    process: ManagedProcess,
    generation: u64,
) -> Result<(), BackendError> {
    todo!()
}

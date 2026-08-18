use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use log::{error, info, warn};
use tauri::AppHandle;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{Mutex, RwLock},
};

use crate::platform::{self, ManagedProcess};

use super::{check_http, check_tcp, BackendError};

/// DSH 启动等待 WebUI 就绪的最长时间。
const START_TIMEOUT: Duration = Duration::from_secs(10);
/// DSH 启动阶段两次就绪探测之间的间隔。
const PROBE_INTERVAL: Duration = Duration::from_millis(200);
/// 单次 TCP/HTTP 就绪探测允许的最长时间。
const PROBE_IO_TIMEOUT: Duration = Duration::from_millis(500);
/// 主动停止 DSH 进程树时等待优雅退出的时间。
const STOP_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// DSH 单实例生命周期的当前阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPhase {
    /// 没有受控 DSH 实例。
    Stopped,
    /// 正在创建 DSH 并等待 WebUI 就绪。
    Starting,
    /// DSH 已通过 WebUI 就绪探测。
    Running,
    /// 正在终止当前 DSH 进程树。
    Stopping,
}

/// 当前 DSH 实例及其进程身份信息。
#[derive(Debug)]
pub(crate) struct HarnessLifecycle {
    /// 当前生命周期阶段。
    pub phase: HarnessPhase,
    /// 每次启动递增的实例代次，用于拒绝旧监控任务更新新实例状态。
    pub generation: u64,
    /// 当前受控 DSH 进程；停止状态下为空。
    pub process: Option<ManagedProcess>,
}

/// 协调 DSH 单实例生命周期变更与状态读取的共享状态。
#[derive(Debug)]
pub(crate) struct HarnessState {
    /// 串行化启动和停止操作的互斥锁。
    pub operation: Arc<Mutex<()>>,
    /// 允许监控任务读取和更新的生命周期状态。
    pub lifecycle: Arc<RwLock<HarnessLifecycle>>,
}

/// 创建 DSH 生命周期共享状态。
pub(crate) fn create_harness_state() -> HarnessState {
    HarnessState {
        operation: Arc::new(Mutex::new(())),
        lifecycle: Arc::new(RwLock::new(HarnessLifecycle {
            phase: HarnessPhase::Stopped,
            generation: 0,
            process: None,
        })),
    }
}

/// 启动单实例 DSH 服务并等待 WebUI 就绪。
pub(crate) async fn start_dsh(
    port: u16,
    _app: &AppHandle,
    state: &HarnessState,
) -> Result<String, BackendError> {
    if port == 0 {
        return Err(BackendError::InvalidPort);
    }

    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| BackendError::OperationInProgress)?;
    let generation = begin_start(&state.lifecycle).await?;

    if check_tcp("127.0.0.1", port, PROBE_IO_TIMEOUT).await? {
        reset_if_current(&state.lifecycle, generation).await;
        return Err(BackendError::PortOccupied);
    }

    let spawned = match platform::spawn_dsh(port) {
        Ok(spawned) => spawned,
        Err(error) => {
            error!("dsh spawn failed: {error:?}");
            reset_if_current(&state.lifecycle, generation).await;
            return Err(BackendError::DshSpawnFailed);
        }
    };
    let process = spawned.process.clone();
    print_stream("stdout", spawned.stdout);
    print_stream("stderr", spawned.stderr);

    {
        let mut lifecycle = state.lifecycle.write().await;
        if lifecycle.generation == generation && lifecycle.phase == HarnessPhase::Starting {
            lifecycle.process = Some(process.clone());
        }
    }
    spawn_exit_monitor(Arc::clone(&state.lifecycle), process.clone(), generation);

    let deadline = Instant::now() + START_TIMEOUT;
    let address = format!("http://127.0.0.1:{port}");
    loop {
        if let Some(exit) = process.try_wait()? {
            warn!("dsh exited before WebUI readiness: {exit:?}");
            reset_if_current(&state.lifecycle, generation).await;
            return Err(BackendError::DshExitedEarly);
        }

        if check_http(&address, PROBE_IO_TIMEOUT).await? {
            let mut lifecycle = state.lifecycle.write().await;
            if lifecycle.generation != generation || lifecycle.process.is_none() {
                return Err(BackendError::DshExitedEarly);
            }
            lifecycle.phase = HarnessPhase::Running;
            info!("dsh WebUI ready at {address}");
            return Ok(address);
        }

        if Instant::now() >= deadline {
            warn!("dsh startup timed out on port {port}");
            let termination = process.terminate_tree(STOP_GRACE_PERIOD).await;
            reset_if_current(&state.lifecycle, generation).await;
            termination?;
            return Err(BackendError::DshStartTimeout);
        }

        tokio::time::sleep(PROBE_INTERVAL).await;
    }
}

/// 终止当前 DSH 进程树并清理生命周期状态。
pub(crate) async fn stop_dsh(state: &HarnessState) -> Result<(), BackendError> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| BackendError::OperationInProgress)?;
    let (generation, process) = {
        let mut lifecycle = state.lifecycle.write().await;
        let process = lifecycle
            .process
            .clone()
            .ok_or(BackendError::ProcessNotRunning)?;
        lifecycle.phase = HarnessPhase::Stopping;
        (lifecycle.generation, process)
    };

    let exit = process.terminate_tree(STOP_GRACE_PERIOD).await?;
    info!("dsh process tree stopped: exit={exit:?}");
    reset_if_current(&state.lifecycle, generation).await;
    Ok(())
}

/// 监控 DSH 进程退出并清理当前实例状态。
pub(crate) async fn monitor_dsh_exit(
    _app: AppHandle,
    state: Arc<HarnessState>,
    process: ManagedProcess,
    generation: u64,
) -> Result<(), BackendError> {
    monitor_exit(state.lifecycle.clone(), process, generation).await
}

/// 将停止状态切换为启动状态并返回新实例代次。
async fn begin_start(lifecycle: &RwLock<HarnessLifecycle>) -> Result<u64, BackendError> {
    let mut lifecycle = lifecycle.write().await;
    match lifecycle.phase {
        HarnessPhase::Stopped => {
            lifecycle.phase = HarnessPhase::Starting;
            lifecycle.generation = lifecycle.generation.wrapping_add(1);
            Ok(lifecycle.generation)
        }
        HarnessPhase::Running => Err(BackendError::DshAlreadyRunning),
        HarnessPhase::Starting | HarnessPhase::Stopping => Err(BackendError::OperationInProgress),
    }
}

/// 创建不持有生命周期操作锁的后台退出监控任务。
fn spawn_exit_monitor(
    lifecycle: Arc<RwLock<HarnessLifecycle>>,
    process: ManagedProcess,
    generation: u64,
) {
    tokio::spawn(async move {
        if let Err(error) = monitor_exit(lifecycle, process, generation).await {
            error!("dsh exit monitor failed: {error:?}");
        }
    });
}

/// 等待进程退出，并仅在代次仍匹配时清理生命周期状态。
async fn monitor_exit(
    lifecycle: Arc<RwLock<HarnessLifecycle>>,
    process: ManagedProcess,
    generation: u64,
) -> Result<(), BackendError> {
    let exit = process.wait().await?;
    let mut lifecycle = lifecycle.write().await;
    if lifecycle.generation == generation && lifecycle.process.is_some() {
        lifecycle.phase = HarnessPhase::Stopped;
        lifecycle.process = None;
        info!("dsh exited: generation={generation}, exit={exit:?}");
    }
    Ok(())
}

/// 仅在目标代次仍是当前实例时将生命周期恢复为停止状态。
async fn reset_if_current(lifecycle: &RwLock<HarnessLifecycle>, generation: u64) {
    let mut lifecycle = lifecycle.write().await;
    if lifecycle.generation == generation {
        lifecycle.phase = HarnessPhase::Stopped;
        lifecycle.process = None;
    }
}

/// 异步读取 DSH 输出流，并按来源写入统一日志系统。
fn print_stream(name: &'static str, stream: impl AsyncRead + Unpin + Send + 'static) {
    tokio::spawn(async move {
        let mut stream = stream;
        let mut buffer = [0_u8; 4096];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(length) => {
                    let message = String::from_utf8_lossy(&buffer[..length]);
                    if name == "stderr" {
                        error!("[dsh][{name}] {message}");
                    } else {
                        info!("[dsh][{name}] {message}");
                    }
                }
                Err(error) => {
                    error!("dsh {name} stream read failed: {error:?}");
                    break;
                }
            }
        }
        info!("dsh {name} stream closed");
    });
}

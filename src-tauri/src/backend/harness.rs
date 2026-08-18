use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::Arc,
    time::{Duration, Instant},
};

use tauri::AppHandle;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{Mutex, RwLock},
};

use crate::platform::{self, ManagedProcess, SpawnedProcess};

use super::BackendError;

const START_TIMEOUT: Duration = Duration::from_secs(10);
const PROBE_INTERVAL: Duration = Duration::from_millis(200);
const PROBE_IO_TIMEOUT: Duration = Duration::from_millis(500);
const STOP_GRACE_PERIOD: Duration = Duration::from_secs(5);

/// DSH 单实例生命周期的当前阶段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessPhase {
    Stopped,
    Starting,
    Running,
    Stopping,
}

/// 当前 DSH 实例及其进程身份信息。
#[derive(Debug)]
pub(crate) struct HarnessLifecycle {
    pub phase: HarnessPhase,
    pub generation: u64,
    pub process: Option<ManagedProcess>,
}

/// 协调 DSH 单实例生命周期变更与状态读取的共享状态。
#[derive(Debug)]
pub(crate) struct HarnessState {
    pub operation: Arc<Mutex<()>>,
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
    start_dsh_with(port, state, platform::spawn_dsh).await
}

async fn start_dsh_with(
    port: u16,
    state: &HarnessState,
    spawn: fn(u16) -> Result<SpawnedProcess, platform::PlatformError>,
) -> Result<String, BackendError> {
    if port == 0 {
        return Err(BackendError::InvalidPort);
    }

    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| BackendError::OperationInProgress)?;
    let generation = begin_start(&state.lifecycle).await?;

    if port_is_occupied(port) {
        reset_if_current(&state.lifecycle, generation).await;
        return Err(BackendError::PortOccupied);
    }

    let spawned = match spawn(port) {
        Ok(spawned) => spawned,
        Err(error) => {
            eprintln!("[dsh][spawn-error] {error:?}");
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
    loop {
        if let Some(exit) = process.try_wait()? {
            eprintln!("[dsh][exited-before-ready] {exit:?}");
            reset_if_current(&state.lifecycle, generation).await;
            return Err(BackendError::DshExitedEarly);
        }

        if http_is_ready(port) {
            let mut lifecycle = state.lifecycle.write().await;
            if lifecycle.generation != generation || lifecycle.process.is_none() {
                return Err(BackendError::DshExitedEarly);
            }
            lifecycle.phase = HarnessPhase::Running;
            let address = format!("http://127.0.0.1:{port}");
            println!("[dsh][ready] {address}");
            return Ok(address);
        }

        if Instant::now() >= deadline {
            eprintln!("[dsh][start-timeout] port={port}");
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
    println!("[dsh][stopped] exit={exit:?}");
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

fn spawn_exit_monitor(
    lifecycle: Arc<RwLock<HarnessLifecycle>>,
    process: ManagedProcess,
    generation: u64,
) {
    tokio::spawn(async move {
        if let Err(error) = monitor_exit(lifecycle, process, generation).await {
            eprintln!("[dsh][monitor-error] {error:?}");
        }
    });
}

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
        println!("[dsh][exited] generation={generation} exit={exit:?}");
    }
    Ok(())
}

async fn reset_if_current(lifecycle: &RwLock<HarnessLifecycle>, generation: u64) {
    let mut lifecycle = lifecycle.write().await;
    if lifecycle.generation == generation {
        lifecycle.phase = HarnessPhase::Stopped;
        lifecycle.process = None;
    }
}

fn loopback(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

fn port_is_occupied(port: u16) -> bool {
    TcpStream::connect_timeout(&loopback(port), PROBE_IO_TIMEOUT).is_ok()
}

fn http_is_ready(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&loopback(port), PROBE_IO_TIMEOUT) else {
        return false;
    };
    if stream.set_read_timeout(Some(PROBE_IO_TIMEOUT)).is_err()
        || stream.set_write_timeout(Some(PROBE_IO_TIMEOUT)).is_err()
        || stream
            .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .is_err()
    {
        return false;
    }

    let mut response = [0_u8; 64];
    let Ok(length) = stream.read(&mut response) else {
        return false;
    };
    let status = String::from_utf8_lossy(&response[..length]);
    status
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .is_some_and(|code| (200..300).contains(&code))
}

fn print_stream(name: &'static str, stream: impl AsyncRead + Unpin + Send + 'static) {
    tokio::spawn(async move {
        let mut stream = stream;
        let mut buffer = [0_u8; 4096];
        loop {
            match stream.read(&mut buffer).await {
                Ok(0) => break,
                Ok(length) => {
                    print!(
                        "[dsh][{name}] {}",
                        String::from_utf8_lossy(&buffer[..length])
                    );
                }
                Err(error) => {
                    eprintln!("[dsh][{name}-read-error] {error:?}");
                    break;
                }
            }
        }
        println!("[dsh][{name}-closed]");
    });
}

#[cfg(all(test, windows))]
mod tests {
    use std::{
        net::TcpListener,
        process::Command,
        time::{Duration, Instant},
    };

    use super::*;

    fn unused_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .expect("应能绑定临时端口")
            .local_addr()
            .expect("临时监听器应有本地地址")
            .port()
    }

    async fn current_phase(state: &HarnessState) -> HarnessPhase {
        state.lifecycle.read().await.phase
    }

    #[tokio::test(flavor = "current_thread")]
    async fn create_state_starts_stopped() {
        let state = create_harness_state();
        assert_eq!(current_phase(&state).await, HarnessPhase::Stopped);
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "手动场景：打印不存在命令的完整输出"]
    async fn scenario_1_missing_dsh_prints_failure() {
        let state = create_harness_state();
        let port = unused_port();
        println!("[scenario-1] start missing dsh on port {port}");

        let result = start_dsh_with(port, &state, platform::spawn_missing_dsh_for_test).await;
        tokio::time::sleep(Duration::from_millis(200)).await;

        println!("[scenario-1] result={result:?}");
        assert!(matches!(result, Err(BackendError::DshExitedEarly)));
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "手动场景：占用端口后验证启动被拒绝"]
    async fn scenario_2_occupied_port_prints_failure() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("应能占用临时端口");
        let port = listener.local_addr().expect("监听器应有地址").port();
        let state = create_harness_state();
        println!("[scenario-2] occupied port={port}");

        let result = start_dsh_with(port, &state, platform::spawn_dsh).await;

        println!("[scenario-2] result={result:?}");
        assert!(matches!(result, Err(BackendError::PortOccupied)));
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "手动场景：需要已安装 dsh"]
    async fn scenario_3_dsh_starts_web_server() {
        let state = create_harness_state();
        let port = unused_port();
        println!("[scenario-3] starting dsh on port {port}");

        let address = start_dsh_with(port, &state, platform::spawn_dsh)
            .await
            .expect("dsh 应成功启动");
        println!("[scenario-3] ready address={address}");

        stop_dsh(&state).await.expect("dsh 应成功停止");
        println!("[scenario-3] final phase={:?}", current_phase(&state).await);
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "手动场景：需要已安装 dsh，运行至少十秒"]
    async fn scenario_4_dsh_stops_after_ten_second_countdown() {
        let state = create_harness_state();
        let port = unused_port();
        let address = start_dsh_with(port, &state, platform::spawn_dsh)
            .await
            .expect("dsh 应成功启动");
        println!("[scenario-4] ready address={address}");

        for remaining in (1..=10).rev() {
            println!("[scenario-4] auto stop in {remaining}s");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        stop_dsh(&state).await.expect("倒计时后应成功停止 dsh");
        println!("[scenario-4] final phase={:?}", current_phase(&state).await);
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "手动场景：需要已安装 dsh，并通过 taskkill 模拟崩溃"]
    async fn scenario_5_external_kill_is_observed() {
        let state = create_harness_state();
        let port = unused_port();
        let address = start_dsh_with(port, &state, platform::spawn_dsh)
            .await
            .expect("dsh 应成功启动");
        let process = state
            .lifecycle
            .read()
            .await
            .process
            .clone()
            .expect("运行状态应保存进程");
        let pid = process.process_id();
        println!("[scenario-5] ready address={address} pid={pid}");

        let output = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
            .expect("taskkill 应可执行");
        println!(
            "[scenario-5] taskkill exit={:?}\nstdout={}\nstderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        while current_phase(&state).await != HarnessPhase::Stopped && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        println!("[scenario-5] final phase={:?}", current_phase(&state).await);
        assert_eq!(current_phase(&state).await, HarnessPhase::Stopped);
    }
}

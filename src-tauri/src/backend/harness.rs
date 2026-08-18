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
    start_dsh_inner(port, state).await
}

async fn start_dsh_inner(port: u16, state: &HarnessState) -> Result<String, BackendError> {
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
                    #[cfg(test)]
                    eprint!("[dsh][{name}] {message}");
                }
                Err(error) => {
                    error!("dsh {name} stream read failed: {error:?}");
                    #[cfg(test)]
                    eprintln!("dsh {name} stream read failed: {error:?}");
                    break;
                }
            }
        }
        info!("dsh {name} stream closed");
        #[cfg(test)]
        eprintln!("dsh {name} stream closed");
    });
}

#[cfg(all(test, unix))]
mod tests {
    use std::{
        error::Error,
        io,
        net::TcpListener,
        process::Output,
        time::{Duration, Instant},
    };

    use tokio::process::Command;

    use super::{
        create_harness_state, start_dsh_inner, stop_dsh, BackendError, HarnessPhase, HarnessState,
    };

    const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
    const INSTALL_TIMEOUT: Duration = Duration::from_secs(10 * 60);
    const CRASH_TIMEOUT: Duration = Duration::from_secs(10);

    type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

    /// 按真实环境顺序执行破坏性 DSH 生命周期调试，仅允许显式单独运行。
    #[tokio::test(flavor = "current_thread")]
    #[ignore = "会全局安装 DSH、启动真实 WebUI 并执行进程信号测试"]
    async fn debug_unix_dsh_lifecycle_in_required_order() -> TestResult {
        println!("\n[1/7] 检查 node、npm 存在，并验证未安装 DSH 时启动失败");
        let node = run_command("node", &["--version"], COMMAND_TIMEOUT).await?;
        ensure_success("node --version", &node)?;
        let npm = run_command("npm", &["--version"], COMMAND_TIMEOUT).await?;
        ensure_success("npm --version", &npm)?;
        ensure_dsh_missing().await?;

        let state = create_harness_state();
        let missing_dsh_port = unused_port()?;
        let missing_dsh_result = start_dsh_inner(missing_dsh_port, &state).await;
        println!("未安装 DSH 启动结果: {missing_dsh_result:?}");
        assert!(matches!(
            missing_dsh_result,
            Err(BackendError::DshSpawnFailed)
        ));
        assert_stopped(&state).await;

        println!("\n[2/7] 使用 npm i -g @deepseek-ai/dsh 全局安装 DSH");
        let install = run_command("npm", &["i", "-g", "@deepseek-ai/dsh"], INSTALL_TIMEOUT).await?;
        ensure_success("npm i -g @deepseek-ai/dsh", &install)?;

        println!("\n[3/7] 执行 dsh -V 验证安装结果");
        let version = run_command("dsh", &["-V"], COMMAND_TIMEOUT).await?;
        ensure_success("dsh -V", &version)?;

        let occupied_listener = TcpListener::bind("127.0.0.1:0")?;
        let port = occupied_listener.local_addr()?.port();

        println!("\n[4/7] 使用已占用端口 {port} 启动 DSH");
        let occupied_result = start_dsh_inner(port, &state).await;
        println!("端口占用启动结果: {occupied_result:?}");
        print_lifecycle("端口占用检查后", &state).await;
        assert!(matches!(occupied_result, Err(BackendError::PortOccupied)));
        drop(occupied_listener);

        println!("\n[5/7] 释放端口 {port} 后启动 DSH WebUI");
        let address = start_dsh_inner(port, &state).await?;
        println!("WebUI 已就绪: {address}");
        print_lifecycle("正常启动后", &state).await;
        assert_eq!(address, format!("http://127.0.0.1:{port}"));

        println!("\n[6/7] DSH 运行 10 秒后主动终止完整进程组");
        for remaining in (1..=10).rev() {
            println!("距离 stop_dsh 还有 {remaining} 秒");
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        let managed_process = current_process(&state).await?;
        stop_dsh(&state).await?;
        let stopped_exit = managed_process.wait().await?;
        println!("主动停止退出结果: {stopped_exit:?}");
        println!("主动停止 try_wait 缓存: {:?}", managed_process.try_wait()?);
        assert_eq!(stopped_exit.exit_code, Some(0));
        assert_stopped(&state).await;
        print_lifecycle("主动停止后", &state).await;

        println!("\n[7/7] 再次启动 DSH，并从生命周期管理器外部 SIGKILL leader");
        let crash_address = start_dsh_inner(port, &state).await?;
        let crashed_process = current_process(&state).await?;
        println!("崩溃测试 WebUI 已就绪: {crash_address}");
        println!("强杀前 ManagedProcess: {crashed_process:#?}");
        crate::platform::force_kill_dsh_for_test(&crashed_process)?;
        let crashed_exit = tokio::time::timeout(CRASH_TIMEOUT, crashed_process.wait()).await??;
        println!("外界强杀退出结果: {crashed_exit:?}");
        println!("外界强杀 try_wait 缓存: {:?}", crashed_process.try_wait()?);
        assert_eq!(crashed_exit.exit_code, None);
        wait_until_stopped(&state, CRASH_TIMEOUT).await?;
        assert_stopped(&state).await;
        print_lifecycle("崩溃监控清理后", &state).await;

        Ok(())
    }

    async fn run_command(program: &str, args: &[&str], timeout: Duration) -> TestResult<Output> {
        let started = Instant::now();
        let output =
            tokio::time::timeout(timeout, Command::new(program).args(args).output()).await??;
        println!(
            "$ {program} {}\n耗时: {:?}\n退出状态: {:?}\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            started.elapsed(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        Ok(output)
    }

    fn ensure_success(command: &str, output: &Output) -> TestResult {
        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "{command} 执行失败: status={:?}, stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ))
            .into())
        }
    }

    async fn ensure_dsh_missing() -> TestResult {
        match Command::new("dsh").arg("-V").output().await {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                println!("dsh -V 创建失败，符合未安装环境预期: {error:?}");
                Ok(())
            }
            Err(error) => Err(io::Error::other(format!(
                "dsh -V 返回了非 NotFound 创建错误: {error:?}"
            ))
            .into()),
            Ok(output) => Err(io::Error::other(format!(
                "测试要求初始环境未安装 DSH，但 dsh -V 可执行: status={:?}, stdout={}, stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ))
            .into()),
        }
    }

    fn unused_port() -> TestResult<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        drop(listener);
        Ok(port)
    }

    async fn assert_stopped(state: &HarnessState) {
        let lifecycle = state.lifecycle.read().await;
        assert_eq!(lifecycle.phase, HarnessPhase::Stopped);
        assert!(lifecycle.process.is_none());
    }

    async fn current_process(state: &HarnessState) -> TestResult<crate::platform::ManagedProcess> {
        state
            .lifecycle
            .read()
            .await
            .process
            .clone()
            .ok_or_else(|| io::Error::other("生命周期中没有 ManagedProcess").into())
    }

    async fn wait_until_stopped(state: &HarnessState, timeout: Duration) -> TestResult {
        tokio::time::timeout(timeout, async {
            loop {
                if state.lifecycle.read().await.phase == HarnessPhase::Stopped {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .map_err(|error| io::Error::other(format!("等待生命周期恢复 Stopped 超时: {error}")))?;
        Ok(())
    }

    async fn print_lifecycle(label: &str, state: &HarnessState) {
        let lifecycle = state.lifecycle.read().await;
        println!(
            "{label}: phase={:?}, generation={}, process={:#?}, cached_exit={:?}",
            lifecycle.phase,
            lifecycle.generation,
            lifecycle.process,
            lifecycle
                .process
                .as_ref()
                .map(crate::platform::ManagedProcess::try_wait),
        );
    }
}

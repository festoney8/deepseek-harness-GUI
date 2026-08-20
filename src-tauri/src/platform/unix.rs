use std::{
    io,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::sync::{Mutex as AsyncMutex, Notify};

use super::{PlatformError, ProcessExit, ProcessKind, SpawnedProcess};

/// Unix 平台上可克隆的受控进程组句柄。
#[derive(Debug, Clone)]
pub(crate) struct ManagedProcess {
    /// 共享持有 Unix 平台进程组控制资源。
    inner: Arc<UnixProcess>,
}

/// 封装 Unix 进程组标识及共享退出状态。
#[derive(Debug)]
struct UnixProcess {
    /// 创建进程时固定的业务用途。
    kind: ProcessKind,
    /// 由唯一监督任务负责回收的直接子进程 PID。
    leader_pid: LeaderPid,
    /// 接收进程树终止信号的独立进程组 PGID。
    process_group_id: ProcessGroupId,
    /// 由唯一监督任务写入且可供多个调用者重复读取的退出结果。
    exit: Mutex<Option<Result<ProcessExit, WaitFailure>>>,
    /// 退出结果写入后通知全部异步等待者。
    exited: Notify,
    /// 保证并发终止请求只执行一次实际信号流程。
    termination: AsyncMutex<()>,
}

/// 直接子进程 PID，避免与进程组 ID 混用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeaderPid(libc::pid_t);

/// 受控进程组 PGID，避免与直接子进程 PID 混用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProcessGroupId(libc::pid_t);

/// 可缓存和复制的等待失败信息。
#[derive(Debug, Clone)]
struct WaitFailure {
    kind: io::ErrorKind,
    message: Arc<str>,
}

impl WaitFailure {
    fn from_io(error: io::Error) -> Self {
        Self {
            kind: error.kind(),
            message: Arc::from(error.to_string()),
        }
    }

    fn to_io(&self) -> io::Error {
        io::Error::new(self.kind, self.message.to_string())
    }
}

impl ManagedProcess {
    /// 非阻塞读取已经缓存的进程退出结果。
    pub(crate) fn try_wait(&self) -> Result<Option<ProcessExit>, PlatformError> {
        let exit = self.inner.exit.lock().map_err(|_| PlatformError::Wait {
            kind: self.inner.kind,
            source: io::Error::other("进程退出状态锁已损坏"),
        })?;

        match exit.as_ref() {
            Some(Ok(status)) => Ok(Some(*status)),
            Some(Err(error)) => Err(PlatformError::Wait {
                kind: self.inner.kind,
                source: error.to_io(),
            }),
            None => Ok(None),
        }
    }

    /// 异步等待唯一监督任务缓存进程退出结果。
    pub(crate) async fn wait(&self) -> Result<ProcessExit, PlatformError> {
        loop {
            let notified = self.inner.exited.notified();
            if let Some(exit) = self.try_wait()? {
                return Ok(exit);
            }
            notified.await;
        }
    }

    /// 先向进程组发送 SIGTERM，并在宽限期后使用 SIGKILL。
    pub(crate) async fn terminate_tree(
        &self,
        grace_period: Duration,
    ) -> Result<ProcessExit, PlatformError> {
        if let Some(exit) = self.try_wait()? {
            return Ok(exit);
        }

        let _termination = self.inner.termination.lock().await;
        if let Some(exit) = self.try_wait()? {
            return Ok(exit);
        }

        signal_process_group(self.inner.process_group_id, libc::SIGTERM, self.inner.kind)?;
        match tokio::time::timeout(grace_period, self.wait()).await {
            Ok(exit) => exit,
            Err(_) => {
                if self.try_wait()?.is_none() {
                    signal_process_group(
                        self.inner.process_group_id,
                        libc::SIGKILL,
                        self.inner.kind,
                    )?;
                }
                self.wait().await
            }
        }
    }
}

/// 创建独立进程组中的受控 DSH 进程树。
pub(super) fn spawn_dsh(port: u16) -> Result<SpawnedProcess, PlatformError> {
    spawn_dsh_command("dsh", port)
}

fn spawn_dsh_command(program: &str, port: u16) -> Result<SpawnedProcess, PlatformError> {
    let mut command = Command::new(program);
    command.args(["--profile", "web", "--port", &port.to_string(), "--no-open"]);
    spawn(command, ProcessKind::Dsh)
}

fn spawn(mut command: Command, kind: ProcessKind) -> Result<SpawnedProcess, PlatformError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut child = tokio::process::Command::from(command)
        .spawn()
        .map_err(|source| PlatformError::Spawn { kind, source })?;
    let Some(process_id) = child.id() else {
        return Err(rollback_spawn(
            child,
            kind,
            io::Error::other("子进程没有可用 PID"),
        ));
    };
    let native_pid = match libc::pid_t::try_from(process_id) {
        Ok(native_pid) => native_pid,
        Err(_) => {
            return Err(rollback_spawn(
                child,
                kind,
                io::Error::other("子进程 PID 超出平台范围"),
            ));
        }
    };

    let Some(stdout) = child.stdout.take() else {
        return Err(rollback_spawn(
            child,
            kind,
            io::Error::other("子进程 stdout 管道不可用"),
        ));
    };
    let Some(stderr) = child.stderr.take() else {
        return Err(rollback_spawn(
            child,
            kind,
            io::Error::other("子进程 stderr 管道不可用"),
        ));
    };

    let inner = Arc::new(UnixProcess {
        kind,
        leader_pid: LeaderPid(native_pid),
        process_group_id: ProcessGroupId(native_pid),
        exit: Mutex::new(None),
        exited: Notify::new(),
        termination: AsyncMutex::new(()),
    });
    tokio::spawn(reap_child(child, Arc::clone(&inner)));

    Ok(SpawnedProcess {
        process: ManagedProcess { inner },
        stdout,
        stderr,
    })
}

fn rollback_spawn(
    mut child: tokio::process::Child,
    kind: ProcessKind,
    source: io::Error,
) -> PlatformError {
    let source = match child.start_kill() {
        Ok(()) => {
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
            source
        }
        Err(rollback_source) => rollback_source,
    };
    PlatformError::Spawn { kind, source }
}

async fn reap_child(mut child: tokio::process::Child, process: Arc<UnixProcess>) {
    debug_assert_eq!(child.id(), Some(process.leader_pid.0 as u32));
    let result = child.wait().await.map(|status| ProcessExit {
        exit_code: status.code(),
    });
    let cached = result.map_err(WaitFailure::from_io);

    if let Ok(mut exit) = process.exit.lock() {
        *exit = Some(cached);
    }
    process.exited.notify_waiters();
}

fn signal_process_group(
    process_group_id: ProcessGroupId,
    signal: libc::c_int,
    kind: ProcessKind,
) -> Result<(), PlatformError> {
    // SAFETY: PGID 在成功 spawn 后由正 PID 构造；负 PGID 是 kill(2) 指定进程组的接口，
    // 调用不转移任何资源所有权。ESRCH 表示目标组已消失，按终止契约视为成功。
    let result = unsafe { libc::kill(-process_group_id.0, signal) };
    map_signal_result(result, kind)
}

fn map_signal_result(result: libc::c_int, kind: ProcessKind) -> Result<(), PlatformError> {
    if result == 0 {
        return Ok(());
    }

    let source = io::Error::last_os_error();
    if source.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(PlatformError::Control { kind, source })
    }
}

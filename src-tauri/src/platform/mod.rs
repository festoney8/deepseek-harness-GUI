use std::io;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
use unix as current;
#[cfg(windows)]
use windows as current;

pub(crate) use current::ManagedProcess;

/// 标识平台进程所属的业务用途。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessKind {
    /// 由专用生命周期管理器创建的 DSH 服务进程。
    Dsh,
}

/// 平台进程退出后缓存的系统级结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessExit {
    /// 系统提供的数字退出码；无法获取时为空。
    pub exit_code: Option<i32>,
}

/// 成功创建受控进程后移交给业务层的资源集合。
pub(crate) struct SpawnedProcess {
    /// 可克隆的进程树控制句柄。
    pub process: ManagedProcess,
    /// 供业务层并发读取的标准输出管道。
    pub stdout: tokio::process::ChildStdout,
    /// 供业务层并发读取的标准错误管道。
    pub stderr: tokio::process::ChildStderr,
}

/// 平台进程构建、启动、控制和等待阶段的内部错误。
#[derive(Debug, thiserror::Error)]
pub(crate) enum PlatformError {
    /// 无法根据业务进程类型构造平台命令。
    #[error("构建 {kind:?} 命令失败: {source}")]
    BuildCommand {
        /// 发生错误的业务进程类型。
        kind: ProcessKind,
        /// 平台 API 返回的原始错误。
        #[source]
        source: io::Error,
    },
    /// 无法创建目标进程或完成必要的资源绑定。
    #[error("启动 {kind:?} 进程失败: {source}")]
    Spawn {
        /// 发生错误的业务进程类型。
        kind: ProcessKind,
        /// 平台 API 返回的原始错误。
        #[source]
        source: io::Error,
    },
    /// 无法操作目标进程树的控制资源。
    #[error("控制 {kind:?} 进程树失败: {source}")]
    Control {
        /// 发生错误的业务进程类型。
        kind: ProcessKind,
        /// 平台 API 返回的原始错误。
        #[source]
        source: io::Error,
    },
    /// 无法等待或回收目标进程。
    #[error("等待 {kind:?} 进程失败: {source}")]
    Wait {
        /// 发生错误的业务进程类型。
        kind: ProcessKind,
        /// 平台 API 返回的原始错误。
        #[source]
        source: io::Error,
    },
}

/// 使用当前平台实现启动受控 DSH 进程树。
pub(crate) fn spawn_dsh(port: u16) -> Result<SpawnedProcess, PlatformError> {
    current::spawn_dsh(port)
}

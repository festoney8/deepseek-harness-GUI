use std::{path::Path, sync::Arc, time::Duration};

use super::{PlatformError, ProcessExit, ProcessKind, SpawnedProcess};

/// Windows 平台上可克隆的受控进程树句柄。
#[derive(Debug, Clone)]
pub(crate) struct ManagedProcess {
    /// 共享持有 Windows 平台进程控制资源。
    inner: Arc<WindowsProcess>,
}

/// 封装 Windows 进程树控制资源及共享退出状态。
#[derive(Debug)]
struct WindowsProcess {
    /// 创建进程时固定的业务用途。
    kind: ProcessKind,
}

impl ManagedProcess {
    /// 返回该进程创建时确定的业务类型。
    pub(crate) fn kind(&self) -> ProcessKind {
        todo!()
    }

    /// 非阻塞读取已经缓存的进程退出结果。
    pub(crate) fn try_wait(&self) -> Result<Option<ProcessExit>, PlatformError> {
        todo!()
    }

    /// 异步等待唯一监督任务缓存进程退出结果。
    pub(crate) async fn wait(&self) -> Result<ProcessExit, PlatformError> {
        todo!()
    }

    /// 终止 Windows Job Object 中的完整进程树。
    pub(crate) async fn terminate_tree(
        &self,
        grace_period: Duration,
    ) -> Result<ProcessExit, PlatformError> {
        todo!()
    }
}

/// 使用 Windows Shell 解释器创建受控命令进程树。
pub(super) fn spawn_shell(command: &str, cwd: &Path) -> Result<SpawnedProcess, PlatformError> {
    todo!()
}

/// 使用 Windows 命令入口创建受控 DSH 进程树。
pub(super) fn spawn_dsh(port: u16) -> Result<SpawnedProcess, PlatformError> {
    todo!()
}

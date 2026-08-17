use std::path::PathBuf;

use super::BackendError;

/// 前端提交的完整 Shell 命令请求。
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellRequest {
    /// 交给当前平台固定解释器执行的命令文本。
    pub command: String,
    /// 可选工作目录，允许绝对路径或相对用户目录的路径。
    pub cwd: Option<PathBuf>,
    /// 可选超时时间，单位为毫秒。
    pub timeout_ms: Option<i64>,
}

/// Shell 命令结束后返回给前端的完整执行结果。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellResult {
    /// 进程退出码；超时或无数字退出码时为空。
    pub exit_code: Option<i32>,
    /// 使用 UTF-8 宽松解码后的标准输出。
    pub stdout: String,
    /// 使用 UTF-8 宽松解码后的标准错误。
    pub stderr: String,
    /// 根据退出状态映射的业务执行状态。
    pub status: ShellStatus,
}

/// Shell 命令在业务层使用的终态。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ShellStatus {
    /// 命令以零退出码结束。
    Success,
    /// 命令以非零退出码或无退出码状态结束。
    Failed,
    /// 命令超过请求的执行时限。
    Timeout,
}

/// 校验请求并在受控进程树中执行 Shell 命令。
pub(crate) async fn shell(request: ShellRequest) -> Result<ShellResult, BackendError> {
    todo!()
}

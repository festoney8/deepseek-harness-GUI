use crate::platform::PlatformError;

/// 后端业务层统一返回的内部错误
#[derive(Debug, thiserror::Error)]
pub(crate) enum BackendError {
    /// 请求的超时时间不大于零
    #[error("超时时间必须大于零")]
    InvalidTimeout,
    /// 远程主机格式不受支持
    #[error("主机地址无效")]
    InvalidHost,
    /// 请求的网络协议不受支持
    #[error("协议无效")]
    InvalidProtocol,
    /// 请求端口不在有效范围内
    #[error("端口无效")]
    InvalidPort,
    /// 目标服务未能通过可用性探测
    #[error("服务不可用")]
    ServiceUnavailable,
    /// 本地 DSH 启动端口已被占用
    #[error("端口已被占用")]
    PortOccupied,
    /// 已有生命周期变更操作正在执行
    #[error("另一个生命周期操作正在进行")]
    OperationInProgress,
    /// 当前已经维护着运行中的 DSH 实例
    #[error("DSH 已经在运行")]
    DshAlreadyRunning,
    /// 当前没有可停止的 DSH 实例
    #[error("DSH 当前未运行")]
    ProcessNotRunning,
    /// DSH 受控进程创建失败
    #[error("无法启动 DSH 进程")]
    DshSpawnFailed,
    /// DSH 未在规定时间内完成就绪
    #[error("DSH 启动超时")]
    DshStartTimeout,
    /// DSH 在完成就绪探测前退出
    #[error("DSH 在就绪前退出")]
    DshExitedEarly,
    /// 系统无法打开本次启动的日志目录
    #[error("无法打开日志目录")]
    OpenLogsFailed,
    /// 系统无法创建本次启动的独立日志目录
    #[error("无法创建日志目录")]
    LogDirCreateFailed,
    /// 系统托盘、菜单或主窗口控制失败
    #[error("托盘或窗口操作失败: {0}")]
    Tray(#[from] tauri::Error),
    /// 动态 URL 窗口控制或创建失败
    #[error("URL 窗口操作失败: {0}")]
    Window(tauri::Error),
    /// 动态 URL 窗口状态锁已损坏
    #[error("URL 窗口状态不可用")]
    WindowStatePoisoned,
    /// 动态 URL 无效或不符合外部页面加载策略
    #[error("URL 无效")]
    InvalidWindowUrl,
    /// 找不到主窗口或应用的默认托盘图标
    #[error("主窗口或默认图标不可用")]
    WindowResourceMissing,
    /// 平台进程管理层返回的类型化错误
    #[error(transparent)]
    Platform(#[from] PlatformError),
}

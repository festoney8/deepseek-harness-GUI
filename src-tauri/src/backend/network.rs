use std::time::Duration;

use super::BackendError;

/// 检查指定主机端口是否可建立 TCP 连接。
pub(crate) async fn check_tcp(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<bool, BackendError> {
    todo!()
}

/// 检查指定 HTTP 地址是否满足服务可用条件。
pub(crate) async fn check_http(url: &str, timeout: Duration) -> Result<bool, BackendError> {
    todo!()
}

/// 检查指定 HTTPS 地址是否满足服务可用条件。
pub(crate) async fn check_https(url: &str, timeout: Duration) -> Result<bool, BackendError> {
    todo!()
}

/// 并行探测远程服务并返回优先使用的规范化地址。
pub(crate) async fn connect_remote(host: String, port: u16) -> Result<String, BackendError> {
    todo!()
}

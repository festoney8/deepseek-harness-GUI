use std::{net::Ipv4Addr, time::Duration};

use super::BackendError;

/// 远程 HTTP/HTTPS 单协议探测的超时时间。
const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// 使用 TCP 连接结果判断目标端口当前是否可达。
///
/// 连接失败和超时都表示端口不可达，而不是后端业务错误；只有零超时
/// 属于调用参数错误。
pub(crate) async fn check_tcp(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<bool, BackendError> {
    if timeout.is_zero() {
        return Err(BackendError::InvalidTimeout);
    }

    Ok(
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect((host, port)))
            .await
            .is_ok_and(|result| result.is_ok()),
    )
}

/// 按指定协议探测 `HOST:PORT` 的 HTTP 服务。
///
/// 协议只接受小写 `http` 或 `https`，其他值属于参数错误。
/// 探测失败（URL 解析失败、连接失败、超时、非 2xx 响应）统一表示为
/// `Ok(false)`，语义由调用方决定；只有参数错误作为业务错误返回。
pub(crate) async fn check_url(
    protocol: &str,
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<bool, BackendError> {
    if timeout.is_zero() {
        return Err(BackendError::InvalidTimeout);
    }
    if protocol != "http" && protocol != "https" {
        return Err(BackendError::InvalidProtocol);
    }

    let url = format!("{protocol}://{host}:{port}");
    let Ok(parsed_url) = reqwest::Url::parse(&url) else {
        return Ok(false);
    };
    if parsed_url.scheme() != protocol {
        return Ok(false);
    }

    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|_| BackendError::ServiceUnavailable)?;
    let Ok(response) = client.get(parsed_url).send().await else {
        return Ok(false);
    };

    Ok(response.status().is_success())
}

/// 按用户指定协议探测远程服务并返回对应地址。
pub(crate) async fn connect_remote(
    protocol: String,
    host: String,
    port: u16,
) -> Result<String, BackendError> {
    if port == 0 {
        return Err(BackendError::InvalidPort);
    }

    let host = normalize_host(&host)?;
    if check_url(&protocol, &host, port, REMOTE_PROBE_TIMEOUT).await? {
        log::info!("remote service available: {protocol}://{host}:{port}");
        Ok(format!("{protocol}://{host}:{port}"))
    } else {
        log::warn!(
            "remote service unavailable: protocol={}, host={host}, port={port}",
            protocol
        );
        Err(BackendError::ServiceUnavailable)
    }
}

/// 将输入主机校验并转换为连接地址使用的规范文本。
fn normalize_host(host: &str) -> Result<String, BackendError> {
    if host.eq_ignore_ascii_case("localhost") {
        return Ok(String::from("localhost"));
    }

    host.parse::<Ipv4Addr>()
        .map(|address| address.to_string())
        .map_err(|_| BackendError::InvalidHost)
}
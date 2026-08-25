use std::{net::Ipv4Addr, time::Duration};

use super::BackendError;

/// 远程 HTTP/HTTPS 单协议探测的超时时间
const REMOTE_PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// 使用 TCP 连接结果判断目标端口当前是否可达
///
/// 连接失败和超时都表示端口不可达，而不是后端业务错误；只有零超时
/// 属于调用参数错误
pub(crate) async fn check_tcp(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<bool, BackendError> {
    if timeout.is_zero() {
        log::warn!("tcp probe rejected: host={host}, port={port}, reason=zero timeout");
        return Err(BackendError::InvalidTimeout);
    }

    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect((host, port))).await {
        Ok(Ok(_)) => {
            log::debug!("tcp probe succeeded: host={host}, port={port}");
            Ok(true)
        }
        Ok(Err(error)) => {
            log::debug!("tcp probe failed: host={host}, port={port}, error={error}");
            Ok(false)
        }
        Err(_) => {
            log::debug!("tcp probe timed out: host={host}, port={port}");
            Ok(false)
        }
    }
}

/// 按指定协议探测 `HOST:PORT` 的 HTTP 服务
///
/// 协议只接受小写 `http` 或 `https`，其他值属于参数错误
/// 探测失败（URL 解析失败、连接失败、超时、非 2xx 响应且非 401）统一表示为
/// `Ok(false)`，语义由调用方决定；只有参数错误作为业务错误返回
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
        .map_err(|error| {
            log::error!("http client build failed: {error}");
            BackendError::ServiceUnavailable
        })?;
    let response = match client.get(parsed_url).send().await {
        Ok(response) => response,
        Err(error) => {
            log::debug!("http probe request failed: url={url}, error={error}");
            return Ok(false);
        }
    };

    let status = response.status();
    if status.is_success() || status == reqwest::StatusCode::UNAUTHORIZED {
        log::debug!("http probe succeeded: url={url}, status={status}");
        Ok(true)
    } else {
        log::debug!("http probe returned non-success status: url={url}, status={status}");
        Ok(false)
    }
}

/// 按用户指定协议探测远程服务并返回对应地址
pub(crate) async fn connect_remote(
    protocol: String,
    host: String,
    port: u16,
) -> Result<String, BackendError> {
    log::info!("checking remote service: protocol={protocol}, host={host}, port={port}");
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

/// 将输入主机校验并转换为连接地址使用的规范文本
fn normalize_host(host: &str) -> Result<String, BackendError> {
    if host.eq_ignore_ascii_case("localhost") {
        log::debug!("remote host normalized: input={host}, normalized=localhost");
        return Ok(String::from("localhost"));
    }

    host.parse::<Ipv4Addr>()
        .map(|address| {
            let normalized = address.to_string();
            log::debug!("remote host normalized: input={host}, normalized={normalized}");
            normalized
        })
        .map_err(|_| {
            log::warn!("remote host rejected: host={host}, reason=invalid IPv4 address");
            BackendError::InvalidHost
        })
}

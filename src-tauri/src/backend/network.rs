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

/// 检查指定 HTTP 地址是否满足服务可用条件。
pub(crate) async fn check_http(url: &str, timeout: Duration) -> Result<bool, BackendError> {
    check_http_url(url, "http", timeout).await
}

/// 检查指定 HTTPS 地址是否满足服务可用条件。
pub(crate) async fn check_https(url: &str, timeout: Duration) -> Result<bool, BackendError> {
    check_http_url(url, "https", timeout).await
}

/// 按指定协议探测 HTTP 服务。
async fn check_http_url(
    url: &str,
    expected_scheme: &str,
    timeout: Duration,
) -> Result<bool, BackendError> {
    if timeout.is_zero() {
        return Err(BackendError::InvalidTimeout);
    }

    let Ok(parsed_url) = reqwest::Url::parse(url) else {
        return Ok(false);
    };
    if parsed_url.scheme() != expected_scheme {
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

/// 并行探测远程服务并返回优先使用的规范化地址。
pub(crate) async fn connect_remote(host: String, port: u16) -> Result<String, BackendError> {
    if port == 0 {
        return Err(BackendError::InvalidPort);
    }

    let host = normalize_host(&host)?;
    let http_url = format!("http://{host}:{port}");
    let https_url = format!("https://{host}:{port}");
    let http_probe = check_http(&http_url, REMOTE_PROBE_TIMEOUT);
    let https_probe = check_https(&https_url, REMOTE_PROBE_TIMEOUT);
    tokio::pin!(http_probe);
    tokio::pin!(https_probe);

    let mut http_done = false;
    let mut https_done = false;
    let mut http_available = false;
    loop {
        if https_done && http_available {
            log::info!("remote HTTP service available after HTTPS fallback: {http_url}");
            return Ok(http_url.clone());
        }
        if http_done && https_done {
            log::warn!("remote service unavailable: host={host}, port={port}");
            return Err(BackendError::ServiceUnavailable);
        }

        tokio::select! {
            result = &mut https_probe, if !https_done => {
                https_done = true;
                match result {
                    Ok(true) => {
                        log::info!("remote HTTPS service available: {https_url}");
                        return Ok(https_url.clone());
                    }
                    Ok(false) => log::debug!("remote HTTPS probe failed: {https_url}"),
                    Err(error) => log::warn!("remote HTTPS probe errored: {https_url}, error={error:?}"),
                }
            }
            result = &mut http_probe, if !http_done => {
                http_done = true;
                match result {
                    Ok(true) => {
                        http_available = true;
                        if https_done {
                            log::info!("remote HTTP service available after HTTPS fallback: {http_url}");
                            return Ok(http_url.clone());
                        }
                        log::debug!("remote HTTP service available; waiting for HTTPS: {http_url}");
                    }
                    Ok(false) => log::debug!("remote HTTP probe failed: {http_url}"),
                    Err(error) => log::warn!("remote HTTP probe errored: {http_url}, error={error:?}"),
                }
            }
        }
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

use std::time::Duration;

use super::BackendError;

/// 检查指定主机端口是否可建立 TCP 连接。
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
    todo!()
}

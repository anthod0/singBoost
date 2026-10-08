use base64::Engine;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum HttpError {
    #[error("request failed: {0}")]
    Request(String),
}

pub(crate) fn get_bytes(url: &str, timeout: Duration, max_size: u64) -> Result<Vec<u8>, HttpError> {
    get_bytes_impl(url, timeout, max_size, None)
}

pub(crate) fn get_bytes_with_basic_auth(
    url: &str,
    timeout: Duration,
    max_size: u64,
    credentials: Option<(&str, &str)>,
) -> Result<Vec<u8>, HttpError> {
    let authorization = credentials.map(|(username, password)| {
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"))
        )
    });
    get_bytes_impl(url, timeout, max_size, authorization.as_deref())
}

fn get_bytes_impl(
    url: &str,
    timeout: Duration,
    max_size: u64,
    authorization: Option<&str>,
) -> Result<Vec<u8>, HttpError> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .user_agent(concat!("SingBoost/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent: ureq::Agent = config.into();
    let mut request = agent.get(url);
    if let Some(authorization) = authorization {
        request = request.header("Authorization", authorization);
    }
    let mut response = request
        .call()
        .map_err(|err| HttpError::Request(err.to_string()))?;
    let body = response
        .body_mut()
        .with_config()
        .limit(max_size.saturating_add(1))
        .read_to_vec()
        .map_err(|err| HttpError::Request(err.to_string()))?;
    if body.len() as u64 > max_size {
        Err(HttpError::Request(format!(
            "response exceeds the {max_size}-byte limit"
        )))
    } else {
        Ok(body)
    }
}

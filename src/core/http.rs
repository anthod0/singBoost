use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum HttpError {
    #[error("request failed: {0}")]
    Request(String),
}

pub(crate) fn get_bytes(url: &str, timeout: Duration, max_size: u64) -> Result<Vec<u8>, HttpError> {
    get_bytes_impl(url, timeout, max_size)
}

#[cfg(not(windows))]
fn get_bytes_impl(url: &str, timeout: Duration, max_size: u64) -> Result<Vec<u8>, HttpError> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .user_agent(concat!("SingBoost/", env!("CARGO_PKG_VERSION")))
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .get(url)
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

#[cfg(windows)]
fn get_bytes_impl(url: &str, timeout: Duration, max_size: u64) -> Result<Vec<u8>, HttpError> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let timeout_ms = timeout.as_millis().min(i32::MAX as u128).to_string();
    let max_size_text = max_size.to_string();
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "$ErrorActionPreference='Stop'; $url=$env:SINGBOOST_HTTP_URL; $timeout=[int]$env:SINGBOOST_HTTP_TIMEOUT_MS; $max=[long]$env:SINGBOOST_HTTP_MAX_SIZE; $deadline=[DateTime]::UtcNow.AddMilliseconds($timeout); $request=[System.Net.HttpWebRequest]::Create($url); $request.Timeout=$timeout; $request.ReadWriteTimeout=$timeout; $request.UserAgent=$env:SINGBOOST_HTTP_USER_AGENT; $response=$request.GetResponse(); try { if ($response.ContentLength -gt $max) { throw 'response exceeds size limit' }; $stream=$response.GetResponseStream(); $ms=New-Object System.IO.MemoryStream; $buffer=New-Object byte[] 81920; $total=0L; while ($true) { $remaining=[int][Math]::Max(0,($deadline-[DateTime]::UtcNow).TotalMilliseconds); if ($remaining -le 0) { throw 'request timed out' }; $task=$stream.ReadAsync($buffer,0,$buffer.Length); if (!$task.Wait($remaining)) { $request.Abort(); throw 'request timed out' }; $read=$task.Result; if ($read -le 0) { break }; $total += $read; if ($total -gt $max) { throw 'response exceeds size limit' }; $ms.Write($buffer,0,$read) }; $bytes=$ms.ToArray(); [Console]::OpenStandardOutput().Write($bytes,0,$bytes.Length) } finally { if ($response) { $response.Close() } }",
        ])
        .env("SINGBOOST_HTTP_URL", url)
        .env("SINGBOOST_HTTP_TIMEOUT_MS", timeout_ms)
        .env("SINGBOOST_HTTP_MAX_SIZE", max_size_text)
        .env(
            "SINGBOOST_HTTP_USER_AGENT",
            concat!("SingBoost/", env!("CARGO_PKG_VERSION")),
        )
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| HttpError::Request(err.to_string()))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(HttpError::Request(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

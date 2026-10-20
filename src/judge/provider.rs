//! One bounded, non-retrying TypeSafe exchange. No action follows its answer.
use crate::Result;
use reqwest::{blocking::Client, redirect::Policy};
use std::{fs, io::Read, path::Path, time::Duration};

pub const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";

pub struct Exchange {
    pub status: Option<u16>,
    pub body: Vec<u8>,
    pub error: Option<&'static str>,
}

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .no_proxy()
        .https_only(true)
        .redirect(Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(Duration::from_secs(10))
        .build()?)
}

pub fn post(client: &Client, key: &str, body: &[u8], timeout: Duration, cap: usize) -> Exchange {
    exchange(client, ENDPOINT, key, body, timeout, cap)
}

fn exchange(
    client: &Client,
    endpoint: &str,
    key: &str,
    body: &[u8],
    timeout: Duration,
    cap: usize,
) -> Exchange {
    let response = client
        .post(endpoint)
        .bearer_auth(key)
        .header("Content-Type", "application/json")
        .body(body.to_vec())
        .timeout(timeout)
        .send();
    let mut response = match response {
        Ok(value) => value,
        Err(error) => {
            return Exchange {
                status: None,
                body: vec![],
                error: Some(if error.is_timeout() {
                    "request_timeout"
                } else {
                    "transport_failure"
                }),
            };
        }
    };
    let status = Some(response.status().as_u16());
    let mut bytes = Vec::new();
    let read = response
        .by_ref()
        .take(cap as u64 + 1)
        .read_to_end(&mut bytes);
    let error = if bytes.len() > cap {
        bytes.truncate(cap);
        Some("response_byte_limit")
    } else if read.is_err() {
        Some("response_read_failure")
    } else {
        None
    };
    Exchange {
        status,
        body: bytes,
        error,
    }
}

/// No env-file execution, shell expansion or discovery in another project.
pub fn credential(file: Option<&Path>) -> Result<String> {
    if file.is_none()
        && let Ok(key) = std::env::var("TYPESAFE_API_KEY")
    {
        return validate_key(key);
    }
    let default = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .map(|p| p.join(".config/paperhands/typesafe.env"));
    let path = file
        .or(default.as_deref())
        .ok_or("set TYPESAFE_API_KEY or pass --key-file")?;
    let meta = fs::symlink_metadata(path).map_err(
        |_| "TypeSafe credential file unavailable; set TYPESAFE_API_KEY or pass --key-file",
    )?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() > 8192 {
        return Err("credential must be a regular private file of at most 8192 bytes".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if meta.permissions().mode() & 0o077 != 0 {
            return Err("credential file must have mode 0600 or stricter".into());
        }
    }
    let text = fs::read_to_string(path).map_err(|_| "credential file is unreadable UTF-8")?;
    let mut found = None;
    for line in text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
    {
        let line = line.strip_prefix("export ").unwrap_or(line);
        let (name, value) = line
            .split_once('=')
            .ok_or("invalid credential declaration; content withheld")?;
        if name.trim() != "TYPESAFE_API_KEY" || found.is_some() {
            return Err(
                "credential file must declare TYPESAFE_API_KEY once and no other variables".into(),
            );
        }
        let value = value.trim();
        let value = value
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
            .or_else(|| value.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
            .unwrap_or(value);
        found = Some(validate_key(value.into())?);
    }
    found.ok_or_else(|| "credential file has no TYPESAFE_API_KEY".into())
}

fn validate_key(key: String) -> Result<String> {
    if key.is_empty()
        || key.len() > 4096
        || !key
            .bytes()
            .all(|c| c.is_ascii_graphic() && !b"'\"`$\\".contains(&c))
    {
        return Err("invalid TypeSafe credential; content withheld".into());
    }
    Ok(key)
}

/// Provider error pages can reflect Authorization. Keep bytes except the secret.
pub fn redact(bytes: &[u8], key: &str) -> (Vec<u8>, bool) {
    let secret = key.as_bytes();
    let mut result = Vec::new();
    let mut at = 0;
    let mut changed = false;
    while at < bytes.len() {
        if bytes[at..].starts_with(secret) {
            result.extend_from_slice(b"[REDACTED]");
            at += secret.len();
            changed = true;
        } else {
            result.push(bytes[at]);
            at += 1;
        }
    }
    (result, changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener, thread};

    #[test]
    fn real_http_exchange_bounds_body_and_never_follows_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            for reply in [
                "HTTP/1.1 302 Found\r\nContent-Length: 0\r\nLocation: http://127.0.0.1:1/leak\r\nConnection: close\r\n\r\n",
                "HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\npartial-body",
            ] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let n = stream.read(&mut request).unwrap();
                assert!(String::from_utf8_lossy(&request[..n]).contains("Bearer fake-test-key"));
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .retry(reqwest::retry::never())
            .build()
            .unwrap();
        let url = format!("http://{address}/test");
        let first = exchange(
            &client,
            &url,
            "fake-test-key",
            b"{}",
            Duration::from_secs(1),
            4,
        );
        assert_eq!(first.status, Some(302));
        let second = exchange(
            &client,
            &url,
            "fake-test-key",
            b"{}",
            Duration::from_secs(1),
            4,
        );
        assert_eq!(second.body, b"part");
        assert_eq!(second.error, Some("response_byte_limit"));
        server.join().unwrap();
        assert_eq!(
            redact(b"a fake-test-key b", "fake-test-key"),
            (b"a [REDACTED] b".to_vec(), true)
        );
    }
}

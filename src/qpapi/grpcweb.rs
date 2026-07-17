use anyhow::{anyhow, Context, Result};
use base64::Engine;
use prost::Message;
use reqwest::blocking::{Client as HttpClient, RequestBuilder};
use reqwest::header::{ACCEPT, CONTENT_TYPE, SET_COOKIE};
use std::collections::BTreeMap;
use std::time::Duration;

use super::pb;

#[derive(Debug, Clone)]
pub struct Client {
    pub host: String,
    pub cookie: String,
    pub window_id: String,
    http: HttpClient,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct GrpcError {
    pub code: String,
    pub app_code: i32,
    pub message: String,
    pub domain: String,
}

impl GrpcError {
    pub fn hint(&self) -> Option<&'static str> {
        let message = self.message.to_ascii_lowercase();
        if is_auth_expired_message(&message) {
            Some("Your QueryPie session expired. Run `querypie auth login`, then retry.")
        } else if self.app_code == 10308 || message.contains("privilege has deactivated") {
            Some("Your access to this connection has expired. Request access again in QueryPie.")
        } else if self.app_code == 14000 || message.contains("ledger table policy") {
            Some("This object is governed by a Ledger Table Policy and cannot be queried ad hoc.")
        } else if message.contains("connect timeout") {
            Some("QueryPie could not reach the database. Check the connection/instance, or try again.")
        } else if message.contains("sessionnotfound") {
            Some("The cached session is stale. Run `querypie session clear` and retry.")
        } else {
            None
        }
    }

    pub fn is_session_not_found(&self) -> bool {
        self.code == "10" && self.message.contains("SessionNotFound")
    }

    pub fn is_auth_expired(&self) -> bool {
        is_auth_expired_message(&self.message.to_ascii_lowercase())
    }
}

impl Client {
    pub fn new(
        host: impl Into<String>,
        cookie: impl Into<String>,
        window_id: impl Into<String>,
    ) -> Result<Self> {
        Self::new_with_timeout(host, cookie, window_id, Duration::from_secs(60))
    }

    pub fn new_with_timeout(
        host: impl Into<String>,
        cookie: impl Into<String>,
        window_id: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self> {
        Ok(Self {
            host: host.into(),
            cookie: cookie.into(),
            window_id: window_id.into(),
            http: HttpClient::builder().timeout(timeout).build()?,
        })
    }

    pub(crate) fn unary<Req, Resp>(&self, method: &str, req: &Req) -> Result<Resp>
    where
        Req: Message,
        Resp: Message + Default,
    {
        let data = self.send_unary(method, req)?;
        Ok(Resp::decode(data.as_slice())?)
    }

    pub(crate) fn send_unary<Req>(&self, method: &str, req: &Req) -> Result<Vec<u8>>
    where
        Req: Message,
    {
        let mut body = Vec::new();
        req.encode(&mut body)?;
        self.call_raw(method, &body)
    }

    fn call_raw(&self, method: &str, msg: &[u8]) -> Result<Vec<u8>> {
        let url = engine_grpc_url(&self.host, method);
        let resp = grpc_web_post(&self.http, url, &self.cookie, Some(&self.window_id), msg)
            .send()
            .context("send grpc-web request")?;

        check_response_headers(resp.headers())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow!("{method}: HTTP {}", status.as_u16()));
        }

        let body = resp.text().context("read grpc-web body")?;
        let frames = decode_frames(&body)?;
        let mut data = Vec::new();
        for frame in frames {
            if frame.flag & 0x80 != 0 {
                check_trailer(&frame.payload)?;
            } else {
                data.extend_from_slice(&frame.payload);
            }
        }
        Ok(data)
    }
}

pub fn refresh_access_token(host: &str, cookies: &str) -> Result<Option<Vec<String>>> {
    if !cookie_header_contains(cookies, "qp_refresh_token") {
        return Ok(None);
    }
    let url = engine_grpc_url(host, "api.user.AccountService/RefreshToken");
    let mut body = Vec::new();
    pb::RefreshTokenRequest::default().encode(&mut body)?;
    let http = HttpClient::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let resp = grpc_web_post(&http, url, cookies, None, &body)
        .send()
        .context("send refresh token request")?;

    if !resp.status().is_success() {
        return Ok(None);
    }
    if response_grpc_status(resp.headers())
        .as_deref()
        .is_some_and(|status| status != "0")
    {
        return Ok(None);
    }
    let set_cookies = resp
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !set_cookies
        .iter()
        .any(|cookie| set_cookie_name(cookie).as_deref() == Some("qp_access_token"))
    {
        return Ok(None);
    }
    let body = resp.text().context("read refresh token body")?;
    if grpc_web_status(&body)
        .as_deref()
        .is_some_and(|status| status != "0")
    {
        return Ok(None);
    }
    Ok(Some(set_cookies))
}

fn engine_grpc_url(host: &str, method: &str) -> String {
    format!("https://{host}/engine-grpc/{method}")
}

fn grpc_web_post(
    http: &HttpClient,
    url: String,
    cookies: &str,
    window_id: Option<&str>,
    msg: &[u8],
) -> RequestBuilder {
    let mut request = http
        .post(url)
        .header(CONTENT_TYPE, "application/grpc-web-text")
        .header(ACCEPT, "application/grpc-web-text")
        .header("X-Grpc-Web", "1")
        .header("X-User-Agent", "grpc-web-javascript/0.1")
        .header("Cookie", cookies)
        .body(encode_frame(msg));

    if let Some(window_id) = window_id {
        request = request.header("X-QueryPie-Window-Id", window_id);
    }

    request
}

fn response_grpc_status(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("grpc-status")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn check_response_headers(headers: &reqwest::header::HeaderMap) -> Result<()> {
    let Some(status) = headers.get("grpc-status").and_then(|v| v.to_str().ok()) else {
        return Ok(());
    };
    if status == "0" {
        return Ok(());
    }
    let message = headers
        .get("grpc-message")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    Err(new_grpc_error(status, message).into())
}

#[derive(Debug)]
struct Frame {
    flag: u8,
    payload: Vec<u8>,
}

pub fn encode_frame(msg: &[u8]) -> String {
    let mut buf = Vec::with_capacity(5 + msg.len());
    buf.push(0);
    buf.extend_from_slice(&(msg.len() as u32).to_be_bytes());
    buf.extend_from_slice(msg);
    base64::engine::general_purpose::STANDARD.encode(buf)
}

fn decode_frames(body: &str) -> Result<Vec<Frame>> {
    let raw = decode_grpc_web_text(body.trim())?;
    let mut frames = Vec::new();
    let mut i = 0usize;
    while i + 5 <= raw.len() {
        let flag = raw[i];
        let len = u32::from_be_bytes([raw[i + 1], raw[i + 2], raw[i + 3], raw[i + 4]]) as usize;
        i += 5;
        if i + len > raw.len() {
            return Err(anyhow!("grpc-web: truncated frame"));
        }
        frames.push(Frame {
            flag,
            payload: raw[i..i + len].to_vec(),
        });
        i += len;
    }
    Ok(frames)
}

fn decode_grpc_web_text(body: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            let mut end = i + 1;
            if end < bytes.len() && bytes[end] == b'=' {
                end += 1;
            }
            out.extend(base64::engine::general_purpose::STANDARD.decode(&body[start..end])?);
            start = end;
            i = end;
        } else {
            i += 1;
        }
    }
    if start < body.len() {
        out.extend(base64::engine::general_purpose::STANDARD.decode(&body[start..])?);
    }
    Ok(out)
}

fn parse_trailer(payload: &[u8]) -> (Option<String>, Option<String>) {
    let mut status = None;
    let mut message = None;
    for line in String::from_utf8_lossy(payload).split("\r\n") {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        match k.trim().to_ascii_lowercase().as_str() {
            "grpc-status" => status = Some(v.trim().to_string()),
            "grpc-message" => message = Some(v.trim().to_string()),
            _ => {}
        }
    }
    (status, message)
}

fn check_trailer(payload: &[u8]) -> Result<()> {
    let (status, message) = parse_trailer(payload);
    match status.as_deref() {
        Some("0") | None => Ok(()),
        Some(status) => Err(new_grpc_error(status, message.as_deref().unwrap_or_default()).into()),
    }
}

fn grpc_web_status(body: &str) -> Option<String> {
    let frames = decode_frames(body).ok()?;
    for frame in frames {
        if frame.flag & 0x80 != 0 {
            let (status, _) = parse_trailer(&frame.payload);
            return status;
        }
    }
    None
}

fn cookie_header_contains(cookies: &str, name: &str) -> bool {
    parse_cookie_header(cookies)
        .get(name)
        .is_some_and(|value| !value.is_empty())
}

fn parse_cookie_header(cookies: &str) -> BTreeMap<String, String> {
    cookies
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .map(|(name, value)| (name.trim().to_string(), value.trim().to_string()))
        .filter(|(name, _)| !name.is_empty())
        .collect()
}

fn set_cookie_name(set_cookie: &str) -> Option<String> {
    set_cookie_pair(set_cookie).map(|(name, _)| name.trim().to_string())
}

fn set_cookie_pair(set_cookie: &str) -> Option<(&str, &str)> {
    set_cookie.split(';').next()?.split_once('=')
}

fn new_grpc_error(code: &str, grpc_message: &str) -> GrpcError {
    let (app_code, message, domain) = parse_status(grpc_message);
    GrpcError {
        code: code.to_string(),
        app_code,
        message,
        domain,
    }
}

fn is_auth_expired_message(message: &str) -> bool {
    message.contains("access token expired")
        || message.contains("invalid token")
        || message.contains("no_user")
}

fn parse_status(input: &str) -> (i32, String, String) {
    let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(input) else {
        return (0, input.to_string(), String::new());
    };
    let Ok(error) = pb::CommonError::decode(raw.as_slice()) else {
        return (
            0,
            raw.iter()
                .filter(|b| **b >= 0x20 && **b < 0x7f)
                .map(|b| *b as char)
                .collect(),
            String::new(),
        );
    };
    let app_code = error.code;
    let mut message = error.message.map(|value| value.value).unwrap_or_default();
    let domain = error.source.map(|value| value.value).unwrap_or_default();
    if message.is_empty() {
        message = raw
            .iter()
            .filter(|b| **b >= 0x20 && **b < 0x7f)
            .map(|b| *b as char)
            .collect();
    }
    (app_code, message, domain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_roundtrip() -> Result<()> {
        // given
        let payload = [1, 2, 3];

        // when
        let body = encode_frame(&payload);
        let frames = decode_frames(&body)?;

        // then
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].flag, 0);
        assert_eq!(frames[0].payload, payload);
        Ok(())
    }

    #[test]
    fn rejects_truncated_frame() {
        // given
        let body = base64::engine::general_purpose::STANDARD.encode([0, 0, 0, 0, 3, 1]);

        // when
        let result = decode_frames(&body);

        // then
        assert!(result.is_err());
    }

    #[test]
    fn set_cookie_updates_cookie_header() {
        // given
        let cookies = "qp_access_token=old; qp_refresh_token=refresh";
        let set_cookies = [
            "qp_access_token=new; Path=/; HttpOnly".to_string(),
            "qp_refresh_token=rotated; Path=/engine-grpc/api.user.AccountService/RefreshToken"
                .to_string(),
        ];

        // when
        let merged = merge_set_cookies(cookies, &set_cookies);
        let parsed = parse_cookie_header(&merged);

        // then
        assert_eq!(
            parsed.get("qp_access_token").map(String::as_str),
            Some("new")
        );
        assert_eq!(
            parsed.get("qp_refresh_token").map(String::as_str),
            Some("rotated")
        );
    }

    fn merge_set_cookies(cookies: &str, set_cookies: &[String]) -> String {
        let mut parsed = parse_cookie_header(cookies);
        for set_cookie in set_cookies {
            let Some((name, value)) = set_cookie_pair(set_cookie) else {
                continue;
            };
            let name = name.trim();
            if !name.is_empty() {
                parsed.insert(name.to_string(), value.trim().to_string());
            }
        }
        parsed
            .into_iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

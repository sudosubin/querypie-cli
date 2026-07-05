use anyhow::{bail, Result};

use super::{webview, AuthSession};
use crate::lockfile::HostLock;
use crate::paths;
use crate::qpapi::{Client, GrpcError};
use crate::sessioncache;

use super::status::AuthCheck;

#[derive(Debug, Clone)]
pub struct AuthService {
    host: String,
}

impl AuthService {
    pub fn new(host: impl AsRef<str>) -> Result<Self> {
        let host = paths::normalize_host(host.as_ref());
        if host.trim().is_empty() {
            bail!("auth: host is required");
        }
        Ok(Self { host })
    }

    pub fn login(&self) -> Result<AuthSession> {
        let _lock = HostLock::acquire(&self.host)?;
        self.login_with_lock_held()
    }

    fn login_with_lock_held(&self) -> Result<AuthSession> {
        webview::clear_profile(&self.host)?;
        webview::login(&self.host)
    }

    pub fn logout(&self) -> Result<()> {
        webview::clear_profile(&self.host)
    }

    fn read_cookie_via_child(&self) -> Result<Option<String>> {
        let output = std::process::Command::new(std::env::current_exe()?)
            .arg("--host")
            .arg(&self.host)
            .arg("auth")
            .arg("read-cookie")
            .stderr(std::process::Stdio::null())
            .output()?;
        cookie_from_stdout(output.stdout)
    }

    pub(crate) fn read_cookie_in_process(&self) -> Result<Option<String>> {
        webview::read_cookies(&self.host)
    }

    pub fn read_or_login_cookie(&self) -> Result<String> {
        if let Some(cookies) = self.read_cookie_via_child()? {
            return Ok(cookies);
        }
        self.login_if_previously_authenticated(|| {
            format!(
                "not logged in to {}; run `querypie --host {} auth login` first",
                self.host, self.host
            )
        })
        .map(|session| session.cookies)
    }

    pub fn check(&self) -> Result<AuthCheck> {
        let Some(cookies) = self.read_cookie_via_child()? else {
            return Ok(AuthCheck::missing(self.host.clone()));
        };
        if let Some(check) = self.check_cookies(&cookies)? {
            return Ok(check);
        }

        let Some(cookies) = self.refresh_cookie_via_child()? else {
            return Ok(AuthCheck::expired(self.host.clone()));
        };
        Ok(self
            .check_cookies(&cookies)?
            .unwrap_or_else(|| AuthCheck::expired(self.host.clone())))
    }

    pub fn refresh_cookie_via_child(&self) -> Result<Option<String>> {
        let output = std::process::Command::new(std::env::current_exe()?)
            .arg("--host")
            .arg(&self.host)
            .arg("auth")
            .arg("refresh-cookie")
            .stderr(std::process::Stdio::null())
            .output()?;
        let cookies = cookie_from_stdout(output.stdout)?;
        if cookies.is_some() || output.status.success() {
            return Ok(cookies);
        }
        Ok(None)
    }

    pub fn refresh_or_login_cookie(&self) -> Result<String> {
        if let Some(cookies) = self.refresh_cookie_via_child()? {
            return Ok(cookies);
        }
        self.login_if_previously_authenticated(|| {
            format!(
                "QueryPie session expired and refresh failed; run `querypie --host {} auth login`",
                self.host
            )
        })
        .map(|session| session.cookies)
    }

    pub(crate) fn refresh_cookie_in_process(&self) -> Result<Option<String>> {
        let _lock = HostLock::acquire(&self.host)?;
        let host = self.host.clone();
        webview::refresh_cookies_if_needed(&self.host, move |cookies| {
            match Client::new(&host, cookies, new_window_id())?.connections() {
                Ok(_) => Ok(false),
                Err(err) if is_auth_expired(&err) => Ok(true),
                Err(err) => Err(err),
            }
        })
    }

    fn validate_cookie(&self, cookies: &str) -> Result<()> {
        Client::new(&self.host, cookies, new_window_id())?
            .connections()
            .map(|_| ())
    }

    fn check_cookies(&self, cookies: &str) -> Result<Option<AuthCheck>> {
        match self.validate_cookie(cookies) {
            Ok(()) => Ok(Some(AuthCheck::valid(self.host.clone()))),
            Err(err) if is_auth_expired(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn login_if_previously_authenticated<F>(&self, message: F) -> Result<AuthSession>
    where
        F: FnOnce() -> String,
    {
        if !self.has_login_history() {
            bail!("{}", message());
        }

        let _lock = HostLock::acquire(&self.host)?;
        if let Some(cookies) = self.read_cookie_via_child()? {
            if self.check_cookies(&cookies)?.is_some() {
                return Ok(self.session_from_cookies(cookies));
            }
        }

        self.login_with_lock_held()
    }

    fn has_login_history(&self) -> bool {
        webview::has_profile(&self.host) || sessioncache::has_host(&self.host)
    }

    fn session_from_cookies(&self, cookies: String) -> AuthSession {
        AuthSession {
            host: self.host.clone(),
            cookies,
        }
    }
}

pub fn is_auth_expired(err: &anyhow::Error) -> bool {
    err.downcast_ref::<GrpcError>()
        .map(|e| e.is_auth_expired())
        .unwrap_or(false)
}

fn new_window_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

fn cookie_from_stdout(stdout: Vec<u8>) -> Result<Option<String>> {
    let cookies = String::from_utf8(stdout)?.trim().to_string();
    Ok((!cookies.is_empty()).then_some(cookies))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_auth_expired_grpc_error() {
        // given
        let err = GrpcError {
            code: "16".to_string(),
            app_code: 0,
            message: "Access token expired".to_string(),
            domain: "ENGINE".to_string(),
        };
        let err = anyhow::Error::new(err);

        // when / then
        assert!(is_auth_expired(&err));
    }
}

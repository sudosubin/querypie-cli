mod error;
mod service;
mod status;
mod webview;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub use error::{is_login_canceled, AuthError};
pub use service::AuthService;
pub use status::{AuthCheck, AuthState};

use crate::sessioncache;

const AUTH_COOKIE: &str = "qp_access_token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub host: String,
    pub cookies: String,
}

pub(crate) fn refresh_cookies_in_process(host: &str) -> Result<Option<String>> {
    AuthService::new(host)?.refresh_cookie_in_process()
}

pub(crate) fn read_cookies_in_process(host: &str) -> Result<Option<String>> {
    AuthService::new(host)?.read_cookie_in_process()
}

pub(crate) fn known_hosts() -> Vec<String> {
    let mut hosts = BTreeSet::new();
    hosts.extend(webview::hosts());
    hosts.extend(sessioncache::hosts());
    hosts.into_iter().collect()
}

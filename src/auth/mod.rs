mod service;
mod status;
mod webview;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub use service::AuthService;
pub use status::{AuthCheck, AuthState};

use crate::paths;

const AUTH_COOKIE: &str = "qp_access_token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub host: String,
    pub cookies: String,
    pub timestamp: String,
}

pub fn refresh_cookies_in_process(host: &str) -> Result<Option<String>> {
    AuthService::new(host)?.refresh_cookie_in_process()
}

pub fn read_cookies_in_process(host: &str) -> Result<Option<String>> {
    AuthService::new(host)?.read_cookie_in_process()
}

pub fn known_hosts() -> Vec<String> {
    let mut hosts = BTreeSet::new();
    collect_webview_hosts(&mut hosts);
    collect_cache_hosts(&mut hosts);
    hosts.into_iter().collect()
}

pub(crate) fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn collect_webview_hosts(hosts: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(paths::webview_data_root()) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        if let Some(host) = entry.file_name().to_str().map(paths::normalize_host) {
            if !host.is_empty() {
                hosts.insert(host);
            }
        }
    }
}

fn collect_cache_hosts(hosts: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(paths::cache_dir()) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(host) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let host = paths::normalize_host(host);
        if !host.is_empty() && host != "sessions" {
            hosts.insert(host);
        }
    }
}

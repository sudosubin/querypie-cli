use anyhow::Result;
use std::collections::BTreeMap;
use tauri::webview::Cookie;
use tauri::WebviewWindow;

use super::AuthUrls;

pub(super) fn read_cookie_header<R: tauri::Runtime>(
    window: &WebviewWindow<R>,
    urls: &AuthUrls,
) -> Result<Option<String>> {
    let mut cookies = window.cookies_for_url(urls.app.clone())?;
    cookies.extend(window.cookies_for_url(urls.refresh.clone())?);
    let cookies = cookies
        .into_iter()
        .filter(|c| !c.value().is_empty())
        .map(|c| (c.name().to_string(), c.value().to_string()))
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ");
    Ok((!cookies.is_empty()).then_some(cookies))
}

pub(super) fn has_cookie(cookies: &str, name: &str) -> bool {
    cookies.split(';').any(|part| {
        part.trim()
            .split_once('=')
            .is_some_and(|(cookie_name, value)| {
                cookie_name.trim() == name && !value.trim().is_empty()
            })
    })
}

pub(super) fn parse_set_cookie(host: &str, set_cookie: &str) -> Option<Cookie<'static>> {
    let mut cookie = Cookie::parse(set_cookie.to_string()).ok()?;
    if cookie.domain().is_none() {
        cookie.set_domain(host.to_string());
    }
    if cookie.path().is_none() {
        cookie.set_path("/");
    }
    Some(cookie)
}

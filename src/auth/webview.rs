use anyhow::{anyhow, Context, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use url::Url;
use uuid::Uuid;

mod cookies;

use super::{now_unix, AuthSession, AUTH_COOKIE};
use crate::paths;
use crate::qpapi;
use crate::qpapi::Client;

use self::cookies::{has_cookie, parse_set_cookie, read_cookie_header};

#[derive(Clone)]
struct AuthUrls {
    app: Url,
    refresh: Url,
}

impl AuthUrls {
    fn new(host: &str) -> Result<Self> {
        Ok(Self {
            app: Url::parse(&format!("https://{host}/"))?,
            refresh: Url::parse(&format!(
                "https://{host}/engine-grpc/api.user.AccountService/RefreshToken"
            ))?,
        })
    }
}

pub fn login(host: &str) -> Result<AuthSession> {
    let host = paths::normalize_host(host);
    let urls = AuthUrls::new(&host)?;
    let result: Arc<Mutex<Option<AuthSession>>> = Arc::new(Mutex::new(None));
    let result_for_setup = Arc::clone(&result);
    let host_for_setup = host.clone();
    let urls_for_setup = urls.clone();

    let app = tauri::Builder::default()
        .setup(move |app| {
            hide_from_macos_dock(app);
            let data_dir = webview_data_dir(&host_for_setup);
            let result_for_load = Arc::clone(&result_for_setup);
            let result_for_poll = Arc::clone(&result_for_setup);
            let urls_for_load = urls_for_setup.clone();
            let urls_for_poll = urls_for_setup.clone();
            WebviewWindowBuilder::new(
                app,
                "querypie-login",
                WebviewUrl::External(urls_for_setup.app.clone()),
            )
            .title("QueryPie Login")
            .inner_size(420.0, 640.0)
            .center()
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .decorations(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(true)
            .data_directory(data_dir)
            .on_page_load(move |window, payload| {
                if payload.event() != PageLoadEvent::Finished {
                    return;
                }
                capture_session_if_ready(&window, &urls_for_load, &result_for_load);
            })
            .build()
            .inspect(|window| {
                let poll_window = window.clone();
                std::thread::spawn(move || loop {
                    if result_for_poll
                        .lock()
                        .map(|slot| slot.is_some())
                        .unwrap_or(true)
                    {
                        break;
                    }
                    capture_session_if_ready(&poll_window, &urls_for_poll, &result_for_poll);
                    std::thread::sleep(Duration::from_millis(300));
                });
            })?;
            Ok(())
        })
        .build(tauri_context())
        .context("build Tauri login webview")?;

    let _ = app.run_return(|_, _| {});
    result
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .ok_or_else(|| anyhow!("login canceled before a QueryPie session was established"))
}

pub fn read_cookies(host: &str) -> Result<Option<String>> {
    with_cookie_window(host, |window, urls| {
        let cookies = read_cookie_header(window, urls)?;
        Ok(cookies.filter(|cookies| has_cookie(cookies, AUTH_COOKIE)))
    })
}

pub fn refresh_cookies_if_needed<F>(host: &str, should_refresh: F) -> Result<Option<String>>
where
    F: FnOnce(&str) -> Result<bool> + Send + 'static,
{
    let host = paths::normalize_host(host);
    let host_for_refresh = host.clone();
    with_cookie_window(&host, move |window, urls| {
        let Some(cookies) = read_cookie_header(window, urls)? else {
            return Ok(None);
        };
        if !should_refresh(&cookies)? {
            return Ok(Some(cookies));
        }
        let Some(set_cookies) = qpapi::grpcweb::refresh_access_token(&host_for_refresh, &cookies)?
        else {
            return Ok(None);
        };
        for set_cookie in set_cookies {
            if let Some(cookie) = parse_set_cookie(&host_for_refresh, &set_cookie) {
                window.set_cookie(cookie)?;
            }
        }
        let Some(cookies) = read_cookie_header(window, urls)? else {
            return Ok(None);
        };
        Ok(has_cookie(&cookies, AUTH_COOKIE).then_some(cookies))
    })
}

pub fn clear_profile(host: &str) -> Result<()> {
    if host.trim().is_empty() {
        let root = webview_data_root();
        if root.exists() {
            std::fs::remove_dir_all(root)?;
        }
        return Ok(());
    }
    let host = paths::normalize_host(host);
    let dir = webview_data_dir(&host);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

fn with_cookie_window<T, F>(host: &str, f: F) -> Result<T>
where
    T: Clone + Send + 'static,
    F: FnOnce(&WebviewWindow, &AuthUrls) -> Result<T> + Send + 'static,
{
    let host = paths::normalize_host(host);
    let urls = AuthUrls::new(&host)?;
    let result: Arc<Mutex<Option<Result<T, String>>>> = Arc::new(Mutex::new(None));
    let result_for_setup = Arc::clone(&result);
    let urls_for_setup = urls.clone();
    let host_for_setup = host.clone();
    let f = Arc::new(Mutex::new(Some(f)));

    let app = tauri::Builder::default()
        .setup(move |app| {
            hide_from_macos_dock(app);
            let window = WebviewWindowBuilder::new(
                app,
                "querypie-cookie",
                WebviewUrl::External(urls_for_setup.app.clone()),
            )
            .title("QueryPie Cookie")
            .visible(false)
            .data_directory(webview_data_dir(&host_for_setup))
            .build()?;
            let urls = urls_for_setup.clone();
            let result = Arc::clone(&result_for_setup);
            let f = Arc::clone(&f);
            let worker_window = window.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                let outcome = f
                    .lock()
                    .ok()
                    .and_then(|mut slot| slot.take())
                    .ok_or_else(|| "cookie operation was already consumed".to_string())
                    .and_then(|f| f(&worker_window, &urls).map_err(|err| err.to_string()));
                if let Ok(mut slot) = result.lock() {
                    *slot = Some(outcome);
                }
                let close_window = worker_window.clone();
                let app_handle = worker_window.app_handle().clone();
                let _ = app_handle.run_on_main_thread(move || {
                    let _ = close_window.close();
                });
            });
            Ok(())
        })
        .build(tauri_context())
        .context("build Tauri cookie webview")?;

    let _ = app.run_return(|_, _| {});
    let outcome = result
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .ok_or_else(|| anyhow!("cookie operation did not complete"))?;
    outcome.map_err(anyhow::Error::msg)
}

fn capture_session_if_ready<R: tauri::Runtime>(
    window: &WebviewWindow<R>,
    urls: &AuthUrls,
    result: &Arc<Mutex<Option<AuthSession>>>,
) {
    if result.lock().map(|slot| slot.is_some()).unwrap_or(true) {
        return;
    }
    let Ok(Some(session)) = read_session_from_window(window, urls) else {
        return;
    };
    if let Ok(mut slot) = result.lock() {
        if slot.is_none() {
            *slot = Some(session);
            window.app_handle().exit(0);
        }
    }
}

fn read_session_from_window<R: tauri::Runtime>(
    window: &WebviewWindow<R>,
    urls: &AuthUrls,
) -> Result<Option<AuthSession>> {
    let Some(cookies) = read_cookie_header(window, urls)? else {
        return Ok(None);
    };
    if !has_cookie(&cookies, AUTH_COOKIE) {
        return Ok(None);
    };
    let host = urls.app.host_str().unwrap_or_default().to_string();
    if !is_authenticated(&host, &cookies) {
        return Ok(None);
    }
    Ok(Some(AuthSession {
        host,
        cookies,
        timestamp: now_unix().to_string(),
    }))
}

fn is_authenticated(host: &str, cookies: &str) -> bool {
    let window_id = Uuid::new_v4().simple().to_string();
    Client::new(host, cookies, window_id)
        .and_then(|client| client.connections().map(|_| ()))
        .is_ok()
}

fn webview_data_dir(host: &str) -> std::path::PathBuf {
    webview_data_root().join(host.replace(['/', ':'], "_"))
}

fn webview_data_root() -> std::path::PathBuf {
    paths::webview_data_root()
}

fn tauri_context() -> tauri::Context<tauri::Wry> {
    tauri::tauri_build_context!()
}

#[cfg(target_os = "macos")]
fn hide_from_macos_dock<R: tauri::Runtime>(app: &mut tauri::App<R>) {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    app.set_dock_visibility(false);
}

#[cfg(not(target_os = "macos"))]
fn hide_from_macos_dock<R: tauri::Runtime>(_app: &mut tauri::App<R>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_host_input() {
        // given
        let cases = [
            ("https://querypie.example.com/", "querypie.example.com"),
            ("querypie.example.com", "querypie.example.com"),
        ];

        for (input, expected) in cases {
            // when
            let normalized = paths::normalize_host(input);

            // then
            assert_eq!(normalized, expected);
        }
    }
}

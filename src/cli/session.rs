use anyhow::{anyhow, bail, Result};
use rand::RngCore;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Global;
use crate::auth::AuthService;
use crate::config;
use crate::qpapi::{Client, GrpcError};
use crate::sessioncache;

pub(super) struct Resolved {
    pub(super) client: Client,
    pub(super) session: String,
    pub(super) db: String,
    pub(super) db_type: i32,
}

impl Global {
    pub(super) fn require_host(&self) -> Result<&str> {
        if self.host.trim().is_empty() {
            Err(anyhow!(
                "no QueryPie host: pass --host or set `host:` in {}",
                config::default_config_file().display()
            ))
        } else {
            Ok(&self.host)
        }
    }

    pub(super) fn new_client(&self) -> Result<Client> {
        Client::new(self.require_host()?, self.cookie(false)?, new_window_id())
    }

    pub(super) fn new_client_with_cookie(&self, cookie: String) -> Result<Client> {
        Client::new(self.require_host()?, cookie, new_window_id())
    }

    pub(super) fn with_session<F>(&self, f: F) -> Result<()>
    where
        F: Fn(&Resolved) -> Result<()>,
    {
        self.require_connection()?;
        let mut resolved = match self.resolve_session(false, false) {
            Err(err) if is_auth_expired(&err) => self.resolve_after_refresh()?,
            other => other?,
        };
        let run = |resolved: &Resolved| {
            self.validate_database(resolved)?;
            f(resolved)
        };

        match run(&resolved) {
            Err(err) if is_auth_expired(&err) => {
                resolved = self.resolve_after_refresh()?;
                run(&resolved)
            }
            Err(err) if is_session_not_found(&err) => {
                if self.verbose {
                    eprintln!("session expired; re-opening");
                }
                resolved = self.resolve_session(true, false)?;
                run(&resolved)
            }
            other => other,
        }
    }

    pub(super) fn refresh_cookie_or_error(&self) -> Result<String> {
        if self.verbose {
            eprintln!("auth expired; refreshing token");
        }
        let _ = sessioncache::clear(&self.host, &self.connection);
        AuthService::new(self.require_host()?)?
            .refresh_cookie_via_child()?
            .ok_or_else(|| {
                anyhow!(
                    "QueryPie session expired and refresh failed; run `querypie --host {} auth login`",
                    self.host
                )
            })
    }

    fn require_connection(&self) -> Result<()> {
        if self.connection.trim().is_empty() {
            Err(anyhow!(
                "no connection: pass --connection or set `connection:` in config"
            ))
        } else {
            Ok(())
        }
    }

    fn cookie(&self, force_login: bool) -> Result<String> {
        let auth = AuthService::new(self.require_host()?)?;
        if force_login {
            Ok(auth.login()?.cookies)
        } else {
            auth.read_cookie_or_error()
        }
    }

    fn resolve_after_refresh(&self) -> Result<Resolved> {
        let cookie = self.refresh_cookie_or_error()?;
        self.resolve_session_with_cookie(true, cookie)
    }

    fn resolve_session(&self, force_reopen: bool, force_login: bool) -> Result<Resolved> {
        let cookie = self.cookie(force_login)?;
        self.resolve_session_with_cookie(force_reopen, cookie)
    }

    fn resolve_session_with_cookie(&self, force_reopen: bool, cookie: String) -> Result<Resolved> {
        let host = self.require_host()?.to_string();
        if force_reopen {
            return self.open_session(host, cookie);
        }
        if let Some(entry) = self.cached_session(&host, &cookie)? {
            return Ok(entry);
        }
        self.open_session(host, cookie)
    }

    fn cached_session(&self, host: &str, cookie: &str) -> Result<Option<Resolved>> {
        let Some(entry) = sessioncache::get(host, &self.connection, &self.engine) else {
            return Ok(None);
        };
        if self.verbose {
            eprintln!(
                "reusing cached session {} (window {})",
                entry.session, entry.window_id
            );
        }
        Ok(Some(Resolved {
            client: Client::new(host, cookie.to_string(), entry.window_id)?,
            session: entry.session,
            db: self.selected_db(entry.db),
            db_type: db_type_for_engine(&self.engine),
        }))
    }

    fn open_session(&self, host: String, cookie: String) -> Result<Resolved> {
        let window_id = new_window_id();
        let client = Client::new(host.clone(), cookie, window_id.clone())?;
        let session = client.open_session(&self.connection, &self.engine)?;
        if self.verbose {
            eprintln!(
                "opened session {} ({}/{}, db={}) window {}",
                session.session, session.engine, session.version, session.db, window_id
            );
        }
        sessioncache::put(sessioncache::Entry {
            host,
            connection: self.connection.clone(),
            engine: self.engine.clone(),
            window_id,
            session: session.session.clone(),
            db: session.db.clone(),
            opened_at: now_unix(),
        })?;
        Ok(Resolved {
            client,
            session: session.session,
            db: self.selected_db(session.db),
            db_type: session.db_type,
        })
    }

    fn validate_database(&self, resolved: &Resolved) -> Result<()> {
        if resolved.db.trim().is_empty() {
            return Ok(());
        }
        let names = resolved
            .client
            .get_databases(&resolved.session, &resolved.db)?;
        if names.iter().any(|name| name == &resolved.db) {
            return Ok(());
        }
        bail!(
            "database `{}` is not accessible for connection `{}`",
            resolved.db,
            self.connection
        );
    }

    fn selected_db(&self, fallback: String) -> String {
        if self.database.is_empty() {
            fallback
        } else {
            self.database.clone()
        }
    }
}

pub(super) fn is_auth_expired(err: &anyhow::Error) -> bool {
    err.downcast_ref::<GrpcError>()
        .map(|e| e.is_auth_expired())
        .unwrap_or(false)
}

fn is_session_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<GrpcError>()
        .map(|e| e.is_session_not_found())
        .unwrap_or(false)
}

fn new_window_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn db_type_for_engine(engine: &str) -> i32 {
    match engine.trim().to_ascii_lowercase().as_str() {
        "mysql" => 1,
        "postgresql" | "postgres" => 3,
        "mongodb" | "mongo" => 13,
        _ => 0,
    }
}

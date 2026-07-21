use anyhow::{anyhow, bail, Result};
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
        let cookie = AuthService::new(self.require_host()?)?.refresh_or_login_cookie()?;
        let _ = sessioncache::clear(&self.host, &self.connection);
        Ok(cookie)
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
            auth.login().map(|session| session.cookies)
        } else {
            auth.read_or_login_cookie()
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
        let Some(entry) =
            sessioncache::get_matching(host, &self.connection, &self.engine, &self.database)
        else {
            return Ok(None);
        };
        if self.verbose {
            eprintln!(
                "reusing cached session {} (window {}, db={})",
                entry.session, entry.window_id, entry.resolved_db
            );
        }
        let db_type = cached_db_type(&entry);
        Ok(Some(Resolved {
            client: Client::new(host, cookie.to_string(), entry.window_id)?,
            session: entry.session,
            db: entry.resolved_db,
            db_type,
        }))
    }

    fn open_session(&self, host: String, cookie: String) -> Result<Resolved> {
        let window_id = new_window_id();
        let client = Client::new(host.clone(), cookie, window_id.clone())?;
        let opened = client.open_session(&self.connection, &self.engine)?;

        let input_db = self.database.trim();
        let resolved_db = if input_db.is_empty() {
            opened.db.clone()
        } else {
            input_db.to_string()
        };
        if !input_db.is_empty() && input_db != opened.db {
            client.change_database(&opened.instance_uuid, input_db)?;
        }

        if self.verbose {
            eprintln!(
                "opened session {} ({}/{}, db={}) window {}",
                opened.session, opened.engine, opened.version, resolved_db, window_id
            );
        }
        sessioncache::put(sessioncache::Entry {
            host,
            connection: opened.connection.clone(),
            engine: opened.engine_name.clone(),
            input_db: self.database.clone(),
            window_id,
            session: opened.session.clone(),
            resolved_db: resolved_db.clone(),
            db_type: opened.db_type,
            opened_at: now_unix(),
        })?;
        Ok(Resolved {
            client,
            session: opened.session,
            db: resolved_db,
            db_type: opened.db_type,
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
    uuid::Uuid::new_v4().simple().to_string()
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

fn cached_db_type(entry: &sessioncache::Entry) -> i32 {
    match entry.db_type {
        0 => db_type_for_engine(&entry.engine),
        db_type => db_type,
    }
}

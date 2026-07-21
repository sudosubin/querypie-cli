use anyhow::{bail, Context, Result};
use clap_complete::CompletionCandidate;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::auth::{self, AuthService};
use crate::config;
use crate::qpapi::{Client, GrpcError};
use crate::sessioncache;

const COMPLETE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy)]
enum CompletionKind {
    Host,
    Connection,
    Engine,
    Database,
    Schema,
    Table,
}

#[derive(Debug, Default)]
struct CompletionContext {
    host: String,
    connection: String,
    engine: String,
    database: String,
    schema: String,
}

struct CompletionSession {
    client: Client,
    session: String,
    db: String,
}

impl CompletionSession {
    fn from_cache(
        ctx: &CompletionContext,
        cookie: &str,
        entry: sessioncache::Entry,
    ) -> Result<Self> {
        Ok(Self {
            client: client_with_window(ctx, cookie, entry.window_id)?,
            session: entry.session,
            db: entry.resolved_db,
        })
    }

    fn open(ctx: &CompletionContext, cookie: &str) -> Result<Self> {
        let window_id = new_window_id();
        let client = client_with_window(ctx, cookie, window_id.clone())?;
        let opened = client.open_session(&ctx.connection, &ctx.engine)?;

        let input_db = ctx.database.trim();
        let resolved_db = if input_db.is_empty() {
            opened.db.clone()
        } else {
            input_db.to_string()
        };
        if !input_db.is_empty() && input_db != opened.db {
            client.change_database(&opened.instance_uuid, input_db)?;
        }

        sessioncache::put(sessioncache::Entry {
            host: ctx.host.clone(),
            connection: opened.connection.clone(),
            engine: opened.engine_name.clone(),
            input_db: ctx.database.clone(),
            window_id,
            session: opened.session.clone(),
            resolved_db: resolved_db.clone(),
            db_type: opened.db_type,
            opened_at: now_unix(),
        })?;
        Ok(Self {
            client,
            session: opened.session,
            db: resolved_db,
        })
    }
}

pub(super) fn complete_hosts(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(CompletionKind::Host, current)
}

pub(super) fn complete_connections(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(CompletionKind::Connection, current)
}

pub(super) fn complete_engines(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(CompletionKind::Engine, current)
}

pub(super) fn complete_databases(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(CompletionKind::Database, current)
}

pub(super) fn complete_schemas(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(CompletionKind::Schema, current)
}

pub(super) fn complete_tables(current: &OsStr) -> Vec<CompletionCandidate> {
    complete(CompletionKind::Table, current)
}

fn complete(kind: CompletionKind, current: &OsStr) -> Vec<CompletionCandidate> {
    complete_values(kind)
        .unwrap_or_default()
        .into_iter()
        .filter(|value| starts_with(value, current))
        .map(CompletionCandidate::new)
        .collect()
}

fn complete_values(kind: CompletionKind) -> Result<Vec<String>> {
    let ctx = CompletionContext::load();
    match kind {
        CompletionKind::Host => Ok(hosts(&ctx)),
        CompletionKind::Connection => connections(&ctx),
        CompletionKind::Engine => engines(&ctx),
        CompletionKind::Database => databases(&ctx),
        CompletionKind::Schema => schemas(&ctx),
        CompletionKind::Table => tables(&ctx),
    }
}

fn hosts(ctx: &CompletionContext) -> Vec<String> {
    let mut values = auth::known_hosts();
    if !ctx.host.is_empty() {
        values.push(ctx.host.clone());
    }
    sorted_unique(values)
}

fn connections(ctx: &CompletionContext) -> Result<Vec<String>> {
    with_client(ctx, |client| {
        Ok(sorted_unique(
            client.connections()?.into_iter().map(|conn| conn.name),
        ))
    })
}

fn engines(ctx: &CompletionContext) -> Result<Vec<String>> {
    with_client(ctx, |client| {
        Ok(sorted_unique(
            client
                .connections()?
                .into_iter()
                .filter(|conn| connection_matches(&conn.name, &ctx.connection))
                .map(|conn| conn.engine().to_string()),
        ))
    })
}

fn databases(ctx: &CompletionContext) -> Result<Vec<String>> {
    with_session(ctx, |resolved| {
        resolved
            .client
            .get_databases(&resolved.session, &resolved.db)
    })
}

fn schemas(ctx: &CompletionContext) -> Result<Vec<String>> {
    with_session(ctx, |resolved| {
        resolved.client.get_schemas(&resolved.session, &resolved.db)
    })
}

fn tables(ctx: &CompletionContext) -> Result<Vec<String>> {
    with_session(ctx, |resolved| {
        resolved
            .client
            .get_tables(&resolved.session, &resolved.db, &ctx.schema)
    })
}

fn with_session<F>(ctx: &CompletionContext, f: F) -> Result<Vec<String>>
where
    F: Fn(&CompletionSession) -> Result<Vec<String>>,
{
    if ctx.connection.is_empty() {
        return Ok(Vec::new());
    }
    with_cookie(ctx, |cookie| {
        let resolved = resolve_session(ctx, cookie)?;
        match f(&resolved) {
            Err(err) if is_session_not_found(&err) => f(&CompletionSession::open(ctx, cookie)?),
            other => other,
        }
    })
}

fn with_client<T, F>(ctx: &CompletionContext, f: F) -> Result<T>
where
    F: Fn(&Client) -> Result<T>,
{
    with_cookie(ctx, |cookie| {
        let client = client_with_window(ctx, cookie, new_window_id())?;
        f(&client)
    })
}

fn with_cookie<T, F>(ctx: &CompletionContext, f: F) -> Result<T>
where
    F: Fn(&str) -> Result<T>,
{
    if ctx.host.is_empty() {
        bail!("host is required");
    }
    let auth = AuthService::new(&ctx.host)?;
    let cookie = auth.read_cookie_via_child()?.context("not logged in")?;
    match f(&cookie) {
        Err(err) if is_auth_expired(&err) => {
            let cookie = auth.refresh_cookie_via_child()?.context("refresh failed")?;
            f(&cookie)
        }
        other => other,
    }
}

fn resolve_session(ctx: &CompletionContext, cookie: &str) -> Result<CompletionSession> {
    if let Some(entry) = cached_session(ctx) {
        return CompletionSession::from_cache(ctx, cookie, entry);
    }
    CompletionSession::open(ctx, cookie)
}

fn cached_session(ctx: &CompletionContext) -> Option<sessioncache::Entry> {
    sessioncache::get_matching(&ctx.host, &ctx.connection, &ctx.engine, &ctx.database)
}

fn client_with_window(ctx: &CompletionContext, cookie: &str, window_id: String) -> Result<Client> {
    Client::new_with_timeout(&ctx.host, cookie, window_id, COMPLETE_TIMEOUT)
}

fn is_auth_expired(err: &anyhow::Error) -> bool {
    err.downcast_ref::<GrpcError>()
        .is_some_and(GrpcError::is_auth_expired)
}

fn is_session_not_found(err: &anyhow::Error) -> bool {
    err.downcast_ref::<GrpcError>()
        .is_some_and(GrpcError::is_session_not_found)
}

impl CompletionContext {
    fn load() -> Self {
        let words = completion_words();
        let config_path = option_value(&words, "--config", None);
        let cfg = config::load(config_path.as_deref()).unwrap_or_default();
        Self::from_words(&words, cfg)
    }

    fn from_words(words: &[String], cfg: config::Config) -> Self {
        Self {
            host: pick(option_value(words, "--host", None), cfg.host),
            connection: pick(
                option_value(words, "--connection", Some("-c")),
                cfg.connection,
            ),
            engine: option_value(words, "--engine", None).unwrap_or_default(),
            database: pick(option_value(words, "--db", Some("-d")), cfg.database),
            schema: option_value(words, "--schema", None).unwrap_or_default(),
        }
    }
}

fn completion_words() -> Vec<String> {
    let mut args = std::env::args_os()
        .skip_while(|arg| arg != "--")
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if args.first().is_some_and(|arg| is_querypie_binary_word(arg)) {
        args.remove(0);
    }
    args
}

fn is_querypie_binary_word(word: &str) -> bool {
    Path::new(word)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem == "querypie")
}

fn option_value(words: &[String], long: &str, short: Option<&str>) -> Option<String> {
    let long_prefix = format!("{long}=");
    let mut iter = words.iter();
    while let Some(word) = iter.next() {
        if let Some(value) = word.strip_prefix(&long_prefix) {
            return Some(unquote_completion_value(value));
        }
        if word == long || short.is_some_and(|short| word == short) {
            return iter.next().map(|value| unquote_completion_value(value));
        }
    }
    None
}

fn unquote_completion_value(value: &str) -> String {
    let Some(quote) = value.chars().next().filter(|ch| *ch == '\'' || *ch == '"') else {
        return value.to_string();
    };
    let value = value.strip_prefix(quote).unwrap_or(value);
    let value = value.strip_suffix(quote).unwrap_or(value);
    if quote == '"' {
        unescape_double_quoted(value)
    } else {
        value.to_string()
    }
}

fn unescape_double_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some(next @ ('"' | '\\' | '$' | '`')) => out.push(next),
            Some(next) => {
                out.push('\\');
                out.push(next);
            }
            None => out.push('\\'),
        }
    }
    out
}

fn pick(flag: Option<String>, cfg: String) -> String {
    flag.filter(|s| !s.trim().is_empty()).unwrap_or(cfg)
}

fn sorted_unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn starts_with(value: &str, current: &OsStr) -> bool {
    let Some(current) = current.to_str() else {
        return false;
    };
    value.starts_with(&unquote_completion_value(current))
}

fn connection_matches(name: &str, query: &str) -> bool {
    query.trim().is_empty()
        || name
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_long_option_values() {
        let words = vec![
            "--host".to_string(),
            "querypie.example.com".to_string(),
            "--connection=main".to_string(),
        ];

        assert_eq!(
            option_value(&words, "--host", None).as_deref(),
            Some("querypie.example.com")
        );
        assert_eq!(
            option_value(&words, "--connection", Some("-c")).as_deref(),
            Some("main")
        );
    }

    #[test]
    fn reads_short_option_values() {
        let words = vec!["-c".to_string(), "main".to_string()];

        assert_eq!(
            option_value(&words, "--connection", Some("-c")).as_deref(),
            Some("main")
        );
    }

    #[test]
    fn reads_quoted_option_values() {
        let words = vec![
            "--connection".to_string(),
            "\"example-main\"".to_string(),
            "--db='example_app'".to_string(),
            "--schema=\"public\"".to_string(),
        ];

        assert_eq!(
            option_value(&words, "--connection", Some("-c")).as_deref(),
            Some("example-main")
        );
        assert_eq!(
            option_value(&words, "--db", Some("-d")).as_deref(),
            Some("example_app")
        );
        assert_eq!(
            option_value(&words, "--schema", None).as_deref(),
            Some("public")
        );
        assert_eq!(
            unquote_completion_value(r#""example \"main\"""#),
            "example \"main\""
        );
    }

    #[test]
    fn matches_quoted_current_prefixes() {
        assert!(starts_with(
            "example-main",
            std::ffi::OsStr::new("\"example")
        ));
        assert!(starts_with(
            "example-main",
            std::ffi::OsStr::new("'example")
        ));
    }
}

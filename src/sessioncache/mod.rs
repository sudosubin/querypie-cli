use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::lockfile::HostLock;
use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub host: String,
    pub connection: String,
    pub engine: String,
    pub window_id: String,
    pub session: String,
    pub db: String,
    pub opened_at: i64,
}

#[derive(Default, Serialize, Deserialize)]
struct HostCacheFile {
    #[serde(default)]
    sessions: BTreeMap<String, BTreeMap<String, SessionEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionEntry {
    window_id: String,
    session: String,
    db: String,
    opened_at: i64,
}

fn path_for_host(host: &str) -> PathBuf {
    paths::host_cache_file(&paths::normalize_host(host))
}

pub fn get(host: &str, conn: &str, engine: &str) -> Option<Entry> {
    let host = paths::normalize_host(host);
    let file = load_host(&host);
    let entry = file.sessions.get(conn)?.get(engine)?;
    Some(Entry {
        host,
        connection: conn.to_string(),
        engine: engine.to_string(),
        window_id: entry.window_id.clone(),
        session: entry.session.clone(),
        db: entry.db.clone(),
        opened_at: entry.opened_at,
    })
}

pub fn put(entry: Entry) -> Result<()> {
    let host = paths::normalize_host(&entry.host);
    let _lock = HostLock::acquire(&host)?;
    let connection = entry.connection.clone();
    let engine = entry.engine.clone();
    let mut file = load_host(&host);
    file.sessions
        .entry(connection)
        .or_default()
        .insert(engine, entry.into());
    save_host(&host, &file)
}

pub fn list() -> Vec<Entry> {
    let mut entries = Vec::new();
    let Ok(files) = std::fs::read_dir(cache_root()) else {
        return entries;
    };
    for file in files.flatten() {
        let path = file.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(host) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        entries.extend(list_host(host));
    }
    entries
}

pub fn clear(host: &str, conn: &str) -> Result<()> {
    let host = paths::normalize_host(host);
    if host.is_empty() {
        clear_all_hosts()
    } else {
        clear_host(&host, conn)
    }
}

fn list_host(host: &str) -> Vec<Entry> {
    let file = load_host(host);
    file.sessions
        .into_iter()
        .flat_map(|(connection, engines)| {
            engines.into_iter().map(move |(engine, entry)| Entry {
                host: host.to_string(),
                connection: connection.clone(),
                engine,
                window_id: entry.window_id,
                session: entry.session,
                db: entry.db,
                opened_at: entry.opened_at,
            })
        })
        .collect()
}

fn clear_all_hosts() -> Result<()> {
    let Ok(files) = std::fs::read_dir(cache_root()) else {
        return Ok(());
    };
    for file in files.flatten() {
        let path = file.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

fn clear_host(host: &str, conn: &str) -> Result<()> {
    let _lock = HostLock::acquire(host)?;
    let mut file = load_host(host);
    if conn.trim().is_empty() {
        file.sessions.clear();
    } else {
        file.sessions.remove(conn);
    }
    save_host(host, &file)
}

fn load_host(host: &str) -> HostCacheFile {
    let Ok(text) = std::fs::read_to_string(path_for_host(host)) else {
        return HostCacheFile::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_host(host: &str, file: &HostCacheFile) -> Result<()> {
    let path = path_for_host(host);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(file)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn cache_root() -> PathBuf {
    paths::cache_dir()
}

impl From<Entry> for SessionEntry {
    fn from(entry: Entry) -> Self {
        Self {
            window_id: entry.window_id,
            session: entry.session,
            db: entry.db,
            opened_at: entry.opened_at,
        }
    }
}

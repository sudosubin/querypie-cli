use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
    pub db_type: i32,
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
    #[serde(default)]
    db_type: i32,
    opened_at: i64,
}

fn path_for_host(host: &str) -> PathBuf {
    paths::host_cache_file(&paths::normalize_host(host))
}

pub fn get_matching(host: &str, conn_query: &str, engine: &str) -> Option<Entry> {
    let host = paths::normalize_host(host);
    let file = load_host(&host);
    find_matching(&file, &host, conn_query, engine)
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
    for host in hosts() {
        entries.extend(list_host(&host));
    }
    entries
}

pub fn hosts() -> Vec<String> {
    let Ok(files) = std::fs::read_dir(cache_root()) else {
        return Vec::new();
    };
    files
        .flatten()
        .filter_map(|file| host_from_cache_path(&file.path()))
        .filter(|host| has_host(host))
        .collect()
}

pub fn has_host(host: &str) -> bool {
    let host = paths::normalize_host(host);
    !host.is_empty() && !load_host(&host).sessions.is_empty()
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
        .iter()
        .flat_map(|(connection, engines)| {
            engines
                .iter()
                .map(move |(engine, entry)| entry.to_public(host, connection, engine))
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
    if conn.trim().is_empty() {
        let path = path_for_host(host);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(());
    }

    let mut file = load_host(host);
    remove_matching(&mut file, conn)?;
    if file.sessions.is_empty() {
        let path = path_for_host(host);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    } else {
        save_host(host, &file)
    }
}

fn find_matching(
    file: &HostCacheFile,
    host: &str,
    conn_query: &str,
    engine_query: &str,
) -> Option<Entry> {
    let engine_query = engine_query.trim().to_ascii_lowercase();
    let needle = conn_query.to_ascii_lowercase();
    let mut exact = Vec::new();
    let mut partial = Vec::new();

    for (connection, engines) in &file.sessions {
        let partial_match = connection.to_ascii_lowercase().contains(&needle);
        for (engine, entry) in engines {
            if !matches_engine(engine, &engine_query) {
                continue;
            }

            if connection == conn_query {
                exact.push(entry.to_public(host, connection, engine));
            } else if partial_match {
                partial.push(entry.to_public(host, connection, engine));
            }
        }
    }

    if !exact.is_empty() {
        return single(exact);
    }
    single(partial)
}

fn matches_engine(engine: &str, query: &str) -> bool {
    query.is_empty() || engine.eq_ignore_ascii_case(query)
}

fn single(mut entries: Vec<Entry>) -> Option<Entry> {
    (entries.len() == 1).then(|| entries.remove(0))
}

fn remove_matching(file: &mut HostCacheFile, conn_query: &str) -> Result<()> {
    let matches = matching_connections(file, conn_query);
    match matches.as_slice() {
        [] => Ok(()),
        [connection] => {
            file.sessions.remove(connection);
            Ok(())
        }
        _ => {
            let choices = matches.join("\n  ");
            bail!("cached connection {conn_query:?} is ambiguous:\n  {choices}")
        }
    }
}

fn matching_connections(file: &HostCacheFile, conn_query: &str) -> Vec<String> {
    if file.sessions.contains_key(conn_query) {
        return vec![conn_query.to_string()];
    }

    let needle = conn_query.to_ascii_lowercase();
    file.sessions
        .keys()
        .filter(|connection| connection.to_ascii_lowercase().contains(&needle))
        .cloned()
        .collect()
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

fn host_from_cache_path(path: &Path) -> Option<String> {
    if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
        return None;
    }
    let host = paths::normalize_host(path.file_stem()?.to_str()?);
    (!host.is_empty() && host != "sessions").then_some(host)
}

impl SessionEntry {
    fn to_public(&self, host: &str, connection: &str, engine: &str) -> Entry {
        Entry {
            host: host.to_string(),
            connection: connection.to_string(),
            engine: engine.to_string(),
            window_id: self.window_id.clone(),
            session: self.session.clone(),
            db: self.db.clone(),
            db_type: self.db_type,
            opened_at: self.opened_at,
        }
    }
}

impl From<Entry> for SessionEntry {
    fn from(entry: Entry) -> Self {
        Self {
            window_id: entry.window_id,
            session: entry.session,
            db: entry.db,
            db_type: entry.db_type,
            opened_at: entry.opened_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_lookup_returns_canonical_cache_entry() {
        let file = cache_file(vec![
            ("production-main", "mysql", "s1", 1),
            ("production-main readonly", "mysql", "s2", 1),
        ]);

        let entry = find_matching(&file, "example.querypie", "production-main", "")
            .expect("expected exact cache match");

        assert_eq!(entry.connection, "production-main");
        assert_eq!(entry.engine, "mysql");
        assert_eq!(entry.session, "s1");
        assert_eq!(entry.db_type, 1);
    }

    #[test]
    fn substring_lookup_returns_single_canonical_match() {
        let file = cache_file(vec![("production-main", "mysql", "s1", 1)]);

        let entry =
            find_matching(&file, "example.querypie", "prod", "").expect("expected cache match");

        assert_eq!(entry.connection, "production-main");
    }

    #[test]
    fn substring_lookup_returns_none_for_multiple_matches() {
        let file = cache_file(vec![
            ("production-main", "mysql", "s1", 1),
            ("production-replica", "mysql", "s2", 1),
        ]);

        let entry = find_matching(&file, "example.querypie", "prod", "");

        assert!(entry.is_none());
    }

    #[test]
    fn engine_filter_narrows_substring_match() {
        let file = cache_file(vec![
            ("production-main", "mysql", "s1", 1),
            ("production-warehouse", "postgresql", "s2", 3),
        ]);

        let entry = find_matching(&file, "example.querypie", "prod", "postgresql")
            .expect("expected engine-filtered cache match");

        assert_eq!(entry.connection, "production-warehouse");
        assert_eq!(entry.engine, "postgresql");
        assert_eq!(entry.db_type, 3);
    }

    #[test]
    fn old_cache_without_db_type_deserializes_with_default() -> Result<()> {
        let file: HostCacheFile = serde_json::from_str(
            r#"{
                "sessions": {
                    "production-main": {
                        "mysql": {
                            "window_id": "w1",
                            "session": "s1",
                            "db": "app",
                            "opened_at": 123
                        }
                    }
                }
            }"#,
        )?;

        let entry = find_matching(&file, "example.querypie", "prod", "mysql")
            .expect("expected old cache match");

        assert_eq!(entry.db_type, 0);
        Ok(())
    }

    #[test]
    fn clear_removes_single_substring_match() -> Result<()> {
        let mut file = cache_file(vec![
            ("production-main", "mysql", "s1", 1),
            ("staging-main", "mysql", "s2", 1),
        ]);

        remove_matching(&mut file, "prod")?;

        assert!(!file.sessions.contains_key("production-main"));
        assert!(file.sessions.contains_key("staging-main"));
        Ok(())
    }

    #[test]
    fn clear_removes_exact_match_before_substring_matches() -> Result<()> {
        let mut file = cache_file(vec![
            ("prod", "mysql", "s1", 1),
            ("production-main", "mysql", "s2", 1),
        ]);

        remove_matching(&mut file, "prod")?;

        assert!(!file.sessions.contains_key("prod"));
        assert!(file.sessions.contains_key("production-main"));
        Ok(())
    }

    #[test]
    fn clear_rejects_ambiguous_substring_match() {
        let mut file = cache_file(vec![
            ("production-main", "mysql", "s1", 1),
            ("production-replica", "mysql", "s2", 1),
        ]);

        let err = remove_matching(&mut file, "prod").expect_err("expected ambiguity error");

        assert!(err.to_string().contains("ambiguous"));
        assert_eq!(file.sessions.len(), 2);
    }

    fn cache_file(entries: Vec<(&str, &str, &str, i32)>) -> HostCacheFile {
        let mut file = HostCacheFile::default();
        for (connection, engine, session, db_type) in entries {
            file.sessions
                .entry(connection.to_string())
                .or_default()
                .insert(
                    engine.to_string(),
                    SessionEntry {
                        window_id: format!("{session}-window"),
                        session: session.to_string(),
                        db: "app".to_string(),
                        db_type,
                        opened_at: 123,
                    },
                );
        }
        file
    }
}

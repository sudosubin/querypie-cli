use std::path::PathBuf;

pub(crate) fn config_file() -> PathBuf {
    config_dir().join("config.yml")
}

pub(crate) fn host_lock_file(host: &str) -> PathBuf {
    lock_dir().join(format!("{}.lock", host_file_stem(host)))
}

pub(crate) fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

fn config_dir() -> PathBuf {
    env_dir("XDG_CONFIG_HOME")
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("querypie")
}

fn lock_dir() -> PathBuf {
    env_dir("XDG_CACHE_HOME")
        .unwrap_or_else(|| home_dir().join(".cache"))
        .join("querypie")
}

fn env_dir(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn host_file_stem(host: &str) -> String {
    normalize_host(host).replace(['/', ':'], "_")
}

use std::path::PathBuf;

pub(crate) fn config_file() -> PathBuf {
    config_dir().join("querypie").join("config.yml")
}

pub(crate) fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("querypie")
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

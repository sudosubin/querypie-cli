use anyhow::Result;

use super::commands::{fmt, OutputArgs};
use super::Global;
use crate::formatting::{self, style, OutputFormat};
use crate::sessioncache;

pub(super) fn list_cached_sessions(output: OutputArgs) -> Result<()> {
    let entries = sessioncache::list();
    if entries.is_empty() && output.output != OutputFormat::Json {
        eprintln!("no cached sessions");
        return Ok(());
    }
    formatting::sessions(&entries, fmt(output))
}

pub(super) fn clear_cached_sessions(global: &Global) -> Result<()> {
    sessioncache::clear(&global.host, &global.connection)?;
    if global.host.trim().is_empty() {
        anstream::println!("{} Cleared all cached sessions", style::success_icon());
    } else if global.connection.trim().is_empty() {
        anstream::println!(
            "{} Cleared cached sessions for {}",
            style::success_icon(),
            global.host
        );
    } else {
        anstream::println!(
            "{} Cleared cached sessions for {} connection {}",
            style::success_icon(),
            global.host,
            global.connection
        );
    }
    Ok(())
}

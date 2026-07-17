mod auth_cmd;
mod commands;
mod data_cmd;
mod session;
mod session_cmd;

use anyhow::Result;
use clap::Parser;
use std::fmt;

use self::commands::Command;
use crate::auth;
use crate::config;
use crate::formatting::style;
use crate::qpapi::GrpcError;

#[derive(Debug, Parser)]
#[command(name = "querypie")]
#[command(about = "Query QueryPie databases from the terminal")]
#[command(
    long_about = "QueryPie CLI authenticates with a lightweight webview session and runs catalog and SQL commands through QueryPie.",
    after_help = "EXAMPLES:
  querypie --host HOST auth login
  querypie connection list
  querypie query -c CONNECTION 'select 1;'"
)]
struct Cli {
    #[arg(long, global = true, value_name = "HOST", help = "QueryPie host")]
    host: Option<String>,
    #[arg(short, long, global = true, help = "Print verbose diagnostics")]
    verbose: bool,
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Path to a QueryPie CLI config file"
    )]
    config: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone)]
pub(super) struct Global {
    host: String,
    connection: String,
    engine: String,
    database: String,
    schema: String,
    verbose: bool,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let (global, command) = cli.into_global_and_command()?;
    command.run(&global)
}

impl Cli {
    fn into_global_and_command(self) -> Result<(Global, Command)> {
        let cfg = config::load(self.config.as_deref())?;
        let mut global = Global {
            host: pick_non_empty(self.host, cfg.host),
            connection: cfg.connection,
            engine: String::new(),
            database: cfg.database,
            schema: String::new(),
            verbose: self.verbose,
        };
        self.command.apply_selection(&mut global);
        Ok((global, self.command))
    }
}

impl Global {
    pub(in crate::cli) fn set_connection(&mut self, value: &Option<String>) {
        replace_with_non_empty(&mut self.connection, value);
    }

    pub(in crate::cli) fn set_engine(&mut self, value: &Option<String>) {
        replace_with(&mut self.engine, value);
    }

    pub(in crate::cli) fn set_database(&mut self, value: &Option<String>) {
        replace_with_non_empty(&mut self.database, value);
    }

    pub(in crate::cli) fn set_schema(&mut self, value: &Option<String>) {
        replace_with(&mut self.schema, value);
    }
}

pub fn render_error(err: &anyhow::Error) {
    if let Some(err) = err.downcast_ref::<AuthLoginFailed>() {
        anstream::eprintln!("{} {}", style::error_icon(), err.message());
        return;
    }
    if auth::is_login_canceled(err) {
        anstream::eprintln!("{} {}", style::error_icon(), err);
        return;
    }
    if err.downcast_ref::<AuthStatusFailed>().is_some() {
        return;
    }
    if let Some(ge) = err.downcast_ref::<GrpcError>() {
        eprintln!("error: {}", ge.message);
        if let Some(hint) = ge.hint() {
            eprintln!("  {hint}");
        }
    } else {
        eprintln!("error: {err}");
    }
}

#[derive(Debug)]
pub(super) struct AuthLoginFailed {
    message: String,
}

impl AuthLoginFailed {
    fn message(&self) -> &str {
        &self.message
    }
}

impl From<anyhow::Error> for AuthLoginFailed {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: capitalize_first(&err.to_string()),
        }
    }
}

impl fmt::Display for AuthLoginFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AuthLoginFailed {}

#[derive(Debug)]
pub(super) struct AuthStatusFailed;

impl fmt::Display for AuthStatusFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("auth status failed")
    }
}

impl std::error::Error for AuthStatusFailed {}

fn pick_non_empty(flag: Option<String>, cfg: String) -> String {
    flag.filter(|s| !s.trim().is_empty()).unwrap_or(cfg)
}

fn replace_with_non_empty(target: &mut String, value: &Option<String>) {
    if let Some(value) = value.as_ref().filter(|s| !s.trim().is_empty()) {
        target.clone_from(value);
    }
}

fn replace_with(target: &mut String, value: &Option<String>) {
    if let Some(value) = value {
        target.clone_from(value);
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Login failed".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{error::ErrorKind, CommandFactory};

    #[test]
    fn clap_command_debug_asserts() {
        Cli::command().debug_assert();
    }

    #[test]
    fn subcommands_are_alphabetical() {
        assert_subcommands_are_alphabetical(&Cli::command());
    }

    #[test]
    fn top_level_help_only_shows_global_options() {
        let help = help_for(["querypie", "--help"]);

        assert!(help.contains("--host <HOST>"));
        assert!(help.contains("--config <PATH>"));
        assert!(!help.contains("--connection <CONNECTION>"));
        assert!(!help.contains("--engine <ENGINE>"));
        assert!(!help.contains("--db <DATABASE>"));
        assert!(!help.contains("--schema <SCHEMA>"));
    }

    #[test]
    fn scoped_help_shows_selection_options_on_relevant_commands() {
        let auth = help_for(["querypie", "auth", "--help"]);
        assert!(!auth.contains("--connection"));
        assert!(!auth.contains("--engine"));
        assert!(!auth.contains("--db"));
        assert!(!auth.contains("--schema"));

        let database = help_for(["querypie", "database", "list", "--help"]);
        assert!(database.contains("--connection <CONNECTION>"));
        assert!(database.contains("--engine <ENGINE>"));
        assert!(!database.contains("--db"));
        assert!(!database.contains("--schema"));

        let table = help_for(["querypie", "table", "list", "--help"]);
        assert!(table.contains("--connection <CONNECTION>"));
        assert!(table.contains("--engine <ENGINE>"));
        assert!(table.contains("--db <DATABASE>"));
        assert!(table.contains("--schema <SCHEMA>"));

        let clear = help_for(["querypie", "session", "clear", "--help"]);
        assert!(clear.contains("--connection <CONNECTION>"));
        assert!(!clear.contains("--engine"));
        assert!(!clear.contains("--db"));
        assert!(!clear.contains("--schema"));
    }

    fn help_for<const N: usize>(args: [&str; N]) -> String {
        let err = Cli::command().try_get_matches_from(args).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        err.to_string()
    }

    fn assert_subcommands_are_alphabetical(command: &clap::Command) {
        let actual = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect::<Vec<_>>();
        let mut expected = actual.clone();
        expected.sort_unstable();
        assert_eq!(actual, expected, "{} subcommands", command.get_name());

        for subcommand in command.get_subcommands() {
            assert_subcommands_are_alphabetical(subcommand);
        }
    }
}

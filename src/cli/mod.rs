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
  querypie --host querypie.example.com auth login
  querypie --host querypie.example.com connection list
  querypie --host querypie.example.com -c 'example-main [US]' --engine mysql database list
  querypie --host querypie.example.com -c 'example-main [US]' --engine mysql query 'select 1;'"
)]
struct Cli {
    #[arg(long, global = true, value_name = "HOST", help = "QueryPie host")]
    host: Option<String>,
    #[arg(
        short = 'c',
        long,
        global = true,
        value_name = "CONNECTION",
        help = "QueryPie connection name"
    )]
    connection: Option<String>,
    #[arg(
        long,
        global = true,
        value_name = "ENGINE",
        help = "Database engine name, such as mysql"
    )]
    engine: Option<String>,
    #[arg(
        short = 'd',
        long = "db",
        global = true,
        value_name = "DATABASE",
        help = "Database name to use"
    )]
    database: Option<String>,
    #[arg(
        long,
        global = true,
        value_name = "SCHEMA",
        help = "Schema name to use for table commands"
    )]
    schema: Option<String>,
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
        let global = Global {
            host: pick(self.host, cfg.host),
            connection: pick(self.connection, cfg.connection),
            engine: self.engine.unwrap_or_default(),
            database: pick(self.database, cfg.database),
            schema: self.schema.unwrap_or_default(),
            verbose: self.verbose,
        };
        Ok((global, self.command))
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

fn pick(flag: Option<String>, cfg: String) -> String {
    flag.filter(|s| !s.trim().is_empty()).unwrap_or(cfg)
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Login failed".to_string(),
    }
}

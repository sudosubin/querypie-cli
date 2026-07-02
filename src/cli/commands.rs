use anyhow::Result;
use clap::{Args, Subcommand};

use super::{auth_cmd, data_cmd, session_cmd, Global};
use crate::formatting::{Options as FormatOptions, OutputFormat};

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    #[command(about = "List and inspect QueryPie connections")]
    #[command(after_help = "EXAMPLES:\n  querypie --host querypie.example.com connection list")]
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    #[command(about = "List databases for a QueryPie connection")]
    #[command(
        after_help = "EXAMPLES:\n  querypie -c 'example-main [US]' --engine mysql database list"
    )]
    Database {
        #[command(subcommand)]
        command: DatabaseCommand,
    },
    #[command(about = "List schemas for a database")]
    #[command(
        after_help = "EXAMPLES:\n  querypie -c 'example-main [US]' --engine mysql -d example_db schema list"
    )]
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    #[command(about = "List and inspect tables")]
    #[command(
        after_help = "EXAMPLES:\n  querypie -c 'example-main [US]' --engine mysql table list\n  querypie -c 'example-main [US]' --engine mysql table describe example_table\n  querypie -c 'example-main [US]' --engine mysql table ddl example_table"
    )]
    Table {
        #[command(subcommand)]
        command: TableCommand,
    },
    #[command(about = "Run SQL through QueryPie")]
    #[command(
        after_help = "EXAMPLES:\n  querypie -c 'example-main [US]' --engine mysql query 'select 1;'\n  querypie -c 'example-main [US]' --engine mysql query --limit 10 --output json 'select * from example_table;'"
    )]
    Query {
        sql: String,
        #[arg(long, default_value_t = 1000)]
        limit: i32,
        #[command(flatten)]
        output: OutputArgs,
    },
    #[command(about = "Log in, log out, and inspect authentication")]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    #[command(about = "Manage cached QueryPie database sessions")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum ConnectionCommand {
    #[command(about = "List QueryPie connections")]
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum DatabaseCommand {
    #[command(about = "List databases for the selected connection")]
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum SchemaCommand {
    #[command(about = "List schemas for the selected database")]
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum TableCommand {
    #[command(about = "List tables for the selected schema")]
    List(OutputArgs),
    #[command(about = "Show QueryPie table structure")]
    Describe {
        table: String,
        #[command(flatten)]
        output: OutputArgs,
    },
    #[command(about = "Show DDL for a table")]
    Ddl {
        table: String,
        #[command(flatten)]
        output: OutputArgs,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum SessionCommand {
    #[command(about = "List cached QueryPie database sessions")]
    List(OutputArgs),
    #[command(about = "Clear cached QueryPie database sessions")]
    Clear,
}

#[derive(Debug, Subcommand)]
pub(super) enum AuthCommand {
    #[command(about = "Open a webview and log in to QueryPie")]
    Login,
    #[command(about = "Log out and remove QueryPie webview session data")]
    Logout,
    #[command(about = "Show current QueryPie authentication status")]
    Status,
    #[command(hide = true)]
    ReadCookie,
    #[command(hide = true)]
    RefreshCookie,
}

#[derive(Debug, Clone, Copy, Args)]
pub(super) struct OutputArgs {
    #[arg(
        short = 'o',
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        help = "Output format"
    )]
    pub(super) output: OutputFormat,
    #[arg(long, help = "Do not truncate table output")]
    pub(super) no_truncate: bool,
}

impl Command {
    pub(super) fn run(self, global: &Global) -> Result<()> {
        match self {
            Command::Connection { command } => command.run(global),
            Command::Database { command } => command.run(global),
            Command::Schema { command } => command.run(global),
            Command::Table { command } => command.run(global),
            Command::Query { sql, limit, output } => {
                data_cmd::run_query(global, sql, limit, output)
            }
            Command::Auth { command } => command.run(global),
            Command::Session { command } => command.run(global),
        }
    }
}

impl ConnectionCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            ConnectionCommand::List(output) => data_cmd::list_connections(global, output),
        }
    }
}

impl DatabaseCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            DatabaseCommand::List(output) => data_cmd::list_databases(global, output),
        }
    }
}

impl SchemaCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            SchemaCommand::List(output) => data_cmd::list_schemas(global, output),
        }
    }
}

impl TableCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            TableCommand::List(output) => data_cmd::list_tables(global, output),
            TableCommand::Describe { table, output } => {
                data_cmd::describe_table(global, table, output)
            }
            TableCommand::Ddl { table, output } => data_cmd::show_table_ddl(global, table, output),
        }
    }
}

impl AuthCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            AuthCommand::Login => auth_cmd::auth_login(global),
            AuthCommand::Logout => auth_cmd::auth_logout(global),
            AuthCommand::Status => auth_cmd::auth_status(global),
            AuthCommand::ReadCookie => auth_cmd::auth_read_cookie(global),
            AuthCommand::RefreshCookie => auth_cmd::auth_refresh_cookie(global),
        }
    }
}

impl SessionCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            SessionCommand::List(output) => session_cmd::list_cached_sessions(output),
            SessionCommand::Clear => session_cmd::clear_cached_sessions(global),
        }
    }
}

pub(super) fn fmt(output: OutputArgs) -> FormatOptions {
    FormatOptions {
        output: output.output,
        truncate: !output.no_truncate && !no_truncate_env(),
    }
}

fn no_truncate_env() -> bool {
    std::env::var("QUERYPIE_NO_TRUNCATE")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes"
            )
        })
        .unwrap_or(false)
}

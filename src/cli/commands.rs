use anyhow::Result;
use clap::{Args, Subcommand};

use super::{auth_cmd, data_cmd, session_cmd, Global};
use crate::formatting::{Options as FormatOptions, OutputFormat};

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    #[command(about = "Log in, log out, and inspect authentication")]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    #[command(about = "List and inspect QueryPie connections")]
    #[command(after_help = "EXAMPLES:\n  querypie connection list")]
    Connection {
        #[command(subcommand)]
        command: ConnectionCommand,
    },
    #[command(about = "List databases for a QueryPie connection")]
    #[command(after_help = "EXAMPLES:\n  querypie database list -c CONNECTION")]
    Database {
        #[command(subcommand)]
        command: DatabaseCommand,
    },
    #[command(about = "Run SQL through QueryPie")]
    #[command(after_help = "EXAMPLES:\n  querypie query -c CONNECTION 'select 1;'")]
    Query {
        #[command(flatten)]
        selection: DatabaseSelectionArgs,
        sql: String,
        #[arg(long, default_value_t = 1000)]
        limit: i32,
        #[command(flatten)]
        output: OutputArgs,
    },
    #[command(about = "List schemas for a database")]
    #[command(after_help = "EXAMPLES:\n  querypie schema list -c CONNECTION -d DATABASE")]
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    #[command(about = "Manage cached QueryPie database sessions")]
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    #[command(about = "List and inspect tables")]
    #[command(
        after_help = "EXAMPLES:\n  querypie table ddl -c CONNECTION -d DATABASE TABLE\n  querypie table describe -c CONNECTION -d DATABASE TABLE\n  querypie table list -c CONNECTION -d DATABASE"
    )]
    Table {
        #[command(subcommand)]
        command: TableCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum AuthCommand {
    #[command(about = "Open a webview and log in to QueryPie")]
    Login,
    #[command(about = "Log out and remove QueryPie webview session data")]
    Logout,
    #[command(hide = true)]
    ReadCookie,
    #[command(hide = true)]
    RefreshCookie,
    #[command(about = "Show current QueryPie authentication status")]
    Status,
}

#[derive(Debug, Subcommand)]
pub(super) enum ConnectionCommand {
    #[command(about = "List QueryPie connections")]
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum DatabaseCommand {
    #[command(about = "List databases for the selected connection")]
    List {
        #[command(flatten)]
        selection: ConnectionSelectionArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum SchemaCommand {
    #[command(about = "List schemas for the selected database")]
    List {
        #[command(flatten)]
        selection: DatabaseSelectionArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum SessionCommand {
    #[command(about = "Clear cached QueryPie database sessions")]
    Clear {
        #[command(flatten)]
        selection: ConnectionArg,
    },
    #[command(about = "List cached QueryPie database sessions")]
    List(OutputArgs),
}

#[derive(Debug, Subcommand)]
pub(super) enum TableCommand {
    #[command(about = "Show DDL for a table")]
    Ddl {
        #[command(flatten)]
        selection: TableSelectionArgs,
        #[command(flatten)]
        output: OutputArgs,
        table: String,
    },
    #[command(about = "Show QueryPie table structure")]
    Describe {
        #[command(flatten)]
        selection: TableSelectionArgs,
        #[command(flatten)]
        output: OutputArgs,
        table: String,
    },
    #[command(about = "List tables for the selected schema")]
    List {
        #[command(flatten)]
        selection: TableSelectionArgs,
        #[command(flatten)]
        output: OutputArgs,
    },
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

#[derive(Debug, Clone, Args)]
pub(super) struct ConnectionArg {
    #[arg(
        short = 'c',
        long,
        value_name = "CONNECTION",
        help = "QueryPie connection name"
    )]
    connection: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ConnectionSelectionArgs {
    #[command(flatten)]
    connection: ConnectionArg,
    #[arg(
        long,
        value_name = "ENGINE",
        help = "Database engine name, such as mysql"
    )]
    engine: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DatabaseSelectionArgs {
    #[command(flatten)]
    connection: ConnectionSelectionArgs,
    #[arg(
        short = 'd',
        long = "db",
        value_name = "DATABASE",
        help = "Database name to use"
    )]
    database: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct TableSelectionArgs {
    #[command(flatten)]
    database: DatabaseSelectionArgs,
    #[arg(long, value_name = "SCHEMA", help = "Schema name to use")]
    schema: Option<String>,
}

impl Command {
    pub(super) fn run(self, global: &Global) -> Result<()> {
        match self {
            Command::Auth { command } => command.run(global),
            Command::Connection { command } => command.run(global),
            Command::Database { command } => command.run(global),
            Command::Query {
                sql, limit, output, ..
            } => data_cmd::run_query(global, sql, limit, output),
            Command::Schema { command } => command.run(global),
            Command::Session { command } => command.run(global),
            Command::Table { command } => command.run(global),
        }
    }

    pub(super) fn apply_selection(&self, global: &mut Global) {
        match self {
            Command::Auth { .. } | Command::Connection { .. } => {}
            Command::Database { command } => command.apply_selection(global),
            Command::Query { selection, .. } => selection.apply_to(global),
            Command::Schema { command } => command.apply_selection(global),
            Command::Session { command } => command.apply_selection(global),
            Command::Table { command } => command.apply_selection(global),
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
            DatabaseCommand::List { output, .. } => data_cmd::list_databases(global, output),
        }
    }

    fn apply_selection(&self, global: &mut Global) {
        match self {
            DatabaseCommand::List { selection, .. } => selection.apply_to(global),
        }
    }
}

impl SchemaCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            SchemaCommand::List { output, .. } => data_cmd::list_schemas(global, output),
        }
    }

    fn apply_selection(&self, global: &mut Global) {
        match self {
            SchemaCommand::List { selection, .. } => selection.apply_to(global),
        }
    }
}

impl TableCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            TableCommand::Ddl { table, output, .. } => {
                data_cmd::show_table_ddl(global, table, output)
            }
            TableCommand::Describe { table, output, .. } => {
                data_cmd::describe_table(global, table, output)
            }
            TableCommand::List { output, .. } => data_cmd::list_tables(global, output),
        }
    }

    fn apply_selection(&self, global: &mut Global) {
        match self {
            TableCommand::Ddl { selection, .. }
            | TableCommand::Describe { selection, .. }
            | TableCommand::List { selection, .. } => selection.apply_to(global),
        }
    }
}

impl AuthCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            AuthCommand::Login => auth_cmd::auth_login(global),
            AuthCommand::Logout => auth_cmd::auth_logout(global),
            AuthCommand::ReadCookie => auth_cmd::auth_read_cookie(global),
            AuthCommand::RefreshCookie => auth_cmd::auth_refresh_cookie(global),
            AuthCommand::Status => auth_cmd::auth_status(global),
        }
    }
}

impl SessionCommand {
    fn run(self, global: &Global) -> Result<()> {
        match self {
            SessionCommand::Clear { .. } => session_cmd::clear_cached_sessions(global),
            SessionCommand::List(output) => session_cmd::list_cached_sessions(output),
        }
    }

    fn apply_selection(&self, global: &mut Global) {
        match self {
            SessionCommand::Clear { selection } => selection.apply_to(global),
            SessionCommand::List(_) => {}
        }
    }
}

impl ConnectionArg {
    fn apply_to(&self, global: &mut Global) {
        global.set_connection(&self.connection);
    }
}

impl ConnectionSelectionArgs {
    fn apply_to(&self, global: &mut Global) {
        self.connection.apply_to(global);
        global.set_engine(&self.engine);
    }
}

impl DatabaseSelectionArgs {
    fn apply_to(&self, global: &mut Global) {
        self.connection.apply_to(global);
        global.set_database(&self.database);
    }
}

impl TableSelectionArgs {
    fn apply_to(&self, global: &mut Global) {
        self.database.apply_to(global);
        global.set_schema(&self.schema);
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

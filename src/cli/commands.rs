use anyhow::Result;
use clap::{Args, Subcommand};

use super::Global;
use crate::formatting::{self, Options as FormatOptions, OutputFormat};

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
        let _ = (
            &global.host,
            &global.connection,
            &global.engine,
            &global.database,
            &global.schema,
            global.verbose,
        );
        match self {
            Command::Connection { command } => command.run(),
            Command::Database { command } => command.run(),
            Command::Schema { command } => command.run(),
            Command::Table { command } => command.run(),
            Command::Query { output, .. } => formatting::script("", fmt(output)),
            Command::Auth { .. } | Command::Session { .. } => {
                let _ = (
                    crate::formatting::style::success_icon(),
                    crate::formatting::style::error_icon(),
                    crate::formatting::style::null_value(),
                );
                Ok(())
            }
        }
    }
}

impl ConnectionCommand {
    fn run(self) -> Result<()> {
        match self {
            ConnectionCommand::List(output) => {
                formatting::simple_table(&["NAME"], Vec::<Vec<String>>::new(), fmt(output));
                Ok(())
            }
        }
    }
}

impl DatabaseCommand {
    fn run(self) -> Result<()> {
        match self {
            DatabaseCommand::List(output) => formatting::names(&[], fmt(output)),
        }
    }
}

impl SchemaCommand {
    fn run(self) -> Result<()> {
        match self {
            SchemaCommand::List(output) => formatting::names(&[], fmt(output)),
        }
    }
}

impl TableCommand {
    fn run(self) -> Result<()> {
        match self {
            TableCommand::List(output) => formatting::names(&[], fmt(output)),
            TableCommand::Describe { output, .. } | TableCommand::Ddl { output, .. } => {
                formatting::script("", fmt(output))
            }
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

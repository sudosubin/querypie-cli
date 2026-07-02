use clap::{Args, Subcommand, ValueEnum};

use super::Global;

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
    output: OutputFormat,
    #[arg(long, help = "Do not truncate table output")]
    no_truncate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

impl Command {
    pub(super) fn run(self, global: &Global) {
        let _ = (
            &global.host,
            &global.connection,
            &global.engine,
            &global.database,
            &global.schema,
            global.verbose,
        );
        match self {
            Command::Connection { .. }
            | Command::Database { .. }
            | Command::Schema { .. }
            | Command::Table { .. }
            | Command::Query { .. }
            | Command::Auth { .. }
            | Command::Session { .. } => {}
        }
    }
}

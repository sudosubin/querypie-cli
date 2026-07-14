use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use clap_complete::Shell;

use super::{auth_cmd, data_cmd, session_cmd, Global};
use crate::formatting::{Options as FormatOptions, OutputFormat};

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    #[command(about = "Log in, log out, and inspect authentication")]
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    #[command(about = "Generate shell completions")]
    Completion {
        #[arg(value_enum)]
        shell: Shell,
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
        #[command(flatten)]
        args: QueryArgs,
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
        #[arg(add = clap_complete::ArgValueCompleter::new(super::completion::complete_tables))]
        table: String,
    },
    #[command(about = "Show QueryPie table structure")]
    Describe {
        #[command(flatten)]
        selection: TableSelectionArgs,
        #[command(flatten)]
        output: OutputArgs,
        #[arg(add = clap_complete::ArgValueCompleter::new(super::completion::complete_tables))]
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

#[derive(Debug, Args)]
pub(super) struct QueryArgs {
    #[arg(
        short = 'f',
        long,
        value_name = "PATH",
        help = "Read SQL from a file",
        display_order = 5
    )]
    file: Option<PathBuf>,
    #[arg(
        long,
        default_value_t = 1000,
        value_parser = clap::value_parser!(i32).range(1..),
        help = "Maximum rows to fetch",
        display_order = 7
    )]
    limit: i32,
    #[command(flatten)]
    output: OutputArgs,
    #[arg(value_name = "SQL")]
    sql: Option<String>,
}

#[derive(Debug, Clone, Copy, Args)]
pub(super) struct OutputArgs {
    #[arg(long, help = "Do not truncate table output", display_order = 8)]
    pub(super) no_truncate: bool,
    #[arg(
        short = 'o',
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        help = "Output format",
        display_order = 9
    )]
    pub(super) output: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct ConnectionArg {
    #[arg(
        short = 'c',
        long,
        value_name = "CONNECTION",
        help = "QueryPie connection name",
        add = clap_complete::ArgValueCompleter::new(super::completion::complete_connections),
        display_order = 2
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
        help = "Database engine name, such as mysql",
        add = clap_complete::ArgValueCompleter::new(super::completion::complete_engines),
        display_order = 4
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
        help = "Database name to use",
        add = clap_complete::ArgValueCompleter::new(super::completion::complete_databases),
        display_order = 3
    )]
    database: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub(super) struct TableSelectionArgs {
    #[command(flatten)]
    database: DatabaseSelectionArgs,
    #[arg(
        long,
        value_name = "SCHEMA",
        help = "Schema name to use",
        add = clap_complete::ArgValueCompleter::new(super::completion::complete_schemas),
        display_order = 10
    )]
    schema: Option<String>,
}

impl Command {
    pub(super) fn run(self, global: &Global) -> Result<()> {
        match self {
            Command::Auth { command } => command.run(global),
            Command::Completion { .. } => Ok(()),
            Command::Connection { command } => command.run(global),
            Command::Database { command } => command.run(global),
            Command::Query { args, .. } => {
                let (sql, limit, output) = args.into_sql_limit_output()?;
                data_cmd::run_query(global, sql, limit, output)
            }
            Command::Schema { command } => command.run(global),
            Command::Session { command } => command.run(global),
            Command::Table { command } => command.run(global),
        }
    }

    pub(super) fn apply_selection(&self, global: &mut Global) {
        match self {
            Command::Auth { .. } | Command::Completion { .. } | Command::Connection { .. } => {}
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

impl QueryArgs {
    fn into_sql_limit_output(self) -> Result<(String, i32, OutputArgs)> {
        let QueryArgs {
            sql,
            file,
            limit,
            output,
        } = self;
        let stdin_is_terminal = io::stdin().is_terminal();
        let sql = resolve_query_sql(sql, file, stdin_is_terminal, read_stdin, read_file)?;
        Ok((sql, limit, output))
    }
}

pub(super) fn fmt(output: OutputArgs) -> FormatOptions {
    FormatOptions {
        output: output.output,
        truncate: !output.no_truncate && !no_truncate_env(),
    }
}

const NO_SQL_ERROR: &str = "no SQL provided; pass SQL, use `query -`, or use `query --file <PATH>`";
const MULTIPLE_SQL_SOURCES_ERROR: &str =
    "provide exactly one SQL source; pass SQL, use `query -`, or use `query --file <PATH>`";

enum QuerySqlSource {
    Inline(String),
    Stdin,
    File(PathBuf),
}

fn resolve_query_sql(
    sql: Option<String>,
    file: Option<PathBuf>,
    stdin_is_terminal: bool,
    mut read_stdin: impl FnMut() -> Result<String>,
    mut read_file: impl FnMut(&Path) -> Result<String>,
) -> Result<String> {
    let sql = match query_sql_source(sql, file, stdin_is_terminal)? {
        QuerySqlSource::Inline(sql) => sql,
        QuerySqlSource::Stdin => read_stdin()?,
        QuerySqlSource::File(path) => read_file(&path)?,
    };

    validate_query_sql(sql)
}

fn query_sql_source(
    sql: Option<String>,
    file: Option<PathBuf>,
    stdin_is_terminal: bool,
) -> Result<QuerySqlSource> {
    match (sql, file, stdin_is_terminal) {
        (Some(_), Some(_), _) => bail!(MULTIPLE_SQL_SOURCES_ERROR),
        (None, Some(path), _) if path == Path::new("-") => {
            bail!("`--file -` is not supported; use `query -` to read SQL from stdin")
        }
        (None, Some(path), _) => Ok(QuerySqlSource::File(path)),
        (Some(sql), None, _) if sql == "-" => Ok(QuerySqlSource::Stdin),
        (Some(sql), None, _) => Ok(QuerySqlSource::Inline(sql)),
        (None, None, false) => Ok(QuerySqlSource::Stdin),
        (None, None, true) => bail!(NO_SQL_ERROR),
    }
}

fn validate_query_sql(sql: String) -> Result<String> {
    if sql.trim().is_empty() {
        bail!("SQL is empty");
    }

    Ok(sql)
}

fn read_stdin() -> Result<String> {
    let mut sql = String::new();
    io::stdin()
        .read_to_string(&mut sql)
        .context("failed to read SQL from stdin")?;
    Ok(sql)
}

fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read SQL from {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(
        sql: Option<&str>,
        file: Option<&str>,
        stdin_is_terminal: bool,
        stdin_sql: &str,
        file_sql: &str,
    ) -> Result<String> {
        resolve_query_sql(
            sql.map(str::to_owned),
            file.map(PathBuf::from),
            stdin_is_terminal,
            || Ok(stdin_sql.to_owned()),
            |_| Ok(file_sql.to_owned()),
        )
    }

    #[test]
    fn resolves_sql_sources() {
        let cases = [
            (
                "positional sql",
                Some("  select 1;\n"),
                None,
                true,
                "from stdin",
                "from file",
                "  select 1;\n",
            ),
            (
                "dash from stdin",
                Some("-"),
                None,
                true,
                "select from stdin;\n",
                "from file",
                "select from stdin;\n",
            ),
            (
                "piped stdin",
                None,
                None,
                false,
                "select from stdin;\n",
                "from file",
                "select from stdin;\n",
            ),
            (
                "file",
                None,
                Some("query.sql"),
                true,
                "from stdin",
                "  select from file;\n",
                "  select from file;\n",
            ),
        ];

        for (name, sql, file, stdin_is_terminal, stdin_sql, file_sql, expected) in cases {
            let actual = resolve(sql, file, stdin_is_terminal, stdin_sql, file_sql)
                .unwrap_or_else(|err| panic!("{name}: {err:#}"));

            assert_eq!(actual, expected, "{name}");
        }
    }

    #[test]
    fn rejects_invalid_sql_sources() {
        let cases = [
            (
                "file and positional sql",
                Some("select 1;"),
                Some("query.sql"),
                true,
                MULTIPLE_SQL_SOURCES_ERROR,
            ),
            ("empty sql", Some("   \n\t"), None, true, "SQL is empty"),
            ("file dash", None, Some("-"), true, "use `query -`"),
            ("missing sql", None, None, true, NO_SQL_ERROR),
        ];

        for (name, sql, file, stdin_is_terminal, expected) in cases {
            let err = resolve(sql, file, stdin_is_terminal, "from stdin", "from file")
                .expect_err(name)
                .to_string();

            assert!(
                err.contains(expected),
                "{name}: expected {err:?} to contain {expected:?}"
            );
        }
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;
    use crate::cli::Cli;

    #[test]
    fn query_limit_starts_at_one() {
        for arg in ["--limit=-1", "--limit=0"] {
            let err = Cli::try_parse_from(["querypie", "query", arg, "select 1;"])
                .expect_err("limit below 1 should be rejected");
            assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
        }

        let cli = Cli::try_parse_from(["querypie", "query", "--limit", "1", "select 1;"])
            .expect("limit 1 should parse");

        let Command::Query { args, .. } = cli.command else {
            panic!("expected query command");
        };
        assert_eq!(args.limit, 1);
    }
}

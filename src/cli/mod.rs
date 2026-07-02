mod commands;

use clap::Parser;

use self::commands::Command;
use crate::config;

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

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = config::load(cli.config.as_deref())?;
    let global = Global {
        host: pick(cli.host, cfg.host),
        connection: pick(cli.connection, cfg.connection),
        engine: cli.engine.unwrap_or_default(),
        database: pick(cli.database, cfg.database),
        schema: cli.schema.unwrap_or_default(),
        verbose: cli.verbose,
    };
    cli.command.run(&global);
    Ok(())
}

pub fn render_error(err: &dyn std::error::Error) {
    eprintln!("error: {err}");
}

fn pick(flag: Option<String>, cfg: String) -> String {
    flag.filter(|s| !s.trim().is_empty()).unwrap_or(cfg)
}

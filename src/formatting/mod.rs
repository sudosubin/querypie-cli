pub(crate) mod style;
mod table;

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

use self::table::print_table;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub output: OutputFormat,
    pub truncate: bool,
}

pub fn names(names: &[String], opts: Options) -> Result<()> {
    if opts.output == OutputFormat::Json {
        print_json(names)?;
    } else {
        for name in names {
            println!("{name}");
        }
    }
    Ok(())
}

pub fn script(script: &str, opts: Options) -> Result<()> {
    if opts.output == OutputFormat::Json {
        print_json(&serde_json::json!({ "script": script }))?;
    } else {
        println!("{script}");
    }
    Ok(())
}

pub fn simple_table(headers: &[&str], rows: impl IntoIterator<Item = Vec<String>>, opts: Options) {
    print_table(headers, rows, opts.truncate);
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

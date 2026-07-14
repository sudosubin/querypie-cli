pub(crate) mod style;
mod table;

use anyhow::Result;
use clap::ValueEnum;
use serde::Serialize;

use crate::auth::{AuthCheck, AuthState};
use crate::qpapi::{Connection, ResultSet, TableStructure};
use crate::sessioncache;

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

pub fn connections(conns: &[Connection], opts: Options) -> Result<()> {
    #[derive(Serialize)]
    struct Row<'a> {
        name: &'a str,
        engine: &'a str,
        access: &'a str,
        deactivated: bool,
        cluster_uuid: &'a str,
        db_type: i32,
    }

    let rows = conns
        .iter()
        .map(|c| Row {
            name: &c.name,
            engine: c.engine(),
            access: connection_access(c),
            deactivated: c.deactivated,
            cluster_uuid: &c.cluster_uuid,
            db_type: c.db_type,
        })
        .collect::<Vec<_>>();

    if opts.output == OutputFormat::Json {
        print_json(&rows)?;
    } else {
        print_table(
            &["NAME", "ENGINE", "ACCESS"],
            rows.iter().map(|row| {
                vec![
                    row.name.to_string(),
                    row.engine.to_string(),
                    row.access.to_string(),
                ]
            }),
            opts.truncate,
        );
    }
    Ok(())
}

pub fn sessions(entries: &[sessioncache::Entry], opts: Options) -> Result<()> {
    if opts.output == OutputFormat::Json {
        #[derive(Serialize)]
        struct Row<'a> {
            host: &'a str,
            connection: &'a str,
            engine: &'a str,
            db: &'a str,
            session: &'a str,
            opened_at: i64,
        }
        let rows = entries
            .iter()
            .map(|entry| Row {
                host: &entry.host,
                connection: &entry.connection,
                engine: &entry.engine,
                db: &entry.db,
                session: &entry.session,
                opened_at: entry.opened_at,
            })
            .collect::<Vec<_>>();
        print_json(&rows)?;
    } else {
        print_table(
            &["HOST", "CONNECTION", "ENGINE", "DB", "SESSION"],
            entries.iter().map(|entry| {
                vec![
                    entry.host.clone(),
                    entry.connection.clone(),
                    entry.engine.clone(),
                    entry.db.clone(),
                    entry.session.clone(),
                ]
            }),
            opts.truncate,
        );
    }
    Ok(())
}

pub fn auth_status(checks: &[AuthCheck]) -> Result<bool> {
    let mut failed = false;
    for (index, check) in checks.iter().enumerate() {
        if index > 0 {
            println!();
        }
        println!("{}", check.host);
        match check.state {
            AuthState::Missing => {
                failed = true;
                anstream::println!(
                    "  {} Failed to log in to {}",
                    style::error_icon(),
                    check.host
                );
                println!("  - No QueryPie webview session was found.");
                println!(
                    "  - To authenticate, run: querypie --host {} auth login",
                    check.host
                );
            }
            AuthState::Valid => {
                anstream::println!("  {} Logged in to {}", style::success_icon(), check.host);
                println!("  - Webview session: active");
                println!("  - Cookie store: QueryPie webview profile");
            }
            AuthState::Expired => {
                failed = true;
                anstream::println!(
                    "  {} Failed to log in to {}",
                    style::error_icon(),
                    check.host
                );
                println!("  - The QueryPie webview session is expired.");
                println!(
                    "  - To re-authenticate, run: querypie --host {} auth login",
                    check.host
                );
            }
        }
    }
    Ok(failed)
}

pub fn script(script: &str, opts: Options) -> Result<()> {
    if opts.output == OutputFormat::Json {
        print_json(&serde_json::json!({ "script": script }))?;
    } else {
        println!("{script}");
    }
    Ok(())
}

pub fn table_structure(structure: &TableStructure, opts: Options) -> Result<()> {
    if opts.output == OutputFormat::Json {
        print_json(structure)?;
        return Ok(());
    }

    print_table(
        &structure
            .headers
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        structure.rows.iter().cloned(),
        opts.truncate,
    );
    Ok(())
}

pub fn query_result(res: &ResultSet, limit: i32, opts: Options) -> Result<()> {
    if opts.output == OutputFormat::Json {
        print_json(&query_result_json(res, limit))?;
        return Ok(());
    }

    print_table(
        &res.columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        res.rows.iter().map(|row| {
            row.iter()
                .map(|c| {
                    if c.is_null {
                        style::null_value()
                    } else {
                        c.value.clone()
                    }
                })
                .collect::<Vec<_>>()
        }),
        opts.truncate,
    );
    println!("\n{}", query_result_summary(res, limit));
    Ok(())
}

fn connection_access(conn: &Connection) -> &'static str {
    if conn.deactivated {
        "expired"
    } else {
        "active"
    }
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn query_rows_json(res: &ResultSet) -> Vec<Vec<serde_json::Value>> {
    res.rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    if cell.is_null {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(cell.value.clone())
                    }
                })
                .collect()
        })
        .collect()
}

fn query_result_json(res: &ResultSet, limit: i32) -> serde_json::Value {
    serde_json::json!({
        "columns": &res.columns,
        "rows": query_rows_json(res),
        "limit": limit,
        "limit_reached": query_limit_reached(res.rows.len(), limit),
    })
}

fn query_limit_reached(row_count: usize, limit: i32) -> bool {
    row_count >= limit as usize
}

fn query_result_summary(res: &ResultSet, limit: i32) -> String {
    let row_count = res.rows.len();
    if query_limit_reached(row_count, limit) {
        format!("({row_count} rows, limit reached)")
    } else {
        format!("({row_count} rows)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qpapi::{Cell, ColumnInfo};

    #[test]
    fn query_summary_reports_limit_status() {
        for (row_count, limit, expected) in [
            (1000, 1000, "(1000 rows, limit reached)"),
            (42, 1000, "(42 rows)"),
        ] {
            assert_eq!(
                query_result_summary(&result_set(row_count), limit),
                expected
            );
        }
    }

    #[test]
    fn query_json_includes_limit_metadata() {
        let res = result_set(1);

        let value = query_result_json(&res, 1);

        assert_eq!(value["limit"], 1);
        assert_eq!(value["limit_reached"], true);
        assert_eq!(value["columns"][0]["name"], "id");
        assert_eq!(value["rows"][0][0], "1");
    }

    fn result_set(row_count: usize) -> ResultSet {
        ResultSet {
            columns: vec![ColumnInfo {
                name: "id".to_string(),
                type_name: "System.Int64".to_string(),
            }],
            rows: (0..row_count)
                .map(|index| {
                    vec![Cell {
                        value: (index + 1).to_string(),
                        is_null: false,
                    }]
                })
                .collect(),
        }
    }
}

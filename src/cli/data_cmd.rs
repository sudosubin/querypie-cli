use anyhow::Result;

use super::commands::{fmt, OutputArgs};
use super::Global;
use crate::formatting;

pub(super) fn list_connections(global: &Global, output: OutputArgs) -> Result<()> {
    let mut client = global.new_client()?;
    let conns = match client.connections() {
        Err(err) if super::session::is_auth_expired(&err) => {
            let cookie = global.refresh_cookie_or_error()?;
            client = global.new_client_with_cookie(cookie)?;
            client.connections()?
        }
        other => other?,
    };
    formatting::connections(&conns, fmt(output))
}

pub(super) fn list_databases(global: &Global, output: OutputArgs) -> Result<()> {
    global.with_session(|r| {
        let names = r.client.get_databases(&r.session, &r.db)?;
        formatting::names(&names, fmt(output))
    })
}

pub(super) fn list_schemas(global: &Global, output: OutputArgs) -> Result<()> {
    global.with_session(|r| {
        let names = r.client.get_schemas(&r.session, &r.db)?;
        formatting::names(&names, fmt(output))
    })
}

pub(super) fn list_tables(global: &Global, output: OutputArgs) -> Result<()> {
    global.with_session(|r| {
        let names = r.client.get_tables(&r.session, &r.db, &global.schema)?;
        formatting::names(&names, fmt(output))
    })
}

pub(super) fn show_table_ddl(global: &Global, table: String, output: OutputArgs) -> Result<()> {
    global.with_session(|r| {
        let script = r.client.get_table_script(&r.session, &r.db, &table)?;
        formatting::script(&script, fmt(output))
    })
}

pub(super) fn describe_table(global: &Global, table: String, output: OutputArgs) -> Result<()> {
    global.with_session(|r| {
        let structure = r.client.get_table_structure(&r.session, &r.db, &table)?;
        formatting::table_structure(&structure, fmt(output))
    })
}

pub(super) fn run_query(
    global: &Global,
    sql: String,
    limit: i32,
    output: OutputArgs,
) -> Result<()> {
    global.with_session(|r| {
        let res = r.client.query(&r.session, &r.db, &sql, limit, r.db_type)?;
        formatting::query_result(&res, limit, fmt(output))
    })
}

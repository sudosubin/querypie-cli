use anyhow::{anyhow, Result};
use serde::Serialize;

use super::pb;
use super::Client;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TableStructure {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Client {
    pub fn get_databases(&self, session: &str, db: &str) -> Result<Vec<String>> {
        let resp: pb::NameListResponse = self.unary(
            "engine.dictionary.database.DatabaseDictionaryService/getDatabases",
            &pb::GetDatabasesRequest {
                session: session.to_string(),
                db: db.to_string(),
            },
        )?;
        Ok(resp.names)
    }

    pub fn get_schemas(&self, session: &str, db: &str) -> Result<Vec<String>> {
        let resp: pb::NameListResponse = self.unary(
            "engine.dictionary.database.DatabaseDictionaryService/getSchemas",
            &pb::GetSchemasRequest {
                session: session.to_string(),
                db: db.to_string(),
            },
        )?;
        Ok(resp.names)
    }

    pub fn get_tables(&self, session: &str, db: &str, schema: &str) -> Result<Vec<String>> {
        let resp: pb::NameListResponse = self.unary(
            "engine.dictionary.table.TableDictionaryService/getTables",
            &pb::GetTablesRequest {
                session: session.to_string(),
                db: db.to_string(),
                schema: schema.to_string(),
                flag: 1,
            },
        )?;
        Ok(resp.names)
    }

    pub fn get_table_script(&self, session: &str, db: &str, table: &str) -> Result<String> {
        let resp: pb::ScriptResponse = self.unary(
            "engine.dictionary.table.TableDictionaryService/getTableScript",
            &pb::GetTableScriptRequest {
                session: session.to_string(),
                db: db.to_string(),
                table: table.to_string(),
                flag: 1,
            },
        )?;
        Ok(resp.script)
    }

    pub fn get_table_structure(
        &self,
        session: &str,
        db: &str,
        table: &str,
    ) -> Result<TableStructure> {
        let resp: pb::StructureResponse = self.unary(
            "engine.dictionary.table.TableDictionaryService/getTableStructure",
            &pb::GetTableScriptRequest {
                session: session.to_string(),
                db: db.to_string(),
                table: table.to_string(),
                flag: 1,
            },
        )?;
        decode_table_structure(resp)
    }
}

fn decode_table_structure(resp: pb::StructureResponse) -> Result<TableStructure> {
    let data_table = resp
        .data_table
        .ok_or_else(|| anyhow!("table structure response did not include data_table"))?;
    let rows = data_table
        .rows
        .into_iter()
        .map(|row| row.values)
        .collect::<Vec<_>>();
    Ok(TableStructure {
        headers: data_table.columns,
        rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_table_structure() {
        // given
        let response = pb::StructureResponse {
            data_table: Some(pb::DictionaryDataTable {
                columns: vec!["ColumnName".to_string(), "DataType".to_string()],
                rows: vec![pb::DictionaryDataTableRow {
                    values: vec!["id".to_string(), "bigint".to_string()],
                }],
            }),
        };

        // when
        let decoded = decode_table_structure(response).unwrap();

        // then
        assert_eq!(decoded.headers, vec!["ColumnName", "DataType"]);
        assert_eq!(decoded.rows, vec![vec!["id", "bigint"]]);
    }
}

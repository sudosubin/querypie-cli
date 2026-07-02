use anyhow::{anyhow, Result};
use prost::Message;
use rmpv::Value;

use super::pb;
use super::Client;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ColumnInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub value: String,
    pub is_null: bool,
}

#[derive(Debug, Clone)]
pub struct ResultSet {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Vec<Cell>>,
}

impl Client {
    pub fn query(
        &self,
        session: &str,
        db: &str,
        sql: &str,
        limit: i32,
        dialect: i32,
    ) -> Result<ResultSet> {
        let limit = if limit <= 0 { 1000 } else { limit };
        let dialect = if dialect == 0 { 1 } else { dialect };

        let parsed: pb::ParseResponse = self.unary(
            "engine.sql.SQLService/parse",
            &pb::ParseRequest {
                session: session.to_string(),
                dialect,
                db: db.to_string(),
                sql: sql.to_string(),
                editor_model_id: String::new(),
                options: Some(pb::ParseOptions {
                    inner: Some(pb::parse_options::Inner { a: 1 }),
                    reserved3: String::new(),
                }),
            },
        )?;
        let parsed_sql = parsed
            .parsed_sql
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("parse returned no statement"))?;
        let handle = parsed_sql.sql_id.clone();

        let execute_data = self.send_unary(
            "engine.sql.SQLService/execute",
            &pb::ExecuteRequest {
                session: session.to_string(),
                db: db.to_string(),
                sql: sql.to_string(),
                parsed_sql: Some(parsed_sql),
                use_limit: true,
                row_limit: limit,
            },
        )?;
        let execute_columns = decode_execute_columns(&execute_data);

        let gdt = match self.unary(
            "engine.sql.SQLService/getDataTable",
            &pb::GetDataTableRequest {
                session: session.to_string(),
                db: db.to_string(),
                handle,
                row_count: limit,
            },
        ) {
            Ok(gdt) => gdt,
            Err(err) if is_empty_result_error(&err) => {
                return Ok(ResultSet {
                    columns: execute_columns,
                    rows: Vec::new(),
                });
            }
            Err(err) => return Err(err),
        };
        decode_result(&gdt)
    }
}

fn decode_execute_columns(data: &[u8]) -> Vec<ColumnInfo> {
    decode_execute_payload_group(data)
        .or_else(|| {
            pb::KeepAliveExecutionPayloadGroup::decode(data)
                .ok()
                .and_then(|group| group.data)
        })
        .map(|group| {
            group
                .payloads
                .into_iter()
                .filter_map(|payload| payload.executed)
                .flat_map(|executed| executed.columns)
                .map(|column| ColumnInfo {
                    name: column.name,
                    type_name: column.r#type,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn decode_execute_payload_group(data: &[u8]) -> Option<pb::ExecutionPayloadGroup> {
    pb::ExecutionPayloadGroup::decode(data).ok()
}

fn is_empty_result_error(err: &anyhow::Error) -> bool {
    err.downcast_ref::<super::GrpcError>()
        .map(|err| err.message.contains("SqlResultsetNotFound"))
        .unwrap_or_else(|| err.to_string().contains("SqlResultsetNotFound"))
}

fn decode_result(gdt: &pb::GetDataTableResponse) -> Result<ResultSet> {
    let table = gdt
        .table
        .as_ref()
        .ok_or_else(|| anyhow!("getDataTable returned no table"))?;
    let columns = table
        .columns
        .iter()
        .map(|c| ColumnInfo {
            name: c.name.clone(),
            type_name: c.r#type.clone(),
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for blob in &table.rows {
        let values = rmpv::decode::read_value(&mut &blob[..])?;
        let Value::Array(block_rows) = values else {
            continue;
        };
        for row in block_rows {
            let Value::Map(map) = row else {
                continue;
            };
            let cells = map
                .iter()
                .find_map(|(k, v)| (key_name(k) == Some("v")).then_some(v))
                .and_then(|v| match v {
                    Value::Array(values) => Some(values),
                    _ => None,
                });
            let Some(cells) = cells else {
                continue;
            };
            rows.push(cells.iter().map(to_cell).collect());
        }
    }
    Ok(ResultSet { columns, rows })
}
fn to_cell(value: &Value) -> Cell {
    let Value::Map(map) = value else {
        return Cell {
            value: value.to_string(),
            is_null: false,
        };
    };
    let is_null = map
        .iter()
        .find_map(|(k, v)| (key_name(k) == Some("n")).then_some(v))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if is_null {
        return Cell {
            value: String::new(),
            is_null: true,
        };
    }
    let value = map
        .iter()
        .find_map(|(k, v)| (key_name(k) == Some("v")).then_some(v))
        .map(value_to_string)
        .unwrap_or_default();
    Cell {
        value,
        is_null: false,
    }
}

fn key_name(value: &Value) -> Option<&str> {
    value.as_str()
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Nil => String::new(),
        Value::String(s) => s.as_str().unwrap_or_default().to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::F32(f) => f.to_string(),
        Value::F64(f) => f.to_string(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_msgpack_rows_from_data_table() -> Result<()> {
        // given
        let response = pb::GetDataTableResponse {
            table: Some(pb::DataTable {
                handle: "handle".to_string(),
                row_count: 1,
                columns: vec![
                    pb::Column {
                        ordinal: 1,
                        name: "id".to_string(),
                        r#type: "System.Int64".to_string(),
                    },
                    pb::Column {
                        ordinal: 2,
                        name: "name".to_string(),
                        r#type: "System.String".to_string(),
                    },
                ],
                rows: vec![msgpack_rows(vec![vec![
                    cell_value(Value::from(1)),
                    cell_value(Value::from("alice")),
                ]])?],
            }),
        };

        // when
        let result = decode_result(&response)?;

        // then
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.rows[0][0].value, "1");
        assert_eq!(result.rows[0][1].value, "alice");
        Ok(())
    }

    #[test]
    fn decodes_columns_from_execute_payload() -> Result<()> {
        // given
        let group = pb::ExecutionPayloadGroup {
            payloads: vec![pb::ExecutionPayload {
                sql_id: "handle".to_string(),
                executed: Some(pb::CommandExecutedState {
                    sql: "select id where 1 = 0".to_string(),
                    execution_time: 1,
                    db: "example_db".to_string(),
                    object_name: String::new(),
                    columns: vec![pb::Column {
                        ordinal: 1,
                        name: "id".to_string(),
                        r#type: "System.Int64".to_string(),
                    }],
                    is_editable: false,
                }),
                fetching: None,
                complete: None,
                sentence_index: 0,
            }],
        };
        let data = group.encode_to_vec();

        // when
        let columns = decode_execute_columns(&data);

        // then
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[0].type_name, "System.Int64");
        Ok(())
    }

    fn msgpack_rows(rows: Vec<Vec<Value>>) -> Result<Vec<u8>> {
        let rows = rows
            .into_iter()
            .map(|cells| Value::Map(vec![(Value::from("v"), Value::Array(cells))]))
            .collect::<Vec<_>>();
        let mut out = Vec::new();
        rmpv::encode::write_value(&mut out, &Value::Array(rows))?;
        Ok(out)
    }

    fn cell_value(value: Value) -> Value {
        Value::Map(vec![
            (Value::from("v"), value),
            (Value::from("n"), Value::Boolean(false)),
        ])
    }
}

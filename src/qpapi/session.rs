use anyhow::{anyhow, Result};
use uuid::Uuid;

use super::pb;
use super::Client;

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone, serde::Serialize)]
pub struct Connection {
    pub name: String,
    pub cluster_uuid: String,
    pub cluster_id: i32,
    pub db_type: i32,
    pub deactivated: bool,
}

impl Connection {
    pub fn engine(&self) -> &'static str {
        match self.db_type {
            1 => "mysql",
            3 => "postgresql",
            13 => "mongodb",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenedSession {
    pub session: String,
    pub engine: String,
    pub version: String,
    pub db: String,
    pub db_type: i32,
}

impl Client {
    pub fn connections(&self) -> Result<Vec<Connection>> {
        let resp: pb::ClusterGroupsResponse = self.unary(
            "common.connection.ConnectionService/getUserClusterGroups",
            &pb::GetUserClusterGroupsRequest {
                root: ZERO_UUID.to_string(),
            },
        )?;
        let mut out = Vec::new();
        for root in resp.roots {
            for entry in root.entries {
                let Some(item) = entry.item else {
                    continue;
                };
                if item.name.is_empty() || item.uuid.is_empty() {
                    continue;
                }
                out.push(Connection {
                    name: item.name,
                    cluster_uuid: item.uuid,
                    cluster_id: item.id,
                    db_type: item.db_type,
                    deactivated: item.deactivated,
                });
            }
        }
        Ok(out)
    }

    fn find_connection(&self, name: &str, engine: &str) -> Result<Connection> {
        let mut matches = Vec::new();
        let mut exact = false;
        let needle = name.to_ascii_lowercase();
        for conn in self.connections()? {
            if conn.name == name {
                if !exact {
                    matches.clear();
                    exact = true;
                }
                matches.push(conn);
            } else if !exact && conn.name.to_ascii_lowercase().contains(&needle) {
                matches.push(conn);
            }
        }
        if !engine.trim().is_empty() {
            let engine = engine.trim().to_ascii_lowercase();
            matches.retain(|c| c.engine() == engine.as_str());
        }
        match matches.len() {
            1 => Ok(matches.remove(0)),
            0 => Err(anyhow!("connection {name:?} not found")),
            _ => {
                let choices = matches
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.engine()))
                    .collect::<Vec<_>>()
                    .join("\n  ");
                Err(anyhow!(
                    "connection {name:?} is ambiguous; narrow with --engine:\n  {choices}"
                ))
            }
        }
    }

    pub fn open_session(&self, conn_name: &str, engine: &str) -> Result<OpenedSession> {
        let conn = self.find_connection(conn_name, engine)?;
        let insts: pb::InstancesResponse = self.unary(
            "common.connection.ConnectionService/getUserInstances",
            &pb::GetUserInstancesRequest {
                root: ZERO_UUID.to_string(),
                cluster_uuid: conn.cluster_uuid.clone(),
            },
        )?;
        let inst = insts
            .instances
            .first()
            .and_then(|w| w.instance.clone())
            .ok_or_else(|| anyhow!("connection {:?} has no instances", conn.name))?;

        let disp: pb::GetConnectionForDisplayResponse = self.unary(
            "common.connection.ConnectionService/getConnectionForDisplay",
            &pb::GetConnectionForDisplayRequest {
                mode: 3,
                instance_uuid: inst.uuid.clone(),
            },
        )?;
        let conn_section = disp.conn.unwrap_or_default();
        let db_name = conn_section.db_name.clone();
        let host = if conn_section.host.is_empty() {
            inst.host.clone()
        } else {
            conn_section.host.clone()
        };
        let port = if conn_section.port == 0 {
            inst.port
        } else {
            conn_section.port
        };

        let open = pb::OpenRequest {
            flag1: 2,
            instance_uuid: inst.uuid.clone(),
            db_name: db_name.clone(),
            connection: Some(pb::OpenConnection {
                flag1: conn.db_type,
                instance_uuid: inst.uuid.clone(),
                host,
                port,
                flag7: 1,
                db_name,
                cluster_name: conn.name.clone(),
                instance_id: inst.id,
                instance_uuid2: inst.uuid.clone(),
                cluster_id: conn.cluster_id,
                cluster_uuid: conn.cluster_uuid.clone(),
                role: disp.role,
                empty30: String::new(),
                flag43: 1,
                db_account: conn_section.db_account,
                empty52: String::new(),
                empty58: String::new(),
            }),
            client_uuid: Uuid::new_v4().to_string(),
            instance_uuid2: inst.uuid,
            flag23: 3,
        };

        let resp: pb::OpenResponse = self.unary("engine.session.SessionService/open", &open)?;
        if resp.session.is_empty() {
            return Err(anyhow!("open returned no session"));
        }
        Ok(OpenedSession {
            session: resp.session,
            engine: resp.engine,
            version: resp.version,
            db: resp.db,
            db_type: conn.db_type,
        })
    }
}

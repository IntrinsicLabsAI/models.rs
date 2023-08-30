use anyhow::{Context, Result};
use std::{ops::Deref, path::Path};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::router::models::types;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub model_type: String,
    pub runtime: String,
    pub description: String,
}

impl From<&Model> for types::RegisteredModel {
    // Deserialize the shit from our string representation
    fn from(value: &Model) -> Self {
        Self {
            id: uuid::Uuid::parse_str(&value.id).unwrap(),
            name: value.name.clone(),
            model_type: match value.model_type.deref() {
                "completion" => types::ModelType::Completion,
                _ => panic!("Invalid model_type {}", value.model_type),
            },
            runtime: match value.runtime.deref() {
                "ggml" => types::Runtime::Ggml,
                _ => panic!("Invalid runtime {}", value.runtime),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelVersion {
    pub model_id: String,
    pub version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportMetadata {
    pub model_id: String,
    pub model_version: String,
    pub source: String,
    pub imported_at: OffsetDateTime,
}

pub struct DB;

impl DB {
    pub fn open<T: AsRef<Path>>(db_path: T) -> Result<Connection> {
        Connection::open(db_path).context("failed to open connection")
    }
}

#[cfg(test)]
mod test {
    use crate::router::models::types::{ModelType, RegisteredModel, Runtime};

    use super::DB;

    #[test]
    pub fn test_simple() {
        let dir = tempdir::TempDir::new("db_test").unwrap();
        let mut db = DB::open(dir.path().join("test.db")).unwrap();

        // Have a single open TXN and ensure that we enforce the different contact versions here.
        {
            let tx = db.transaction().unwrap();

            {
                let mut stmt = tx
                    .prepare("insert into model values (?, ?, ?, ?, ?)")
                    .unwrap();
                stmt.insert([
                    "0937a774-0d14-46ad-923d-86ca6ce4a569",
                    "model1",
                    "completion",
                    "ggml",
                    "my first model",
                ])
                .unwrap();
                stmt.insert([
                    "beaa4b5a-ae17-4bc2-8af8-28168072cf5a",
                    "model2",
                    "completion",
                    "ggml",
                    "my second model",
                ])
                .unwrap();
            }

            tx.commit().unwrap();

            // Start a new transaction, and validate that we can read the data

            let tx = db.transaction().unwrap();

            {
                let mut stmt = tx
                    .prepare("select id, name, model_type, runtime, description from model")
                    .unwrap();

                let result = stmt
                    .query_map([], |row| {
                        let uid: &str = row.get_ref(0).unwrap().as_str().unwrap();
                        Ok(RegisteredModel {
                            id: uuid::Uuid::parse_str(uid).unwrap().to_owned(),
                            name: row.get::<usize, String>(1).unwrap().to_owned(),
                            model_type: ModelType::Completion,
                            runtime: Runtime::Ggml,
                        })
                    })
                    .unwrap();
                assert_eq!(result.count(), 2);
            }
        }
    }
}

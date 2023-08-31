use anyhow::Context;
use std::path::Path;
use tokio::sync::Mutex;

use rusqlite::{named_params, Connection};
use time::OffsetDateTime;

use crate::api_types::{self, ModelType, RegisterModelRequest, RegisteredModel, Runtime};
use crate::db_types::Model;

/// Handle to the [database connection](rusqlite::Connection)
pub struct DB {
    // The DB Handle owns the connection
    pub connection: Mutex<Connection>,
}

// Constructor
impl DB {
    pub fn open<T: AsRef<Path>>(db_path: T) -> anyhow::Result<Self> {
        // owned connection, will be accessed thru a mutex by all threads.
        // TODO(aduffy): use a threadlocal Connection pool to avoid the unnecessary locks and unlocks,
        // though they probably won't make much of a difference.
        let conn = Connection::open(db_path)?;

        // Enforce FK constraints on connection
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        Ok(Self {
            connection: Mutex::new(conn),
        })
    }
}

// General public methods for users of this type
impl DB {
    /// Register a new model version with the system
    pub async fn register_model(
        &self,
        request: &RegisterModelRequest,
    ) -> anyhow::Result<uuid::Uuid> {
        let model_id = uuid::Uuid::new_v4();
        let model_row = Model {
            id: model_id.to_string(),
            name: request.model.clone(),
            model_type: match request.model_type {
                ModelType::Completion => "completion".to_string(),
            },
            runtime: match request.runtime {
                Runtime::Ggml => "ggml".to_string(),
            },
            description: "".to_string(),
        };

        {
            let mut conn = self.connection.lock().await;
            let tx = conn.transaction().unwrap();

            // insert on model
            tx.prepare(
                "insert into model values (:id, :name, :model_type, :runtime, :description)",
            )?
            .insert(named_params! {
                ":id": &model_row.id,
                ":name": &model_row.name,
                ":model_type": &model_row.model_type,
                ":runtime": &model_row.runtime,
                ":description": &model_row.description,
            })
            .context("insert model table")?;

            // insert on model_version
            tx.prepare("insert into model_version values (:id, :version)")?
                .insert(named_params! { ":id": &model_row.id, ":version": &request.version.to_string() })
                .context("insert model_version table")?;

            // insert on import_metadata
            tx.prepare(
                "insert into import_metadata values (:id, :version, :source_json, :imported_at)",
            )?
            .insert(named_params! {
                ":id": &model_row.id,
                ":version": &request.version.to_string(),
                ":source_json": &serde_json::to_string(&request.import_metadata.source)?,
                ":imported_at": &request.import_metadata.imported_at,
            })
            .context("insert import_metadata table")?;

            // insert on model_params
            tx.prepare("insert into model_params values (:id, :version, :params)")?
                .insert(named_params! {
                    ":id": &model_row.id,
                    ":version": &request.version.to_string(),
                    ":params": &serde_json::to_string(&request.internal_params)?,
                })
                .context("insert model_params table")?;

            tx.commit().context("txn commit")?;
        }

        Ok(model_id)
    }

    pub async fn get_models(&self) -> anyhow::Result<Vec<RegisteredModel>> {
        let mut result_set: Vec<RegisteredModel> = Vec::new();
        {
            let mut conn = self.connection.lock().await;
            let tx = conn.transaction()?;

            let mut stmt =
                tx.prepare("select id, name, model_type, runtime, description from model")?;
            let rows = stmt.query_map([], |row| {
                Ok(Model {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    model_type: row.get(2)?,
                    runtime: row.get(3)?,
                    description: row.get(4)?,
                })
            })?;

            for row in rows {
                let row = &row?;
                let mut stmt = tx.prepare(r"
                        select model_version.version, import_metadata.source, import_metadata.imported_at
                        from model, model_version, model_params, import_metadata
                        where   model.id = model_version.id
                            and model_version.model_id = model_params.model_id
                            and model_version.version = model_params.model_version
                            and model_version.model_id = import_metadata.model_id
                            and model_version.version = import_metadata.model_version
                            and model_version.model_id = :id
                            order by model_version.version")?;
                let versions = stmt.query_map(&[(":id", &row.id)], |row| {
                    let (version, import_source, imported_at): (String, String, String) =
                        (row.get(0)?, row.get(1)?, row.get(2)?);
                    // Deserialize the import metadata from some shit here...etc.
                    let import_source: api_types::ImportSource =
                        serde_json::from_str(&import_source).unwrap();
                    let imported_at: OffsetDateTime = serde_json::from_str(&imported_at).unwrap();
                    Ok(api_types::ModelVersion {
                        version: semver::Version::parse(&version).unwrap(),
                        import_metadata: api_types::ImportMetadata {
                            source: import_source,
                            imported_at,
                        },
                    })
                })?;

                let mut model_versions: Vec<api_types::ModelVersion> = Vec::new();
                for version in versions {
                    model_versions.push(version?)
                }

                let model = RegisteredModel {
                    id: serde_json::from_str(&row.id)?,
                    name: row.name.to_string(),
                    model_type: match row.model_type.as_str() {
                        "completion" => api_types::ModelType::Completion,
                        _ => return Err(anyhow::Error::msg("unknown model_type")),
                    },
                    runtime: match row.runtime.as_str() {
                        "ggml" => api_types::Runtime::Ggml,
                        _ => return Err(anyhow::Error::msg("unknown runtime")),
                    },
                    versions: model_versions,
                };
                result_set.push(model);
            }
        }

        Ok(result_set)
    }

    pub async fn get_model_description(&self, model_name: &str) -> anyhow::Result<String> {
        // Model description for type here.
        let mut conn = self.connection.lock().await;
        let description: String = {
            let tx = conn.transaction()?;
            let mut stmt = tx.prepare("select description from model where name = :name")?;

            stmt.query_row(&[(":name", &model_name)], |row| row.get(0))?
        };

        Ok(description)
    }

    pub async fn update_model_description(
        &self,
        model_name: &str,
        new_desc: &str,
    ) -> anyhow::Result<()> {
        let mut conn = self.connection.lock().await;
        {
            let tx = conn.transaction()?;
            let mut stmt =
                tx.prepare("update model set description = :newdesc where name = :name")?;
            stmt.execute(&[(":newdesc", &new_desc), (":name", &model_name)])?;
        }

        Ok(())
    }
}

/// Root schema for the DB. Should be updated when we add/remove tables
/// NOTE: This should be merged more cleanly with the migration stuff.
pub static ROOT_SCHEMA: &'static str = r"
        create table if not exists model (
            id          text not null,
            name        text unique,
            model_type  text not null,
            runtime     text not null,
            description text not null,

            primary key (id)
        );

        create table if not exists model_version (
            model_id    text not null,
            version     text not null,

            primary key (model_id, version),
            foreign key (model_id) references model(id)
        );

        create table if not exists import_metadata (
            model_id        text not null,
            model_version   text not null,
            source text     not null,
            imported_at     datetime not null,

            primary key (model_id, model_version),
            foreign key (model_id) references model(id),
            foreign key (model_version) references model_version(version)
        );

        create table if not exists model_params (
            model_id        text not null,
            model_version   text not null,
            params          text not null,

            primary key (model_id, model_version),
            foreign key (model_id) references model(id),
            foreign key (model_version) references model_version(version)
        );

        create table if not exists saved_experiments (
            id              text not null,
            model_id        text not null,
            model_version   text not null,
            temperature     float not null,
            tokens          integer not null,
            prompt          text not null,
            output          text not null,
            created_at      datetime not null,

            primary key (id),
            foreign key (model_id) references model(id),
            foreign key (model_version) references model(version)
        );
";

#[cfg(test)]
mod test {
    use super::DB;
    use super::ROOT_SCHEMA;

    #[tokio::test]
    async fn test_simple() {
        let dir = tempdir::TempDir::new("db_test").unwrap();
        let db = DB::open(dir.path().join("test.db")).unwrap();
        // Seed the schema
        db.connection
            .lock()
            .await
            .execute_batch(ROOT_SCHEMA)
            .unwrap();

        // Run the actual test
        assert!(db.get_models().await.unwrap().is_empty());
        // // Execute our root migration to see the table schema accordingly.
        // db.execute_batch(ROOT_SCHEMA).unwrap();

        // // Have a single open TXN and ensure that we enforce the different contact versions here.
        // {
        //     {
        //         let tx = db.transaction().unwrap();
        //         let mut stmt = tx
        //             .prepare("insert into model values (?, ?, ?, ?, ?)")
        //             .unwrap();
        //         stmt.insert([
        //             "0937a774-0d14-46ad-923d-86ca6ce4a569",
        //             "model1",
        //             "completion",
        //             "ggml",
        //             "my first model",
        //         ])
        //         .unwrap();
        //         stmt.insert([
        //             "beaa4b5a-ae17-4bc2-8af8-28168072cf5a",
        //             "model2",
        //             "completion",
        //             "ggml",
        //             "my second model",
        //         ])
        //         .unwrap();

        //         tx.commit().unwrap();
        //     }

        //     // Start a new transaction, and validate that we can read the data

        //     let tx = db.transaction().unwrap();

        //     {
        //         let mut stmt = tx
        //             .prepare("select id, name, model_type, runtime, description from model")
        //             .unwrap();

        //         let result = stmt
        //             .query_map([], |row| {
        //                 let uid: &str = row.get_ref(0).unwrap().as_str().unwrap();
        //                 Ok(RegisteredModel {
        //                     id: uuid::Uuid::parse_str(uid).unwrap().to_owned(),
        //                     name: row.get::<usize, String>(1).unwrap().to_owned(),
        //                     model_type: ModelType::Completion,
        //                     runtime: Runtime::Ggml,
        //                 })
        //             })
        //             .unwrap();
        //         assert_eq!(result.count(), 2);
        //     }
        // }
    }
}

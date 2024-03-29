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
            let rows = stmt
                .query_map([], |row| {
                    Ok(Model {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        model_type: row.get(2)?,
                        runtime: row.get(3)?,
                        description: row.get(4)?,
                    })
                })
                .context("query model table")?;

            for row in rows {
                let row = &row.context("row was malformed")?;
                let mut stmt = tx.prepare(r"
                        select model_version.version, import_metadata.source, import_metadata.imported_at
                        from model, model_version, model_params, import_metadata
                        where   model.id = model_version.model_id
                            and model_version.model_id = model_params.model_id
                            and model_version.version = model_params.model_version
                            and model_version.model_id = import_metadata.model_id
                            and model_version.version = import_metadata.model_version
                            and model_version.model_id = :id
                            order by model_version.version").context("prepare join")?;

                let mut model_versions: Vec<api_types::ModelVersion> = Vec::new();
                let mut join_rows = stmt
                    .query(&[(":id", &row.id)])
                    .context("query join table")?;
                while let Some(join_row) = join_rows.next().transpose() {
                    let join_row = join_row.context("join row was malformed")?;
                    let (version, import_source, imported_at): (String, String, OffsetDateTime) =
                        (join_row.get(0)?, join_row.get(1)?, join_row.get(2)?);
                    let source: api_types::ImportSource =
                        serde_json::from_str(&import_source).context("parse import_source")?;
                    model_versions.push(api_types::ModelVersion {
                        version: semver::Version::parse(&version)?,
                        import_metadata: api_types::ImportMetadata {
                            imported_at,
                            source,
                        },
                    })
                }

                let model = RegisteredModel {
                    id: uuid::Uuid::parse_str(&row.id).context("failed to parse UUID")?,
                    name: row.name.to_string(),
                    model_type: match row.model_type.as_str() {
                        "completion" => api_types::ModelType::Completion,
                        _ => return Err(anyhow::anyhow!("unknown model_type {}", &row.model_type)),
                    },
                    runtime: match row.runtime.as_str() {
                        "ggml" => api_types::Runtime::Ggml,
                        _ => return Err(anyhow::anyhow!("unknown runtime {}", &row.runtime)),
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
            tx.prepare("update model set description = :newdesc where name = :name")?
                .execute(&[(":newdesc", &new_desc), (":name", &model_name)])?;

            tx.commit()?;
        }

        Ok(())
    }

    pub async fn rename_model(&self, model_name: &str, new_model_name: &str) -> anyhow::Result<()> {
        let mut conn = self.connection.lock().await;
        {
            let tx = conn.transaction()?;
            tx.prepare("update model set name = :new_model_name where name = :model_name")?
                .execute(
                    named_params! {":new_model_name": new_model_name, ":model_name": model_name},
                )?;
            tx.commit()?;
        }

        Ok(())
    }

    pub async fn delete_model(&self, model_name: &str) -> anyhow::Result<()> {
        let mut conn = self.connection.lock().await;
        {
            let tx = conn.transaction()?;
            let model_id: String = tx
                .prepare("select id from model where name = :name")?
                .query_row(
                    named_params! {":name": &model_name},
                    |r| -> Result<String, rusqlite::Error> { Ok(r.get(0)?) },
                )?;

            let delete_experiment =
                tx.prepare("delete from saved_experiments where model_id = :model_id")?;
            let delete_params =
                tx.prepare("delete from model_params where model_id = :model_id")?;
            let delete_import =
                tx.prepare("delete from import_metadata where model_id = :model_id")?;
            let delete_version =
                tx.prepare("delete from model_version where model_id = :model_id")?;
            let delete_model = tx.prepare("delete from model where id = :model_id")?;

            for mut stmt in vec![
                delete_experiment,
                delete_params,
                delete_import,
                delete_version,
                delete_model,
            ] {
                stmt.execute(named_params! {":model_id": &model_id})?;
            }

            tx.commit()?;
        }

        anyhow::Ok(())
    }

    pub async fn delete_model_version(
        &self,
        model_name: &str,
        version: &semver::Version,
    ) -> anyhow::Result<()> {
        let mut conn = self.connection.lock().await;
        {
            let tx = conn.transaction()?;
            let model_id: String = tx
                .prepare("select id from model where name = :name")?
                .query_row(
                    named_params! {":name": &model_name},
                    |r| -> Result<String, rusqlite::Error> { Ok(r.get(0)?) },
                )?;

            let delete_experiment =
                tx.prepare("delete from saved_experiments where model_id = :model_id and model_version = :version")?;
            let delete_params = tx.prepare(
                "delete from model_params where model_id = :model_id and model_version = :version",
            )?;
            let delete_import = tx.prepare(
                "delete from import_metadata where model_id = :model_id and model_version = :version",
            )?;
            let delete_version = tx.prepare(
                "delete from model_version where model_id = :model_id and version = :version",
            )?;

            for mut stmt in vec![
                delete_experiment,
                delete_params,
                delete_import,
                delete_version,
            ] {
                stmt.execute(
                    named_params! {":model_id": &model_id, ":version": &version.to_string()},
                )?;
            }

            tx.commit()?;
        }
        anyhow::Ok(())
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
    }
}

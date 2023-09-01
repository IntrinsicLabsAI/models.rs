use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{db::tables::DB, import::Importer};

pub struct ManagedModel {
    pub model: Mutex<llamacpp::Model>,
}

impl ManagedModel {
    pub fn new(model: llamacpp::Model) -> Self {
        ManagedModel {
            model: Mutex::new(model),
        }
    }
}

unsafe impl Send for ManagedModel {}
unsafe impl Sync for ManagedModel {}

pub struct ManagedConnection {
    pub conn: Mutex<Connection>,
}

impl ManagedConnection {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }
}

type ModelHandle = Arc<ManagedModel>;
type DBHandle = Arc<DB>;
type ImporterHandle = Arc<dyn Importer + Sync + Send>;

#[derive(Clone)]
pub struct AppState {
    pub model: ModelHandle,
    pub db: DBHandle,
    pub importer: ImporterHandle,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

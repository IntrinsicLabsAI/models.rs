use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

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
type ConnectionHandle = Arc<ManagedConnection>;

#[derive(Clone)]
pub struct AppState {
    pub model: ModelHandle,
    pub db: ConnectionHandle,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

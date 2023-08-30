use std::sync::Arc;

use rusqlite::Connection;

pub mod model;

#[derive(Clone)]
pub struct AppState {
    pub model: model::LockedModel,
    pub conn: Arc<Connection>,
}

unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

use std::{
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{routing::post, Router};
use env_logger::Env;
use llamacpp::Backend;

use model_server::{
    db::{
        self, manager::LinearMigrationManager, manager::MigrationManager, migration::V0, tables::DB,
    },
    router::generate,
    state::{model::LockedModel, AppState},
};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct EnvVars {
    #[serde(default = "default_listen_addr")]
    host: Ipv4Addr,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_db_path")]
    db_path: String,
}

fn default_listen_addr() -> Ipv4Addr {
    "127.0.0.1".parse().unwrap()
}

fn default_port() -> u16 {
    8000
}

fn default_db_path() -> String {
    String::from("prod.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    log::info!("Loading .env");
    let env: EnvVars = envy::from_env()?;
    log::info!("Environment: {:?}", &env);

    // TODO(aduffy): Replace model mutex with ModelPool
    let backend = Backend::new();
    let model = backend.load_model(&PathBuf::from("/Users/aduffy/Documents/llama2_gguf.bin"))?;

    // Generate a managed connection for the SQLite DB.
    let mut db = DB::open(env.db_path).context("failed to load DB")?;

    // Register migrations
    let mut migration_manager = LinearMigrationManager::new();
    migration_manager.register_migration(Arc::new(V0));

    // Execute migrations
    {
        let txn = db.transaction()?;
        migration_manager.initialize(&txn)?;

        let current_schema_version = migration_manager.get_current_schema_version(&txn)?;
        let target_schema_version = migration_manager.get_target_schema_version();
        migration_manager.upgrade_schema(&txn, current_schema_version, target_schema_version)?;
    }

    let state = AppState {
        model: LockedModel::new(model),
        conn: Arc::new(db),
    };

    let app = Router::new()
        .route("/complete", post(generate))
        .with_state(state);

    let listen_addr: SocketAddr = format!("{}:{}", &env.host, &env.port)
        .parse()
        .context("invalid bind addr")
        .unwrap();
    axum::Server::bind(&listen_addr)
        .serve(app.into_make_service())
        .await
        .context("failed to start axum server")
        .unwrap();

    Ok(())
}

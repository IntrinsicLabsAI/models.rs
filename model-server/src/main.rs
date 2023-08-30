use std::{
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use anyhow::{Context, Result};
use axum::{routing::post, Router};
use env_logger::Env;
use llamacpp::Backend;

use model_server::{
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
}

fn default_listen_addr() -> Ipv4Addr {
    "127.0.0.1".parse().unwrap()
}

fn default_port() -> u16 {
    8000
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

    let state = AppState {
        model: LockedModel::new(model),
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

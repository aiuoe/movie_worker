use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

use movie_worker::{config::Config, server, state::AppState, storage::StorageExt};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::from_env()?;
    cfg.validate()?;
    init_tracing();

    let storage = movie_worker::storage::build_from_env()?;
    tracing::info!(
        backend = ? storage.back_backend_str(),
        bucket = storage.bucket(),
        "storage provider ready"
    );
    storage.ping().await?;

    let state = AppState::new(cfg.clone(), storage);
    let app = server::router(state);

    tracing::info!("movie_worker listening on {} (api={})", cfg.addr, cfg.api_url);

    let listener = tokio::net::TcpListener::bind(&cfg.addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,movie_worker=debug"));
    fmt().with_env_filter(filter).with_target(false).init();
}
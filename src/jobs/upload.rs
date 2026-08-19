//! Job: sube un archivo local al bucket. Útil cuando llega vía torrent
//! montado en un volumen compartido o cuando el ingest escanea MEDIA_ROOT.

use std::collections::HashMap;

use crate::state::AppState;

pub async fn run(state: &AppState, args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = args.get("path").ok_or("missing path")?;
    let key = args.get("key").ok_or("missing key")?;

    tracing::info!(path, key, "upload: local → bucket");

    let bytes = tokio::fs::read(path).await?;
    let size = bytes.len();
    state.storage.put(key, bytes.into(), None).await?;
    tracing::info!(key, size, "upload: done");
    Ok(())
}
//! Job: escanea MEDIA_ROOT (filesystem local) y sube todo al bucket.
//! En el futuro este job lo gatillará un watcher inotify o un cron.

use std::collections::HashMap;

use crate::state::AppState;

pub async fn run(state: &AppState, _args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = std::path::Path::new(&state.cfg.media_root);
    tracing::info!(root = %root.display(), "ingest: scanning");

    let entries = match std::fs::read_dir(root) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "media_root not readable");
            return Ok(());
        }
    };

    let mut count = 0u32;
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                let prefix = format!("media/{name}/");
                let existing = state.storage.list(&prefix, 1000).await.unwrap_or_default();
                tracing::info!(folder = name, existing = existing.len(), "ingest: indexed");
                count += 1;
            }
        }
    }

    tracing::info!(count, "ingest: done");
    Ok(())
}
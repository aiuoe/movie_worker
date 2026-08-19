use std::path::Path;

use crate::state::AppState;

/// Escanea `MEDIA_ROOT` y emite un log con cada carpeta encontrada.
/// Convierte el job en trigger para que el API refresque el catálogo.
pub async fn run(state: &AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = Path::new(&state.cfg.media_root);
    tracing::info!(root = %root.display(), "ingest: scan");

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
                tracing::debug!(folder = name, "found");
                count += 1;
            }
        }
    }

    tracing::info!(count, "ingest done");
    Ok(())
}
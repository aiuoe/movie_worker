//! Job: importar un archivo local terminado por Radarr/Sonarr al bucket.
//!
//! Flujo típico (después de configurar Radarr/Sonarr → Connect → Custom Script):
//!   1. Radarr importa un torrent, lo renombra, lo mueve a /movies/Title (year)
//!   2. Radarr ejecuta un custom script con args {movie_file_path, movie_title, tmdbid}
//!   3. movie_worker recibe el webhook, sube el archivo a MinIO
//!      bajo `media/{tmdbid}-{slug}/es-ES/source.mp4`
//!   4. notifica al API que el media está listo
//!
//! También puede llamarse manualmente vía POST /jobs con `kind: "import"`.

use std::collections::HashMap;
use std::path::Path;

use crate::state::AppState;

pub async fn run(state: &AppState, args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = args.get("path").ok_or("missing path")?;
    let media_id = args.get("media_id").ok_or("missing media_id")?;
    let lang = args.get("lang").map(String::as_str).unwrap_or("es-ES");

    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("file not found: {path}").into());
    }
    let data = tokio::fs::read(p).await?;
    let size = data.len();

    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let ct = match ext {
        "mkv" => "video/x-matroska",
        "mp4" => "video/mp4",
        "avi" => "video/x-msvideo",
        _ => "application/octet-stream",
    };

    let key = format!("media/{media_id}/{lang}/source.{ext}");
    tracing::info!(path, key, size, "import: uploading to bucket");
    state.storage.put(&key, data.into(), Some(ct)).await?;
    tracing::info!(key, "import: done — file is now playable");
    Ok(())
}
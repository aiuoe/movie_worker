//! Job: descarga un archivo desde una URL pública y lo sube al bucket.
//!
//! Input (en JobRecord.notes o en una nota futura):
//!   - source_url: URL directa al .mp4/.mkv (HTTP, no torrent — para eso
//!     integrar qBittorrent+VPN como en el README).
//!   - media_id: id interno del título
//!   - lang: código BCP-47 del audio (ej: "es-ES", "en-US")
//!
//! Output:
//!   - s3://<bucket>/media/<media_id>/<lang>/source.<ext>
//!
//! Hoy descarga con reqwest y streama directo al bucket sin tocar disco
//! (memoria para archivos >2GB: usar stream → multipart upload; lo dejo
//! como TODO porque para el demo alcanza).

use std::collections::HashMap;

use crate::state::AppState;

const HTTP_TIMEOUT_SECS: u64 = 600;

pub async fn run(state: &AppState, args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let media_id = args
        .get("media_id")
        .ok_or("missing media_id")?;
    let source_url = args
        .get("source_url")
        .ok_or("missing source_url")?;
    let lang = args.get("lang").map(String::as_str).unwrap_or("es-ES");

    tracing::info!(media_id, source_url, lang, "download: fetching");

    // Detectar extensión del path URL.
    let ext = source_url
        .rsplit('.')
        .next()
        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("mp4");

    let key = format!("media/{media_id}/{lang}/source.{ext}");

    // Descarga con timeout generoso.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;
    let resp = client.get(source_url).send().await?.error_for_status()?;
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let bytes = resp.bytes().await?;
    let size = bytes.len();
    tracing::info!(key = %key, size, "download: uploading to bucket");

    state
        .storage
        .put(&key, bytes, content_type.as_deref())
        .await?;

    tracing::info!(key = %key, "download: done");
    Ok(())
}
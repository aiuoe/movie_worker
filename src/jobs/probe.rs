//! Job: ffprobe sobre un archivo del bucket. Sube metadata ligera
//! (duración, codec, resolución) de vuelta al log — en producción esto
//! actualizaría Postgres.

use std::collections::HashMap;

use crate::state::AppState;

pub async fn run(state: &AppState, args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let key = args
        .get("key")
        .map(String::as_str)
        .unwrap_or("");

    if key.is_empty() {
        return Err("missing key".into());
    }

    tracing::info!(key, "probe: ffprobe");

    // Bajamos a /tmp y probamos ahí. Para grandes archivos usar stream.
    let data = state.storage.get(key).await?;
    let tmp = std::env::temp_dir().join("probe-input");
    tokio::fs::write(&tmp, &data).await?;

    let out = std::process::Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "stream=codec_name,width,height,duration",
            "-show_entries", "format=duration,bit_rate",
            "-of", "json",
            tmp.to_str().unwrap(),
        ])
        .output();

    let _ = std::fs::remove_file(&tmp);

    match out {
        Ok(o) if o.status.success() => {
            tracing::info!(output = %String::from_utf8_lossy(&o.stdout), "probe ok");
            Ok(())
        }
        Ok(o) => Err(format!("ffprobe failed: {}", String::from_utf8_lossy(&o.stderr)).into()),
        Err(e) => {
            tracing::warn!(error = %e, "ffprobe not in PATH, skipping");
            Ok(())
        }
    }
}
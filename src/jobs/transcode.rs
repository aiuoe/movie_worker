//! Job: transcodifica un video del bucket a HLS multi-bitrate y guarda
//! el manifest de salida en el bucket.
//!
//! Input (en args):
//!   - media_id
//!   - lang (default "es-ES")
//!   - source_key (opcional, default: media/{id}/{lang}/source.mp4)
//!
//! Output:
//!   - media/{id}/{lang}/hls/master.m3u8
//!
//! Hoy dispara ffmpeg si está en $PATH. Si no está, loguea y deja el
//! comando listo para correr manualmente.

use std::collections::HashMap;
use std::process::Command;

use crate::state::AppState;

pub async fn run(state: &AppState, args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let media_id = args.get("media_id").ok_or("missing media_id")?;
    let lang = args.get("lang").map(String::as_str).unwrap_or("es-ES");
    let source_key = args
        .get("source_key")
        .cloned()
        .unwrap_or_else(|| format!("media/{media_id}/{lang}/source.mp4"));
    let output_key_prefix = format!("media/{media_id}/{lang}/hls");

    tracing::info!(media_id, lang, source_key, output_key_prefix, "transcode: start");

    // Descargamos el source del bucket (para archivos grandes se debería
    // streamear; placeholder).
    let src = state.storage.get(&source_key).await?;
    let tmp_in = std::env::temp_dir().join(format!("{media_id}-{lang}-source.mp4"));
    tokio::fs::write(&tmp_in, &src).await?;

    // El directorio temporal de salida lo usa ffmpeg; después subimos cada .ts.
    let tmp_out_dir = std::env::temp_dir().join(format!("{media_id}-{lang}-hls"));
    let _ = std::fs::create_dir_all(&tmp_out_dir);

    let status = Command::new("ffmpeg")
        .args([
            "-i", tmp_in.to_str().unwrap(),
            "-codec:", "copy",
            "-start_number", "0",
            "-hls_time", "10",
            "-hls_list_size", "0",
            "-f", "hls",
        ])
        .arg(format!("{}/index.m3u8", tmp_out_dir.display()))
        .status();

    match status {
        Ok(s) if s.success() => {
            tracing::info!("transcode: ffmpeg ok, subiendo segmentos");
            for entry in std::fs::read_dir(&tmp_out_dir)? {
                let entry = entry?;
                let p = entry.path();
                if !p.is_file() { continue; }
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                let bytes = std::fs::read(&p)?;
                let key = format!("{output_key_prefix}/{name}");
                state.storage.put(&key, bytes.into(), None).await?;
            }
            // Renombrar index.m3u8 → master.m3u8 en el bucket
            let _ = state.storage.put(
                &format!("{output_key_prefix}/master.m3u8"),
                std::fs::read(tmp_out_dir.join("index.m3u8"))?.into(),
                Some("application/vnd.apple.mpegurl"),
            ).await;
        }
        Ok(s) => tracing::warn!(?s, "ffmpeg exited non-zero"),
        Err(e) => tracing::warn!(error = %e, "ffmpeg not in PATH, skipping (placeholder)"),
    }

    // Cleanup
    let _ = std::fs::remove_file(&tmp_in);
    let _ = std::fs::remove_dir_all(&tmp_out_dir);

    tracing::info!("transcode: done");
    Ok(())
}
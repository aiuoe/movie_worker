use crate::state::AppState;

/// Transcode a HLS multi-bitrate. Hoy es un placeholder: si ffmpeg está
/// en el PATH disparamos la pipeline, si no, logueamos y dejamos listo
/// el comando para correr manualmente.
pub async fn run(_state: &AppState, media_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let input = format!("{}/{}/source.mkv", _state.cfg.media_root, media_id);
    let output_dir = format!("{}/{}/hls", _state.cfg.media_root, media_id);
    tracing::info!(input = %input, output = %output_dir, "transcode: ffmpeg → HLS");

    let _ = tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&output_dir).ok();
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-i", &input,
                "-codec:", "copy",
                "-start_number", "0",
                "-hls_time", "10",
                "-hls_list_size", "0",
                "-f", "hls",
            ])
            .arg(format!("{}/index.m3u8", output_dir))
            .status();

        match status {
            Ok(s) if s.success() => tracing::info!("transcode ok"),
            Ok(s) => tracing::warn!(?s, "ffmpeg exited non-zero"),
            Err(e) => tracing::warn!(error = %e, "ffmpeg not in PATH, skipping (placeholder)"),
        }
    })
    .await;

    Ok(())
}
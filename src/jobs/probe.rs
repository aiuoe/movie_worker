use std::process::Command;

use crate::state::AppState;

/// ffprobe sobre el archivo asociado al media_id. Por ahora asumimos
/// `${MEDIA_ROOT}/${media_id}/source.mkv`. Cuando se enchufe la metadata
/// real esto se vuelve una llamada HTTP al storage layer.
pub async fn run(_state: &AppState, media_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let path = format!("{}/{}/source.mkv", _state.cfg.media_root, media_id);
    tracing::info!(path = %path, "probe: ffprobe");

    let out = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "stream=codec_name,width,height,duration",
            "-of", "json",
            &path,
        ])
        .output();

    match out {
        Ok(o) if o.status.success() => {
            tracing::debug!(output = %String::from_utf8_lossy(&o.stdout), "probe ok");
            Ok(())
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            Err(format!("ffprobe failed: {stderr}").into())
        }
        Err(e) => {
            // ffprobe puede no estar instalado todavía — logueamos y seguimos.
            tracing::warn!(error = %e, "ffprobe not available, skipping");
            Ok(())
        }
    }
}
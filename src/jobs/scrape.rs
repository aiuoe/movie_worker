use crate::state::AppState;

/// Scraping de metadata. Cuando se enchufe TMDB/TVDB esto se convierte
/// en un HTTP client con caché. Por ahora valida que el media_id tenga
/// forma razonable para evitar jobs basura.
pub async fn run(_state: &AppState, media_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if media_id.len() < 3 {
        return Err(format!("invalid media_id: {media_id}").into());
    }
    tracing::info!(media_id, "scrape: placeholder (TMDB hook pending)");
    Ok(())
}
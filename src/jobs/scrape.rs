//! Job: scraping de metadata. Cuando se enchufe TMDB/TVDB esto se convierte
//! en un HTTP client con caché. Por ahora valida el input.

use std::collections::HashMap;

use crate::state::AppState;

pub async fn run(_state: &AppState, args: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let media_id = args.get("media_id").map(String::as_str).unwrap_or("");
    if media_id.len() < 3 {
        return Err(format!("invalid media_id: {media_id}").into());
    }
    tracing::info!(media_id, "scrape: placeholder (TMDB hook pending)");
    Ok(())
}
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use rusqlite::Connection;
use reqwest::Client;
use tauri::{State, Emitter};
use tokio::time::{sleep, Duration};

use crate::db::{self, Game};
use crate::error::AppError;
use crate::rawg::{self, RawgCandidate};
use crate::scanner;

/// Limite mensuelle du plan gratuit RAWG (documentation officielle).
pub const RAWG_FREE_MONTHLY_LIMIT: u64 = 20_000;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub http: Client,
    pub api_key: Mutex<Option<String>>,
    /// Compteur de requêtes effectuées vers l'API RAWG depuis le démarrage.
    /// Utilise AtomicU64 pour éviter le Mutex sur une valeur simple.
    pub api_requests_used: AtomicU64,
}

// ---------------------------------------------------------------------------
// Payload pour les événements de progression
// ---------------------------------------------------------------------------
#[derive(Clone, serde::Serialize)]
struct MetaProgressPayload {
    current: usize,
    total: usize,
    game_name: String,
    fetched: usize,
    needs_review: usize,
}

// ---------------------------------------------------------------------------
// Commandes
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_games(state: State<AppState>) -> Result<Vec<Game>, AppError> {
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    db::list_games(&conn)
}

#[tauri::command]
pub fn get_scan_dirs(state: State<AppState>) -> Result<Vec<String>, AppError> {
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    db::list_scan_dirs(&conn)
}

#[tauri::command]
pub fn add_scan_dir(path: String, state: State<AppState>) -> Result<(), AppError> {
    const MAX_DIRS: usize = 20;
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    let current = db::list_scan_dirs(&conn)?;
    if current.len() >= MAX_DIRS {
        return Err(AppError::TooManyDirectories { max: MAX_DIRS });
    }
    let validated = scanner::validate_directory(&path)?;
    db::add_scan_dir(&conn, &validated.to_string_lossy())
}

#[tauri::command]
pub fn remove_scan_dir(path: String, state: State<AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    db::remove_scan_dir(&conn, &path)
}

#[tauri::command]
pub fn scan_now(state: State<AppState>) -> Result<usize, AppError> {
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    let raw_dirs = db::list_scan_dirs(&conn)?;
    if raw_dirs.is_empty() { return Ok(0); }

    let dirs: Vec<std::path::PathBuf> = raw_dirs.iter()
        .filter_map(|p| scanner::validate_directory(p)
            .map_err(|e| log::warn!("Répertoire ignoré '{}': {}", p, e)).ok())
        .collect();

    let files = scanner::scan_directories(&dirs);
    let mut new_count = 0;

    for f in &files {
        let clean_name = rawg::clean_filename_for_search(&f.file_name);
        let game = db::Game {
            id: 0, iso_path: f.path.clone(), file_name: f.file_name.clone(),
            clean_name, rawg_id: None, rawg_name: None, description: None,
            cover_url: None, rating: None, metacritic: None, released: None,
            genres: None, platforms: None, metadata_fetched: false, needs_review: false,
        };

        let rows_before = conn.query_row(
            "SELECT COUNT(*) FROM games WHERE iso_path = ?1",
            rusqlite::params![f.path],
            |r| r.get::<_, i64>(0),
        ).unwrap_or(0);

        db::upsert_game(&conn, &game)?;
        if rows_before == 0 { new_count += 1; }
    }

    Ok(new_count)
}

/// Batch de métadonnées avec émission d'événements de progression.
/// Retourne (fetched, needs_review).
#[tauri::command]
pub async fn fetch_metadata_batch(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(usize, usize), AppError> {
    let api_key = {
        let g = state.api_key.lock().unwrap_or_else(|p| p.into_inner());
        g.as_ref().map(|k| k.clone()).ok_or(AppError::MissingApiKey)?
    };

    let games_to_fetch = {
        let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
        db::games_without_metadata(&conn)?
    };

    let total = games_to_fetch.len();
    if total == 0 { return Ok((0, 0)); }

    let mut fetched = 0usize;
    let mut needs_review = 0usize;

    for (idx, game) in games_to_fetch.iter().enumerate() {
        // Émettre la progression AVANT la requête
        let _ = app.emit("meta-progress", MetaProgressPayload {
            current: idx,
            total,
            game_name: game.clean_name.clone(),
            fetched,
            needs_review,
        });

        sleep(Duration::from_millis(rawg::RATE_LIMIT_DELAY_MS)).await;

        // Chaque appel à search_game_auto fait 1 ou 2 requêtes (search + detail)
        match rawg::search_game_auto(&state.http, &game.clean_name, &api_key).await {
            Ok(Some(meta)) => {
                state.api_requests_used.fetch_add(2, Ordering::Relaxed);
                let sim = rawg::similarity(&game.clean_name, &meta.name);
                let review = sim < rawg::AUTO_ACCEPT_THRESHOLD;
                let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
                match db::update_metadata(&conn, game.id, &meta, review) {
                    Ok(_) => { fetched += 1; if review { needs_review += 1; } }
                    Err(e) => log::error!("DB update game_id={}: {}", game.id, e),
                }
            }
            Ok(None) => {
                state.api_requests_used.fetch_add(1, Ordering::Relaxed);
                let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
                let _ = conn.execute(
                    "UPDATE games SET needs_review = 1 WHERE id = ?1",
                    rusqlite::params![game.id],
                );
                needs_review += 1;
            }
            Err(AppError::RateLimited) => {
                log::warn!("Rate limit RAWG, pause 5s");
                sleep(Duration::from_secs(5)).await;
            }
            Err(AppError::InvalidApiKey) => return Err(AppError::InvalidApiKey),
            Err(e) => log::error!("Erreur RAWG '{}': {}", game.clean_name, e),
        }
    }

    // Événement final
    let _ = app.emit("meta-progress", MetaProgressPayload {
        current: total,
        total,
        game_name: String::new(),
        fetched,
        needs_review,
    });

    Ok((fetched, needs_review))
}

/// Retourne les statistiques d'utilisation de l'API RAWG.
/// (requêtes_utilisées_depuis_démarrage, limite_mensuelle_gratuite)
#[tauri::command]
pub fn get_api_stats(state: State<AppState>) -> (u64, u64) {
    let used = state.api_requests_used.load(Ordering::Relaxed);
    (used, RAWG_FREE_MONTHLY_LIMIT)
}

#[tauri::command]
pub async fn search_candidates(
    game_id: i64,
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<RawgCandidate>, AppError> {
    let api_key = {
        let g = state.api_key.lock().unwrap_or_else(|p| p.into_inner());
        g.as_ref().map(|k| k.clone()).ok_or(AppError::MissingApiKey)?
    };
    let query = query.trim().to_string();
    if query.len() > 100 { return Err(AppError::QueryTooLong); }
    let _ = game_id;
    let result = rawg::search_candidates(&state.http, &query, &api_key).await;
    state.api_requests_used.fetch_add(1, Ordering::Relaxed);
    result
}

#[tauri::command]
pub async fn associate_game(
    game_id: i64,
    rawg_id: i64,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let api_key = {
        let g = state.api_key.lock().unwrap_or_else(|p| p.into_inner());
        g.as_ref().map(|k| k.clone()).ok_or(AppError::MissingApiKey)?
    };
    let meta = rawg::get_game_details(&state.http, rawg_id, &api_key).await?;
    state.api_requests_used.fetch_add(1, Ordering::Relaxed);
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    db::associate_game(&conn, game_id, &meta)
}

#[tauri::command]
pub fn reset_game_metadata(game_id: i64, state: State<AppState>) -> Result<(), AppError> {
    let conn = state.db.lock().unwrap_or_else(|p| p.into_inner());
    db::reset_metadata(&conn, game_id)
}

#[tauri::command]
pub fn set_api_key(
    key: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    if key.len() > 128 { return Err(AppError::MissingApiKey); }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err(AppError::MissingApiKey);
    }
    {
        let mut g = state.api_key.lock().unwrap_or_else(|p| p.into_inner());
        *g = if key.is_empty() { None } else { Some(key.clone()) };
    }
    use tauri_plugin_store::StoreExt;
    let store = app.store("settings.json").map_err(|e| {
        log::error!("Store inaccessible: {}", e);
        AppError::MissingApiKey
    })?;
    store.set("rawg_api_key", serde_json::Value::String(key));
    store.save().map_err(|e| {
        log::error!("Sauvegarde store échouée: {}", e);
        AppError::MissingApiKey
    })?;
    Ok(())
}

#[tauri::command]
pub fn has_api_key(state: State<AppState>) -> bool {
    state.api_key.lock().unwrap_or_else(|p| p.into_inner()).is_some()
}

#[tauri::command]
pub async fn test_api_key(key: String, state: State<'_, AppState>) -> Result<bool, AppError> {
    if key.is_empty() || key.len() > 128 { return Ok(false); }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Ok(false);
    }
    let result = state.http
        .get("https://api.rawg.io/api/games")
        .query(&[("key", key.as_str()), ("search", "tetris"),
                 ("page_size", "1"), ("search_precise", "true")])
        .send().await;
    state.api_requests_used.fetch_add(1, Ordering::Relaxed);
    match result {
        Ok(resp) => match resp.status().as_u16() {
            200 => Ok(true),
            401 | 403 => Ok(false),
            429 => Err(AppError::RateLimited),
            code => Err(AppError::UnexpectedApiResponse(code)),
        },
        Err(e) => Err(AppError::Network(e)),
    }
}

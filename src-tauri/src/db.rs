use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: i64,
    pub iso_path: String,
    pub file_name: String,
    pub clean_name: String,
    pub rawg_id: Option<i64>,
    pub rawg_name: Option<String>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub rating: Option<f64>,
    pub metacritic: Option<i64>,
    pub released: Option<String>,
    pub genres: Option<String>,
    pub platforms: Option<String>,
    pub metadata_fetched: bool,
    /// true = score de similarité insuffisant, l'utilisateur doit valider manuellement
    pub needs_review: bool,
}

pub fn open_db(db_path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        PRAGMA synchronous = NORMAL;
    ")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS games (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            iso_path         TEXT NOT NULL UNIQUE,
            file_name        TEXT NOT NULL,
            clean_name       TEXT NOT NULL,
            rawg_id          INTEGER,
            rawg_name        TEXT,
            description      TEXT,
            cover_url        TEXT,
            rating           REAL,
            metacritic       INTEGER,
            released         TEXT,
            genres           TEXT,
            platforms        TEXT,
            metadata_fetched INTEGER NOT NULL DEFAULT 0,
            needs_review     INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS scan_dirs (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE
        );
    ")?;

    // Migration idempotente : ajouter la colonne si elle n'existe pas encore
    // (pour les bases créées avant v0.1.8)
    let _ = conn.execute_batch(
        "ALTER TABLE games ADD COLUMN needs_review INTEGER NOT NULL DEFAULT 0;"
    );

    Ok(())
}

fn row_to_game(row: &rusqlite::Row) -> rusqlite::Result<Game> {
    Ok(Game {
        id:               row.get(0)?,
        iso_path:         row.get(1)?,
        file_name:        row.get(2)?,
        clean_name:       row.get(3)?,
        rawg_id:          row.get(4)?,
        rawg_name:        row.get(5)?,
        description:      row.get(6)?,
        cover_url:        row.get(7)?,
        rating:           row.get(8)?,
        metacritic:       row.get(9)?,
        released:         row.get(10)?,
        genres:           row.get(11)?,
        platforms:        row.get(12)?,
        metadata_fetched: row.get::<_, i64>(13).map(|v| v != 0)?,
        needs_review:     row.get::<_, i64>(14).map(|v| v != 0)?,
    })
}

const SELECT_COLS: &str = "
    id, iso_path, file_name, clean_name, rawg_id, rawg_name,
    description, cover_url, rating, metacritic, released, genres, platforms,
    metadata_fetched, needs_review";

pub fn upsert_game(conn: &Connection, game: &Game) -> Result<i64, AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO games (iso_path, file_name, clean_name, metadata_fetched, needs_review)
         VALUES (?1, ?2, ?3, 0, 0)",
        params![game.iso_path, game.file_name, game.clean_name],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM games WHERE iso_path = ?1",
        params![game.iso_path],
        |row| row.get(0),
    )?;
    Ok(id)
}

pub fn update_metadata(
    conn: &Connection,
    id: i64,
    meta: &crate::rawg::RawgGame,
    needs_review: bool,
) -> Result<(), AppError> {
    let genres = serde_json::to_string(&meta.genres)?;
    let platforms = serde_json::to_string(&meta.platforms)?;

    conn.execute(
        "UPDATE games SET
            rawg_id = ?1, rawg_name = ?2, description = ?3, cover_url = ?4,
            rating = ?5, metacritic = ?6, released = ?7, genres = ?8,
            platforms = ?9, metadata_fetched = 1, needs_review = ?10
         WHERE id = ?11",
        params![
            meta.id, meta.name, meta.description_raw, meta.background_image,
            meta.rating, meta.metacritic, meta.released, genres, platforms,
            needs_review as i64, id,
        ],
    )?;
    Ok(())
}

/// Association manuelle : lie un jeu à un ID RAWG choisi par l'utilisateur.
pub fn associate_game(
    conn: &Connection,
    game_id: i64,
    meta: &crate::rawg::RawgGame,
) -> Result<(), AppError> {
    // Association manuelle : needs_review = 0 car l'utilisateur a confirmé
    update_metadata(conn, game_id, meta, false)
}

/// Réinitialise les métadonnées d'un jeu pour permettre une nouvelle association.
pub fn reset_metadata(conn: &Connection, game_id: i64) -> Result<(), AppError> {
    conn.execute(
        "UPDATE games SET
            rawg_id = NULL, rawg_name = NULL, description = NULL, cover_url = NULL,
            rating = NULL, metacritic = NULL, released = NULL, genres = NULL,
            platforms = NULL, metadata_fetched = 0, needs_review = 0
         WHERE id = ?1",
        params![game_id],
    )?;
    Ok(())
}

pub fn list_games(conn: &Connection) -> Result<Vec<Game>, AppError> {
    let query = format!(
        "SELECT {} FROM games ORDER BY clean_name COLLATE NOCASE ASC",
        SELECT_COLS
    );
    let mut stmt = conn.prepare(&query)?;
    let games = stmt.query_map([], row_to_game)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(games)
}

pub fn games_without_metadata(conn: &Connection) -> Result<Vec<Game>, AppError> {
    let query = format!(
        "SELECT {} FROM games WHERE metadata_fetched = 0 ORDER BY id ASC",
        SELECT_COLS
    );
    let mut stmt = conn.prepare(&query)?;
    let games = stmt.query_map([], row_to_game)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(games)
}

pub fn add_scan_dir(conn: &Connection, path: &str) -> Result<(), AppError> {
    conn.execute("INSERT OR IGNORE INTO scan_dirs (path) VALUES (?1)", params![path])?;
    Ok(())
}

pub fn remove_scan_dir(conn: &Connection, path: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM scan_dirs WHERE path = ?1", params![path])?;
    Ok(())
}

pub fn list_scan_dirs(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT path FROM scan_dirs ORDER BY path ASC")?;
    let dirs = stmt.query_map([], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(dirs)
}

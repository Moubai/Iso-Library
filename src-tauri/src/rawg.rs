use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::Duration;
use crate::error::AppError;

// ---------------------------------------------------------------------------
// Constantes
// ---------------------------------------------------------------------------

const RAWG_BASE: &str = "https://api.rawg.io/api";
const MAX_QUERY_LEN: usize = 100;
const REQUEST_TIMEOUT_SECS: u64 = 10;
pub const RATE_LIMIT_DELAY_MS: u64 = 200;

/// Seuil de confiance en dessous duquel on force la révision manuelle.
pub const AUTO_ACCEPT_THRESHOLD: f64 = 0.85;

// ---------------------------------------------------------------------------
// Regex compilées une seule fois
// ---------------------------------------------------------------------------

static RE_PARENS: OnceLock<regex_lite::Regex> = OnceLock::new();
static RE_SCENE_SEPARATOR: OnceLock<regex_lite::Regex> = OnceLock::new();

fn re_parens() -> &'static regex_lite::Regex {
    RE_PARENS.get_or_init(|| regex_lite::Regex::new(r"\([^)]*\)|\[[^\]]*\]").unwrap())
}

fn re_scene_separator() -> &'static regex_lite::Regex {
    // Un point suivi de lettres minuscules ou d'un tiret suivi de lettres
    // signale généralement le début du tag de groupe scene : "Kingdom.Hearts.III-Mephisto"
    // On veut couper avant le "-<Groupe>" en fin de nom
    RE_SCENE_SEPARATOR.get_or_init(|| regex_lite::Regex::new(r"-[A-Za-z][A-Za-z0-9]{2,}$").unwrap())
}

// ---------------------------------------------------------------------------
// Structures de désérialisation RAWG
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawgGame {
    pub id: i64,
    pub name: String,
    pub released: Option<String>,
    pub background_image: Option<String>,
    pub rating: Option<f64>,
    pub metacritic: Option<i64>,
    pub description_raw: Option<String>,
    #[serde(default)]
    pub genres: Vec<RawgGenre>,
    #[serde(default)]
    pub platforms: Vec<RawgPlatformWrapper>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawgGenre {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawgPlatformWrapper {
    pub platform: RawgPlatform,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawgPlatform {
    pub name: String,
}

/// Candidat retourné au frontend pour sélection manuelle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawgCandidate {
    pub rawg_id: i64,
    pub name: String,
    pub released: Option<String>,
    pub background_image: Option<String>,
    pub rating: Option<f64>,
    /// Score de similarité Jaro-Winkler [0.0, 1.0] entre le nom cherché et ce candidat.
    pub similarity: f64,
    /// En dessous du seuil AUTO_ACCEPT_THRESHOLD, révision manuelle requise.
    pub needs_review: bool,
}

#[derive(Debug, Deserialize)]
pub struct RawgSearchResponse {
    pub results: Vec<RawgSearchResult>,
}

#[derive(Debug, Deserialize)]
pub struct RawgSearchResult {
    pub id: i64,
    pub name: String,
    pub released: Option<String>,
    pub background_image: Option<String>,
    pub rating: Option<f64>,
}

// ---------------------------------------------------------------------------
// Client HTTP
// ---------------------------------------------------------------------------

pub fn build_client() -> Result<Client, AppError> {
    let client = Client::builder()
        .https_only(true)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()?;
    Ok(client)
}

// ---------------------------------------------------------------------------
// Nettoyage du nom de fichier
//
// Logique par étapes pour les noms style "scene" :
//   "Kingdom.Hearts.III-Mephisto.iso"
//     1. Retirer extension         -> "Kingdom.Hearts.III-Mephisto"
//     2. Couper avant -Groupe      -> "Kingdom.Hearts.III"
//     3. Remplacer points par esp. -> "Kingdom Hearts III"
//     4. Retirer (USA) [NTSC-U]    -> inchangé
//     5. Normaliser espaces        -> "Kingdom Hearts III"
//
//   "Super.Mario.Galaxy.(USA).iso"
//     -> "Super Mario Galaxy"
//
//   "Gran_Turismo_4 [NTSC-U].iso"
//     -> "Gran Turismo 4"
// ---------------------------------------------------------------------------

pub fn clean_filename_for_search(filename: &str) -> String {
    // 1. Retirer l'extension (.iso, .ISO, etc.)
    let stem = filename
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(filename);

    // 2. Couper le tag de groupe scene : "-Mephisto", "-SKIDROW", "-CODEX", etc.
    //    Pattern : tiret suivi d'un mot de 3+ caractères en fin de chaîne.
    //    On s'assure que ce n'est pas juste un numéro romain ou "III", "IV"
    let stem = re_scene_separator().replace(stem, "");

    // 3. Remplacer les points par des espaces (noms style "Kingdom.Hearts.III")
    let cleaned = stem.replace('.', " ");

    // 4. Remplacer underscores et tirets restants par des espaces
    let cleaned = cleaned.replace('_', " ");
    let cleaned = cleaned.replace('-', " ");

    // 5. Retirer les tags entre parenthèses et crochets : (USA), [NTSC-U], (Rev 1)
    let cleaned = re_parens().replace_all(&cleaned, "");

    // 6. Normaliser les espaces multiples et trimmer
    let cleaned: String = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // 7. Tronquer
    cleaned.chars().take(MAX_QUERY_LEN).collect()
}

// ---------------------------------------------------------------------------
// Score de similarité
// ---------------------------------------------------------------------------

/// Calcule un score Jaro-Winkler insensible à la casse entre deux chaînes.
pub fn similarity(a: &str, b: &str) -> f64 {
    strsim::jaro_winkler(
        &a.to_lowercase(),
        &b.to_lowercase(),
    )
}

// ---------------------------------------------------------------------------
// Recherche de candidats
//
// Retourne jusqu'à 5 candidats triés par score de similarité décroissant.
// Le premier candidat est le meilleur match automatique.
// ---------------------------------------------------------------------------

pub async fn search_candidates(
    client: &Client,
    query: &str,
    api_key: &str,
) -> Result<Vec<RawgCandidate>, AppError> {
    if query.len() > MAX_QUERY_LEN {
        return Err(AppError::QueryTooLong);
    }
    if api_key.is_empty() {
        return Err(AppError::MissingApiKey);
    }

    log::debug!("RAWG candidates: query='{}'", query);

    let resp = client
        .get(format!("{}/games", RAWG_BASE))
        .query(&[
            ("key", api_key),
            ("search", query),
            ("page_size", "10"),   // On récupère 10, on garde les 5 meilleurs
            ("search_precise", "false"),
        ])
        .send()
        .await?;

    match resp.status().as_u16() {
        200 => {}
        401 => return Err(AppError::InvalidApiKey),
        429 => return Err(AppError::RateLimited),
        code => return Err(AppError::UnexpectedApiResponse(code)),
    }

    let data: RawgSearchResponse = resp.json().await?;

    let mut candidates: Vec<RawgCandidate> = data.results
        .into_iter()
        .map(|r| {
            let sim = similarity(query, &r.name);
            RawgCandidate {
                rawg_id: r.id,
                name: r.name,
                released: r.released,
                background_image: r.background_image,
                rating: r.rating,
                similarity: sim,
                needs_review: sim < AUTO_ACCEPT_THRESHOLD,
            }
        })
        .collect();

    // Trier par score décroissant
    candidates.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(5);

    Ok(candidates)
}

/// Récupère les détails complets d'un jeu par son ID RAWG.
pub async fn get_game_details(
    client: &Client,
    game_id: i64,
    api_key: &str,
) -> Result<RawgGame, AppError> {
    if api_key.is_empty() {
        return Err(AppError::MissingApiKey);
    }

    let resp = client
        .get(format!("{}/games/{}", RAWG_BASE, game_id))
        .query(&[("key", api_key)])
        .send()
        .await?;

    match resp.status().as_u16() {
        200 => {}
        401 => return Err(AppError::InvalidApiKey),
        429 => return Err(AppError::RateLimited),
        code => return Err(AppError::UnexpectedApiResponse(code)),
    }

    Ok(resp.json().await?)
}

/// Point d'entrée pour le batch automatique :
/// - Cherche les candidats
/// - Si le meilleur score >= seuil, retourne les détails complets automatiquement
/// - Sinon retourne None (le frontend devra proposer la sélection manuelle)
pub async fn search_game_auto(
    client: &Client,
    query: &str,
    api_key: &str,
) -> Result<Option<RawgGame>, AppError> {
    let candidates = search_candidates(client, query, api_key).await?;

    let best = match candidates.into_iter().next() {
        Some(c) => c,
        None => return Ok(None),
    };

    if best.needs_review {
        log::info!(
            "RAWG: '{}' -> meilleur match '{}' score={:.2} < seuil={:.2}, révision requise",
            query, best.name, best.similarity, AUTO_ACCEPT_THRESHOLD
        );
        return Ok(None);
    }

    get_game_details(client, best.rawg_id, api_key).await.map(Some)
}

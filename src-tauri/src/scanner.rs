use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use crate::error::AppError;

const MAX_DEPTH: usize = 8;

/// Taille minimale : 32 Ko pour ISO, 1 Ko pour archives (un zip vide fait ~22 octets)
const MIN_ISO_SIZE_BYTES: u64 = 32 * 1024;
const MIN_ARCHIVE_SIZE_BYTES: u64 = 1024;

const FORBIDDEN_PREFIXES_WINDOWS: &[&str] = &[
    r"C:\Windows",
    r"C:\Program Files",
    r"C:\Program Files (x86)",
    r"C:\ProgramData\Microsoft",
];

/// Extensions de fichiers supportées (insensible à la casse)
const SUPPORTED_EXTENSIONS: &[&str] = &["iso", "zip", "7z"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IsoFile {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    /// Extension du fichier (iso, zip, 7z)
    pub extension: String,
}

pub fn validate_directory(raw_path: &str) -> Result<PathBuf, AppError> {
    let path = Path::new(raw_path);
    if !path.is_absolute() {
        return Err(AppError::ForbiddenPath("Le chemin doit être absolu".to_string()));
    }
    let canonical = path.canonicalize().map_err(|_| AppError::InvalidDirectory)?;
    if !canonical.is_dir() {
        return Err(AppError::InvalidDirectory);
    }
    let canonical_str = canonical.to_string_lossy().to_lowercase();
    for forbidden in FORBIDDEN_PREFIXES_WINDOWS {
        if canonical_str.starts_with(&forbidden.to_lowercase()) {
            return Err(AppError::ForbiddenPath(forbidden.to_string()));
        }
    }
    Ok(canonical)
}

pub fn scan_directory(dir: &Path) -> Vec<IsoFile> {
    let mut results = Vec::new();

    let walker = WalkDir::new(dir)
        .max_depth(MAX_DEPTH)
        .follow_links(false)
        .into_iter();

    for entry_result in walker {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) => { log::warn!("Scanner: entrée ignorée: {}", err); continue; }
        };

        if !entry.file_type().is_file() { continue; }

        let path = entry.path();
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) { continue; }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(err) => {
                log::warn!("Scanner: métadonnées inaccessibles {:?}: {}", path, err);
                continue;
            }
        };

        // Taille minimale selon le type
        let min_size = if ext == "iso" { MIN_ISO_SIZE_BYTES } else { MIN_ARCHIVE_SIZE_BYTES };
        if metadata.len() < min_size {
            log::debug!("Scanner: {:?} ignoré (trop petit: {} octets)", path, metadata.len());
            continue;
        }

        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if file_name.is_empty() { continue; }

        let canonical_path = match path.canonicalize() {
            Ok(p) => p.to_string_lossy().to_string(),
            Err(_) => path.to_string_lossy().to_string(),
        };

        results.push(IsoFile {
            path: canonical_path,
            file_name,
            size_bytes: metadata.len(),
            extension: ext,
        });
    }

    results
}

pub fn scan_directories(dirs: &[PathBuf]) -> Vec<IsoFile> {
    let mut seen = std::collections::HashSet::new();
    let mut all = Vec::new();
    for dir in dirs {
        for f in scan_directory(dir) {
            if seen.insert(f.path.clone()) { all.push(f); }
        }
    }
    all.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    all
}

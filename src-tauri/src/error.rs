use thiserror::Error;

/// Erreurs internes de l'application.
///
/// SECURITE : Ces variantes ne doivent PAS exposer de données sensibles dans leur
/// message (ex: pas de clé API dans les strings d'erreur). Le `Display` de chaque
/// variante est ce qui remonte au frontend via serde, donc tout message est
/// potentiellement visible par l'utilisateur - c'est intentionnel ici.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Erreur base de données: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Erreur réseau: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Erreur de sérialisation: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Répertoire invalide ou inaccessible")]
    InvalidDirectory,

    #[error("Chemin interdit: {0}")]
    ForbiddenPath(String),

    #[error("Clé API manquante - veuillez la configurer dans les paramètres")]
    MissingApiKey,

    #[error("Clé API invalide (réponse 401 de RAWG)")]
    InvalidApiKey,

    #[error("Limite de taux RAWG atteinte, réessayez dans quelques secondes")]
    RateLimited,

    #[error("Réponse RAWG inattendue: code {0}")]
    UnexpectedApiResponse(u16),

    #[error("Trop de répertoires configurés (max {max})")]
    TooManyDirectories { max: usize },

    #[error("Nom de jeu trop long pour la recherche")]
    QueryTooLong,
}

/// Implémentation requise par Tauri pour sérialiser les erreurs vers le frontend.
/// On sérialise uniquement le message Display, jamais le debug interne.
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

# ISO Library

Médiathèque de jeux vidéo au format ISO, avec métadonnées RAWG.
Interface style Plex/Jellyfin. Développé avec Tauri 2 (Rust + WebView natif).

## Prérequis pour compiler

- **Rust** : https://rustup.rs (rustc >= 1.77)
- **Node.js** >= 18 : https://nodejs.org
- **Tauri CLI v2** et dépendances Windows :
  - Visual Studio Build Tools 2022 (composant "Développement Desktop C++")
  - WebView2 Runtime (inclus par défaut sur Windows 10/11)

```powershell
# Installer les dépendances Node
npm install

# Lancer en mode développement
npm run dev

# Construire l'installeur
npm run build
```

## Premier démarrage

1. Aller dans **Paramètres**
2. Entrer votre clé API RAWG (gratuite sur rawg.io/apidocs)
   - La clé est testée contre l'API avant d'être sauvegardée
3. Ajouter un ou plusieurs répertoires contenant vos fichiers .iso
4. Revenir dans **Bibliothèque** et cliquer **Scanner**
5. Cliquer **Récupérer métadonnées** pour enrichir la bibliothèque

## Architecture

```
iso-library/
├── src/                    Frontend (HTML/CSS/JS vanilla)
│   ├── index.html
│   ├── style.css
│   └── main.js
├── src-tauri/              Backend Rust
│   ├── src/
│   │   ├── main.rs         Point d'entrée Tauri
│   │   ├── lib.rs
│   │   ├── commands.rs     Commandes IPC exposées au frontend
│   │   ├── db.rs           Couche SQLite (requêtes paramétrées)
│   │   ├── rawg.rs         Client API RAWG
│   │   └── scanner.rs      Scan filesystem .iso
│   ├── Cargo.toml
│   └── tauri.conf.json     Configuration + CSP
├── SECURITY_AUDIT_1.md
└── README.md
```

## Données locales

- **Base de données** : `%APPDATA%\iso-library\library.db` (SQLite)
- **Paramètres / Clé API** : `%APPDATA%\iso-library\settings.json`

## Sécurité

Voir `SECURITY_AUDIT_1.md` pour l'audit complet.

Points principaux :
- Clé API jamais loggée ni retournée au frontend
- Toutes les requêtes SQL utilisent des paramètres liés (pas d'interpolation)
- Validation et canonicalisation des chemins côté Rust avant usage
- `follow_links(false)` dans le scanner (pas de traversal par symlinks)
- CSP stricte : `img-src` limité à `media.rawg.io`
- TLS uniquement (`https_only(true)` dans reqwest)

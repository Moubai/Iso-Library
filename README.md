# ISO Library
Vibe codé avec Claude.ai opus.
Médiathèque de jeux vidéo au format ISO, avec métadonnées RAWG.
Interface style Plex/Jellyfin. Développé avec Tauri 2 (Rust + WebView natif).

## Prérequis

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

## TO DO
Remplacer Rawg par igdb.com

## folder

```
iso-library/
│   package.json
│   README.md
│
├───docs
│       CHANGELOG.md
│       SECURITY_AUDIT_1.md
│
├───Release
│       ISO Library_0.1.8_x64-setup.exe
│
└───src-tauri
    │   build.rs
    │   Cargo.toml
    │   tauri.conf.json
    │
    ├───capabilities
    │       default.json
    │
    ├───frontend
    │       index.html
    │       main.js
    │       style.css
    │
    ├───icons
    │       128x128.png
    │       128x128@2x.png
    │       32x32.png
    │       icon.icns
    │       icon.ico
    │       icon.png
    │
    └───src
            commands.rs
            db.rs
            error.rs
            lib.rs
            main.rs
            rawg.rs
            scanner.rs
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

# Changelog

Toutes les modifications notables de ce projet sont documentées dans ce fichier.

Format basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.0.0/).
Versionnage selon [Semantic Versioning](https://semver.org/lang/fr/).

---

## [0.1.8] - 2026-05-26

### Ajouté

- **Nettoyage intelligent des noms de fichiers** : les noms style "scene" sont maintenant
  correctement traités. `Kingdom.Hearts.III-Mephisto.iso` donne `Kingdom Hearts III`
  (le suffixe `-Mephisto` est détecté comme tag de groupe et supprimé).
  Les points sont remplacés par des espaces avant de retirer les tags.
- **Système de candidats RAWG avec score de similarité** : au lieu de prendre le premier
  résultat aveuglément, l'API retourne jusqu'à 5 candidats scorés par Jaro-Winkler.
  Seuil d'acceptation automatique : 85%. En dessous, le jeu est marqué `needs_review`.
- **Badge "À réviser"** sur les cartes et dans la sidebar (compteur dédié).
- **Modale d'association manuelle** : bouton "Associer manuellement" / "Réassocier"
  dans la modale de détail. L'utilisateur peut taper un titre libre, voir les candidats
  avec leur score, et choisir.
- **Dissociation** : bouton "Effacer métadonnées" pour réinitialiser un jeu et
  permettre une nouvelle association.
- **Scroll infini** : la grille charge 15 jeux initialement puis 15 de plus à chaque
  fois que le bas de page est atteint (Intersection Observer).
- **5 colonnes maximum** dans la grille (CSS `max-width` + `auto-fill` avec `minmax(160px, 1fr)`).
- **Titre affiché sous chaque miniature** avec troncature à 2 lignes.
- **Placeholder enrichi** : quand aucune cover n'est disponible, le nom du jeu est
  affiché dans la zone cover.
- **Nouveau crate `strsim 0.11.1`** pour le calcul de similarité Jaro-Winkler.
- **Nouvelles commandes Tauri** : `search_candidates`, `associate_game`, `reset_game_metadata`.
- **Migration SQLite idempotente** : colonne `needs_review` ajoutée aux bases existantes
  via `ALTER TABLE ... ADD COLUMN`.

### Modifié

- `fetch_metadata_batch` retourne maintenant un tuple `(fetched, needs_review)`.
- La status bar affiche un avertissement orange quand des jeux nécessitent une révision.

---

## [0.1.7] - 2026-05-25

### Corrigé

- **`src-tauri/tauri.conf.json`** : ajout de `"withGlobalTauri": true` et du
  `"label": "main"` sur la fenêtre. Sans `withGlobalTauri`, Tauri n'injecte pas
  `window.__TAURI__` dans le WebView et tous les appels IPC échouent silencieusement.
- **`src-tauri/capabilities/default.json`** : création du fichier de capabilities
  (nouveau en Tauri 2). Sans ce fichier, aucune commande IPC n'est autorisée.
  Permissions accordées : `core:default` et `dialog:default`.
- **`src-tauri/frontend/main.js`** : réécriture complète pour utiliser
  `window.__TAURI__.core.invoke()` et `window.__TAURI__.dialog.open()` au lieu des
  imports ES. Sans bundler, WebView2 ne peut pas résoudre les bare module specifiers.
- **`src-tauri/frontend/index.html`** : suppression de `type="module"` sur la balise
  `<script>`.

### Note

Le binaire produit par `npm run dev` est un build de développement non autonome.
Pour un `.exe` standalone : `npm run build` (installeur dans `src-tauri/target/release/bundle/`).

---

## [0.1.6] - 2026-05-25

### Corrigé

- **`src-tauri/tauri.conf.json`** : suppression de `"dialog": {"all": true}` dans la
  section `plugins`. `tauri-plugin-dialog` 2.x ne déclare pas de schéma de configuration
  dans `tauri.conf.json` — la présence d'un objet causait l'erreur
  *"invalid type: map, expected unit"*. La section `plugins` est désormais vide `{}`.

---

## [0.1.5] - 2026-05-25

### Corrigé

- **`src-tauri/tauri.conf.json`** : suppression de `"store": {}` dans la section
  `plugins`. `tauri-plugin-store` 2.x ne déclare pas de schéma de configuration dans
  `tauri.conf.json` — la présence d'un objet vide causait l'erreur
  *"invalid type: map, expected unit"* au démarrage.

---

## [0.1.4] - 2026-05-25

### Corrigé

- **`src-tauri/Cargo.toml`** : ajout du feature `query` dans reqwest. Dans reqwest 0.13,
  la méthode `.query()` sur `RequestBuilder` n'est plus incluse par défaut — elle dépend
  du feature `query` (qui active `serde_urlencoded`). Son absence causait trois erreurs
  `E0599: no method named 'query' found`.
- **`src-tauri/src/commands.rs`** : suppression du macro `lock!` inutilisé (warning
  `unused_macros`). Les appels `Mutex::lock()` utilisent directement
  `.unwrap_or_else(|p| p.into_inner())` en ligne.

---

## [0.1.3] - 2026-05-25

### Corrigé

- **`src-tauri/Cargo.toml`** : ajout de la section `[package.metadata.bundle]` requise
  par `tauri-build` pour générer les métadonnées de bundle Windows.
- **`src-tauri/icons/`** : ajout des icônes manquantes (`32x32.png`, `128x128.png`,
  `128x128@2x.png`, `icon.ico`, `icon.icns`, `icon.png`). L'icône `.ico` est obligatoire
  pour la génération du fichier de ressources Windows lors de la compilation.

---

## [0.1.2] - 2026-05-25

### Corrigé

- **`src-tauri/src/main.rs`** : suppression des imports `use iso_library_lib::*` remplacés
  par des déclarations `mod` locales (`mod commands`, `mod db`, etc.). Le binaire Tauri
  ne lie pas automatiquement la `[lib]` du même crate.
- **`src-tauri/Cargo.toml`** : suppression des sections `[lib]` et `[[bin]]` superflues.
  Cargo trouve `src/main.rs` sans déclaration explicite.
- **`src-tauri/tauri.conf.json`** : `frontendDist` corrigé de `"../src"` vers
  `"./frontend"`. Le chemin est désormais relatif à `src-tauri/` sans ambiguïté.
- **Frontend déplacé** dans `src-tauri/frontend/` pour correspondre au nouveau chemin.

---

## [0.1.1] - 2026-05-25

### Corrigé

- **`src-tauri/Cargo.toml`** : feature reqwest `rustls-tls` renommée en `rustls`
  (breaking change reqwest 0.13 — l'ancien nom n'existe plus).

---

## [0.1.0] - 2026-05-24

### Ajouté

- Structure initiale du projet Tauri 2 (Rust + WebView natif Windows).
- **Scanner filesystem** : parcours récursif de répertoires à la recherche de fichiers `.iso`.
  - Profondeur maximale de 8 niveaux.
  - Filtre taille minimale 32 Ko (ignore les stubs/placeholders).
  - Déduplication par chemin canonique sur plusieurs répertoires.
  - Blocage des répertoires système Windows (`C:\Windows`, `C:\Program Files`, etc.).
  - Pas de suivi des liens symboliques (`follow_links = false`).
- **Intégration API RAWG** : récupération des métadonnées de jeux (nom, cover, description,
  note, Metacritic, genres, plateformes, année de sortie).
  - Nettoyage automatique des noms de fichiers avant recherche.
  - Vérification de la clé API contre RAWG avant sauvegarde.
  - Délai de politesse de 200 ms entre chaque requête (rate limiting).
  - Pause automatique de 5 s en cas de réponse 429.
- **Base de données SQLite** : persistance locale, mode WAL, clés étrangères, requêtes
  paramétrées exclusivement.
- **Interface médiathèque** : grille de covers style Plex/Jellyfin, recherche en temps réel,
  modal détail.
- **Paramètres** : saisie et sauvegarde de la clé API RAWG, gestion des répertoires de scan.
- **Sécurité** : CSP stricte, clé API jamais exposée au frontend, TLS uniquement,
  `panic = abort`, Mutex poison-safe, regex via `OnceLock`.
- **Audit de sécurité #1**.

### Dépendances principales

| Crate / Package           | Version | Rôle                       |
|---------------------------|---------|----------------------------|
| tauri                     | 2.11.2  | Framework desktop          |
| tauri-plugin-store        | 2.4.3   | Persistance paramètres     |
| tauri-plugin-dialog       | 2.7.1   | Sélecteur de répertoire    |
| rusqlite (bundled)        | 0.39.0  | Base de données SQLite     |
| reqwest (rustls)          | 0.13.3  | Client HTTP                |
| walkdir                   | 2.5.0   | Scan récursif filesystem   |
| serde / serde_json        | 1.0.228 | Sérialisation              |
| tokio                     | 1.52.3  | Runtime async              |
| thiserror                 | 2.0.18  | Gestion d'erreurs typée    |
| regex-lite                | 0.1.9   | Nettoyage noms de fichiers |
| strsim                    | 0.11.1  | Similarité Jaro-Winkler    |
| @tauri-apps/api           | 2.5.0   | Bridge IPC frontend        |
| @tauri-apps/plugin-dialog | 2.2.1   | Dialogue fichier frontend  |

---

## À venir

### Prévu pour [0.2.0]

- Remplacement de `tauri-plugin-store` par `keyring` pour stocker la clé API
  dans le Windows Credential Manager (chiffrement OS).
- Scrape automatique au démarrage (configurable).
- Audit de sécurité #2 post-keyring.

### Prévu pour [0.3.0]

- Filtres par genre, plateforme, année, note.
- Export de la bibliothèque en JSON/CSV.
- Tri de la grille (alphabétique, note, année).

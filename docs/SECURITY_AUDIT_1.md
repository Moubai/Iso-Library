# Audit de sécurité #1 - ISO Library
Date : prototype initial

## Surface d'attaque identifiée

### 1. Entrées utilisateur

| Source | Vecteur | Mitigation en place |
|--------|---------|---------------------|
| Champ clé API | Injection / clé malicieuse | Validation regex `[a-zA-Z0-9\-_]{1..128}` côté Rust avant usage |
| Chemin répertoire | Path traversal, symlinks | `canonicalize()` + blocage chemins système + `follow_links(false)` |
| Nom fichier ISO | XSS dans le DOM | `textContent` exclusivement, jamais `innerHTML` |
| Résultat RAWG | XSS via données externes | `textContent` + validation URL image (whitelist domaine) |

### 2. Réseau

| Risque | Mitigation |
|--------|-----------|
| MITM / downgrade HTTP | `https_only(true)` dans reqwest |
| Redirections malicieuses | `Policy::limited(3)` |
| Blocage indéfini | `timeout(10s)` |
| Fuite clé API dans logs | La clé n'est jamais incluse dans les messages de log |
| Fuite clé API en mémoire frontend | La clé n'est jamais retournée au frontend (`has_api_key` retourne un bool) |

### 3. Stockage

| Risque | Mitigation |
|--------|-----------|
| Injection SQL | 100% paramètres liés (`params![]`), zéro interpolation de chaînes |
| Corruption DB | WAL mode + foreign_keys ON |
| Stockage clé API | tauri-plugin-store (AppData/Roaming) - pas en clair dans le code |

### 4. Interface Tauri (IPC)

| Risque | Mitigation |
|--------|-----------|
| Commandes non autorisées | Whitelist explicite dans `generate_handler![]` |
| Injection via IPC | Désérialisation Serde typée, pas d'eval |
| Chargement d'images arbitraires | CSP `img-src` limitée à `media.rawg.io` dans tauri.conf.json ET meta HTML |

### 5. Scan filesystem

| Risque | Mitigation |
|--------|-----------|
| Symlink traversal | `follow_links(false)` dans walkdir |
| Scan excessif (ReDoS / performance) | `max_depth(8)`, max 20 répertoires |
| Faux positifs (fichiers minuscules) | Filtre taille minimale 32 Ko |

---

## Problèmes identifiés et corrections à apporter

### CRITIQUE

Aucun problème critique identifié dans le prototype.

### MOYEN

**M1 : Stockage clé API**
- Situation actuelle : tauri-plugin-store stocke en JSON dans AppData.
  Le fichier `settings.json` est lisible par l'utilisateur courant (normal)
  mais aussi par tout processus tournant sous le même utilisateur.
- Recommandation : Pour v1.0, utiliser le crate `keyring` qui délègue au
  Windows Credential Manager (stockage chiffré par l'OS).
- Statut : Accepté pour le prototype, à corriger avant release.

**M2 : Clé API validée mais pas vérifiée à la sauvegarde**
- Situation actuelle : `set_api_key` valide le format mais ne teste pas
  la clé contre l'API RAWG.
- Recommandation : Ajouter une commande `test_api_key` qui fait une requête
  de test et retourne le résultat sans stocker si invalide.
- Statut : À implémenter.

### FAIBLE

**F1 : Description RAWG peut contenir du HTML**
- Situation actuelle : RAWG retourne `description_raw` (texte brut) et
  `description` (HTML). On utilise `description_raw` stocké en DB.
  Affiché via `textContent` donc sûr même si HTML.
- Statut : Mitigé.

**F2 : Rate limiting RAWG côté client seulement**
- 200ms de délai + pause 5s sur 429, mais pas de token bucket.
- Statut : Acceptable pour le prototype.

**F3 : Pas de vérification d'intégrité des données DB**
- Si la DB est corrompue manuellement, les données peuvent être incorrectes.
- Statut : Acceptable pour une app locale.

---

## Vecteurs non applicables

- **Authentification multi-utilisateurs** : app desktop mono-utilisateur.
- **CSRF** : pas de serveur web.
- **Server-side injection** : pas de serveur.
- **Élévation de privilèges** : l'app ne demande aucun privilège administrateur.

---

## Prochains audits prévus

- Audit #2 : Après implémentation de `test_api_key` et gestion keyring.
- Audit #3 : Avant release, revue complète des dépendances (`cargo audit`).

/**
 * main.js — ISO Library v0.1.9
 * window.__TAURI__ (withGlobalTauri: true)
 * Sécurité : textContent exclusivement, sanitizeImageUrl pour les images.
 */

const ALLOWED_IMAGE_DOMAINS = ['media.rawg.io'];
function sanitizeImageUrl(url) {
  if (!url) return null;
  try {
    const p = new URL(url);
    if (p.protocol !== 'https:') return null;
    if (!ALLOWED_IMAGE_DOMAINS.some(d => p.hostname === d)) return null;
    return p.href;
  } catch { return null; }
}

function invoke(cmd, args) { return window.__TAURI__.core.invoke(cmd, args); }
function openDialog(options) { return window.__TAURI__.dialog.open(options); }
function listen(event, cb) { return window.__TAURI__.event.listen(event, cb); }

// ---------------------------------------------------------------------------
// État
// ---------------------------------------------------------------------------
let allGames = [];
let filtered = [];
let currentSearch = '';
let filterReview = false;
let sortMode = 'alpha-asc';
let currentGameForAssociate = null;
let progressUnlisten = null;

const PAGE_SIZE = 15;
let renderedCount = 0;
let isLoadingMore = false;

// ---------------------------------------------------------------------------
// DOM helpers
// ---------------------------------------------------------------------------
function el(id) { return document.getElementById(id); }

function showStatus(msg, type = 'info') {
  const bar = el('status-bar');
  bar.textContent = msg;
  bar.className = 'status-bar' +
    (type === 'error' ? ' error' : type === 'success' ? ' success' : type === 'warning' ? ' warning' : '');
  bar.classList.remove('hidden');
  if (type === 'success') setTimeout(() => bar.classList.add('hidden'), 4000);
}

// ---------------------------------------------------------------------------
// Filtre et tri
// ---------------------------------------------------------------------------
function applyFilterAndSort() {
  const q = currentSearch.toLowerCase();
  filtered = allGames.filter(g => {
    if (filterReview && !g.needs_review) return false;
    if (!q) return true;
    return (g.rawg_name || g.clean_name || g.file_name).toLowerCase().includes(q);
  });

  filtered.sort((a, b) => {
    switch (sortMode) {
      case 'alpha-asc':
        return (a.rawg_name || a.clean_name).localeCompare(b.rawg_name || b.clean_name, 'fr', {sensitivity:'base'});
      case 'alpha-desc':
        return (b.rawg_name || b.clean_name).localeCompare(a.rawg_name || a.clean_name, 'fr', {sensitivity:'base'});
      case 'year-desc':
        return (b.released || '').localeCompare(a.released || '');
      case 'year-asc':
        return (a.released || '').localeCompare(b.released || '');
      case 'rating-desc':
        return (b.rating || 0) - (a.rating || 0);
      case 'rating-asc':
        return (a.rating || 0) - (b.rating || 0);
      default: return 0;
    }
  });
}

// ---------------------------------------------------------------------------
// Grille
// ---------------------------------------------------------------------------
function renderGrid(reset = true) {
  const grid = el('game-grid');
  const empty = el('empty-state');
  const container = el('game-grid-container');

  if (reset) {
    while (grid.firstChild) grid.removeChild(grid.firstChild);
    renderedCount = 0;
  }

  if (filtered.length === 0) {
    empty.classList.remove('hidden');
    container.classList.add('hidden');
    return;
  }
  empty.classList.add('hidden');
  container.classList.remove('hidden');

  const end = Math.min(renderedCount + PAGE_SIZE, filtered.length);
  for (let i = renderedCount; i < end; i++) {
    grid.appendChild(buildGameCard(filtered[i]));
  }
  renderedCount = end;
  el('load-more-indicator').classList.toggle('hidden', renderedCount >= filtered.length);
}

function buildGameCard(game) {
  const card = document.createElement('div');
  card.className = 'game-card' + (game.needs_review ? ' needs-review' : '');
  card.setAttribute('role', 'button');
  card.setAttribute('tabindex', '0');
  card.dataset.id = String(game.id);

  if (game.needs_review) {
    const badge = document.createElement('div');
    badge.className = 'review-badge';
    badge.textContent = 'À réviser';
    card.appendChild(badge);
  }

  const wrap = document.createElement('div');
  wrap.className = 'game-card-cover-wrap';
  const coverUrl = sanitizeImageUrl(game.cover_url);
  if (coverUrl) {
    const img = document.createElement('img');
    img.className = 'game-card-cover';
    img.src = coverUrl; img.alt = ''; img.loading = 'lazy';
    img.onerror = () => { wrap.removeChild(img); wrap.appendChild(buildPlaceholder(game)); };
    wrap.appendChild(img);
  } else {
    wrap.appendChild(buildPlaceholder(game));
  }
  card.appendChild(wrap);

  const body = document.createElement('div');
  body.className = 'game-card-body';
  const title = document.createElement('div');
  title.className = 'game-card-title';
  title.textContent = game.rawg_name || game.clean_name || game.file_name;
  body.appendChild(title);

  const row = document.createElement('div');
  row.style.cssText = 'display:flex;gap:6px;align-items:center;';
  if (game.rating) {
    const r = document.createElement('span');
    r.className = 'game-card-rating';
    r.textContent = '\u2605 ' + game.rating.toFixed(1);
    row.appendChild(r);
  }
  if (game.released) {
    const y = document.createElement('span');
    y.className = 'game-card-year';
    y.textContent = String(game.released).slice(0, 4);
    row.appendChild(y);
  }
  if (row.firstChild) body.appendChild(row);
  card.appendChild(body);

  card.addEventListener('click', () => openGameModal(game));
  card.addEventListener('keydown', e => { if (e.key === 'Enter' || e.key === ' ') openGameModal(game); });
  return card;
}

function buildPlaceholder(game) {
  const div = document.createElement('div');
  div.className = 'game-card-placeholder';
  const ico = document.createElement('div'); ico.textContent = '\uD83D\uDCBF';
  const lbl = document.createElement('span'); lbl.textContent = game.clean_name || game.file_name;
  div.appendChild(ico); div.appendChild(lbl);
  return div;
}

function initScrollObserver() {
  const sentinel = el('scroll-sentinel');
  new IntersectionObserver(entries => {
    if (entries[0].isIntersecting && !isLoadingMore && renderedCount < filtered.length) {
      isLoadingMore = true;
      renderGrid(false);
      isLoadingMore = false;
    }
  }, { rootMargin: '100px' }).observe(sentinel);
}

// ---------------------------------------------------------------------------
// Modal détail
// ---------------------------------------------------------------------------
function openGameModal(game) {
  const coverUrl = sanitizeImageUrl(game.cover_url);
  const coverImg = el('modal-cover');
  if (coverUrl) { coverImg.src = coverUrl; coverImg.alt = ''; coverImg.style.display = ''; }
  else { coverImg.style.display = 'none'; }

  el('modal-title').textContent = game.rawg_name || game.clean_name || game.file_name;
  el('modal-desc').textContent = game.description || 'Aucune description disponible.';
  el('modal-path').textContent = game.iso_path;

  const metaDiv = el('modal-meta');
  while (metaDiv.firstChild) metaDiv.removeChild(metaDiv.firstChild);
  if (game.needs_review) metaDiv.appendChild(makeBadge('À réviser', 'review'));
  if (game.released) metaDiv.appendChild(makeBadge(String(game.released).slice(0, 4), 'year'));
  if (game.rating) metaDiv.appendChild(makeBadge('\u2605 ' + game.rating.toFixed(1), 'rating'));
  if (game.metacritic) metaDiv.appendChild(makeBadge('Metacritic: ' + game.metacritic));
  if (game.genres) {
    try { JSON.parse(game.genres).slice(0, 4).forEach(g => metaDiv.appendChild(makeBadge(typeof g === 'object' ? g.name : String(g)))); } catch {}
  }

  const actionsDiv = el('modal-actions');
  while (actionsDiv.firstChild) actionsDiv.removeChild(actionsDiv.firstChild);

  const btnAssoc = document.createElement('button');
  btnAssoc.className = 'btn btn-review btn-sm';
  btnAssoc.textContent = game.metadata_fetched ? 'Réassocier' : 'Associer manuellement';
  btnAssoc.addEventListener('click', () => openAssociateModal(game));
  actionsDiv.appendChild(btnAssoc);

  if (game.metadata_fetched) {
    const btnReset = document.createElement('button');
    btnReset.className = 'btn btn-danger btn-sm';
    btnReset.textContent = 'Effacer métadonnées';
    btnReset.addEventListener('click', async () => {
      try {
        await invoke('reset_game_metadata', { gameId: game.id });
        closeGameModal(); await loadGames();
        showStatus('Métadonnées effacées.', 'success');
      } catch (err) { showStatus('Erreur: ' + String(err), 'error'); }
    });
    actionsDiv.appendChild(btnReset);
  }

  el('game-modal').classList.remove('hidden');
  el('modal-close').focus();
}

function makeBadge(text, cls) {
  const b = document.createElement('span');
  b.className = 'meta-badge' + (cls ? ' ' + cls : '');
  b.textContent = text;
  return b;
}

function closeGameModal() { el('game-modal').classList.add('hidden'); }

// ---------------------------------------------------------------------------
// Modal association
// ---------------------------------------------------------------------------
function openAssociateModal(game) {
  currentGameForAssociate = game;
  el('associate-title').textContent = 'Associer : ' + (game.clean_name || game.file_name);
  // Pré-remplir avec le nom nettoyé (sans points ni tags)
  el('associate-search').value = game.clean_name || '';
  el('associate-hint').textContent = 'Nom nettoyé : "' + (game.clean_name || game.file_name) + '"';
  showAssociateResults([]);
  el('associate-modal').classList.remove('hidden');
  el('associate-search').focus();
  doAssociateSearch();
}

function closeAssociateModal() {
  el('associate-modal').classList.add('hidden');
  currentGameForAssociate = null;
}

async function doAssociateSearch() {
  if (!currentGameForAssociate) return;
  const raw = el('associate-search').value;
  // Nettoyer la query saisie par l'utilisateur : remplacer les points par espaces,
  // supprimer les tags (USA), [NTSC-U], couper après le tiret-groupe
  const query = cleanQueryInput(raw);
  if (!query) return;

  // Mettre à jour le champ avec la valeur nettoyée pour montrer ce qui sera cherché
  el('associate-search').value = query;

  const resultsDiv = el('associate-results');
  while (resultsDiv.firstChild) resultsDiv.removeChild(resultsDiv.firstChild);
  const loading = document.createElement('p');
  loading.className = 'associate-loading';
  loading.textContent = 'Recherche en cours...';
  resultsDiv.appendChild(loading);

  try {
    const candidates = await invoke('search_candidates', { gameId: currentGameForAssociate.id, query });
    showAssociateResults(candidates);
    refreshApiCounter();
  } catch (err) {
    while (resultsDiv.firstChild) resultsDiv.removeChild(resultsDiv.firstChild);
    const p = document.createElement('p');
    p.className = 'associate-empty';
    p.textContent = 'Erreur: ' + String(err);
    resultsDiv.appendChild(p);
  }
}

/**
 * Nettoyage côté frontend de la saisie utilisateur pour l'association manuelle.
 * Miroir simplifié de clean_filename_for_search() côté Rust.
 * Permet à l'utilisateur de coller un nom de fichier brut et d'obtenir un titre propre.
 */
function cleanQueryInput(raw) {
  let s = raw.trim();
  // Retirer extension si présente
  s = s.replace(/\.(iso|zip|7z)$/i, '');
  // Couper le tag groupe scene : -Mephisto, -CODEX, etc. en fin de chaîne
  s = s.replace(/-[A-Za-z][A-Za-z0-9]{2,}$/, '');
  // Remplacer points et underscores par espaces
  s = s.replace(/\./g, ' ').replace(/_/g, ' ').replace(/-/g, ' ');
  // Retirer contenu entre parenthèses et crochets
  s = s.replace(/\([^)]*\)/g, '').replace(/\[[^\]]*\]/g, '');
  // Normaliser espaces
  s = s.replace(/\s+/g, ' ').trim();
  return s;
}

function showAssociateResults(candidates) {
  const resultsDiv = el('associate-results');
  while (resultsDiv.firstChild) resultsDiv.removeChild(resultsDiv.firstChild);
  if (!candidates || candidates.length === 0) {
    const p = document.createElement('p');
    p.className = 'associate-empty';
    p.textContent = 'Aucun résultat. Essayez un autre titre.';
    resultsDiv.appendChild(p);
    return;
  }
  candidates.forEach((c, idx) => {
    const card = document.createElement('div');
    card.className = 'candidate-card' + (idx === 0 ? ' best' : '');
    const thumbUrl = sanitizeImageUrl(c.background_image);
    if (thumbUrl) {
      const img = document.createElement('img');
      img.className = 'candidate-thumb'; img.src = thumbUrl; img.alt = ''; img.loading = 'lazy';
      img.onerror = () => img.replaceWith(makePlaceholderThumb());
      card.appendChild(img);
    } else { card.appendChild(makePlaceholderThumb()); }

    const info = document.createElement('div'); info.className = 'candidate-info';
    const name = document.createElement('div'); name.className = 'candidate-name'; name.textContent = c.name;
    info.appendChild(name);
    if (c.released) {
      const year = document.createElement('div'); year.className = 'candidate-year';
      year.textContent = String(c.released).slice(0, 4) + (c.rating ? '  \u2605 ' + c.rating.toFixed(1) : '');
      info.appendChild(year);
    }
    card.appendChild(info);

    const pct = Math.round(c.similarity * 100);
    const score = document.createElement('div');
    score.className = 'candidate-score ' + (pct >= 85 ? 'high' : pct >= 60 ? 'med' : 'low');
    score.textContent = pct + '%';
    card.appendChild(score);

    card.addEventListener('click', () => confirmAssociate(c));
    resultsDiv.appendChild(card);
  });
}

function makePlaceholderThumb() {
  const div = document.createElement('div');
  div.className = 'candidate-thumb-placeholder';
  div.textContent = '\uD83D\uDCBF';
  return div;
}

async function confirmAssociate(candidate) {
  if (!currentGameForAssociate) return;
  try {
    await invoke('associate_game', { gameId: currentGameForAssociate.id, rawgId: candidate.rawg_id });
    closeAssociateModal(); closeGameModal();
    await loadGames(); refreshApiCounter();
    showStatus('"' + candidate.name + '" associé.', 'success');
  } catch (err) { showStatus('Erreur association: ' + String(err), 'error'); }
}

// ---------------------------------------------------------------------------
// Compteur API
// ---------------------------------------------------------------------------
async function refreshApiCounter() {
  try {
    const [used, limit] = await invoke('get_api_stats');
    const pct = Math.min(100, (used / limit) * 100);
    el('api-counter-text').textContent = used.toLocaleString('fr') + ' / ' + limit.toLocaleString('fr');
    const fill = el('api-fill');
    fill.style.width = pct + '%';
    fill.className = 'api-counter-fill' + (pct >= 90 ? ' danger' : pct >= 70 ? ' warn' : '');
  } catch {}
}

// ---------------------------------------------------------------------------
// Navigation
// ---------------------------------------------------------------------------
function switchView(name) {
  document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
  document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
  const view = el('view-' + name); if (view) view.classList.add('active');
  const btn = document.querySelector('.nav-btn[data-view="' + name + '"]'); if (btn) btn.classList.add('active');
}

// ---------------------------------------------------------------------------
// Données
// ---------------------------------------------------------------------------
async function loadGames() {
  try {
    allGames = await invoke('get_games');
    applyFilterAndSort();
    renderGrid(true);
    updateStats();
  } catch (err) { showStatus('Erreur chargement: ' + String(err), 'error'); }
}

function updateStats() {
  el('stat-total').textContent = String(allGames.length);
  el('stat-meta').textContent = String(allGames.filter(g => g.metadata_fetched).length);
  const rev = allGames.filter(g => g.needs_review).length;
  el('stat-review').textContent = String(rev);
}

async function loadScanDirs() {
  try {
    const dirs = await invoke('get_scan_dirs');
    const list = el('dir-list');
    while (list.firstChild) list.removeChild(list.firstChild);
    dirs.forEach(dir => {
      const li = document.createElement('li'); li.className = 'dir-item';
      const span = document.createElement('span'); span.className = 'dir-path'; span.textContent = dir;
      const btn = document.createElement('button'); btn.className = 'btn btn-danger'; btn.textContent = 'Supprimer';
      btn.addEventListener('click', async () => { await invoke('remove_scan_dir', { path: dir }); await loadScanDirs(); });
      li.appendChild(span); li.appendChild(btn); list.appendChild(li);
    });
  } catch (err) { console.error('Erreur répertoires:', err); }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------
async function doScan() {
  const btn = el('btn-scan');
  btn.disabled = true;
  showStatus('Scan en cours...');
  try {
    const count = await invoke('scan_now');
    showStatus('Scan terminé — ' + count + ' nouveau(x) fichier(s).', 'success');
    await loadGames();
  } catch (err) { showStatus('Erreur scan: ' + String(err), 'error'); }
  finally { btn.disabled = false; }
}

async function doFetchMetadata() {
  const btn = el('btn-fetch-meta');
  btn.disabled = true;

  // Afficher la barre de progression
  const progressWrap = el('progress-wrap');
  progressWrap.classList.remove('hidden');
  el('progress-fill').style.width = '0%';
  el('progress-label').textContent = 'Initialisation...';
  el('progress-count').textContent = '';

  // Écouter les événements de progression
  if (progressUnlisten) { progressUnlisten(); progressUnlisten = null; }
  progressUnlisten = await listen('meta-progress', event => {
    const { current, total, game_name, fetched, needs_review } = event.payload;
    const pct = total > 0 ? Math.round((current / total) * 100) : 0;
    el('progress-fill').style.width = pct + '%';
    el('progress-count').textContent = current + ' / ' + total;
    if (game_name) {
      el('progress-label').textContent = 'Recherche : ' + game_name;
    } else {
      el('progress-label').textContent = 'Terminé — ' + fetched + ' mis à jour, ' + needs_review + ' à réviser';
    }
    refreshApiCounter();
  });

  try {
    const [fetched, review] = await invoke('fetch_metadata_batch');
    let msg = fetched + ' jeu(x) mis à jour.';
    if (review > 0) msg += ' ' + review + ' nécessite(nt) une révision manuelle.';
    showStatus(msg, review > 0 ? 'warning' : 'success');
    await loadGames();
    refreshApiCounter();
  } catch (err) {
    showStatus('Erreur métadonnées: ' + String(err), 'error');
  } finally {
    btn.disabled = false;
    // Garder la barre visible 2s puis la masquer
    setTimeout(() => progressWrap.classList.add('hidden'), 2000);
    if (progressUnlisten) { progressUnlisten(); progressUnlisten = null; }
  }
}

async function saveApiKey() {
  const input = el('api-key-input');
  const key = input.value.trim();
  const indicator = el('api-key-status');
  const btn = el('btn-save-key');
  if (!key) { indicator.textContent = 'Veuillez entrer une clé.'; indicator.className = 'status-indicator err'; return; }
  btn.disabled = true;
  indicator.textContent = 'Vérification...'; indicator.className = 'status-indicator';
  try {
    const valid = await invoke('test_api_key', { key });
    if (!valid) { indicator.textContent = 'Clé invalide ou refusée par RAWG.'; indicator.className = 'status-indicator err'; return; }
    await invoke('set_api_key', { key });
    input.value = '';
    indicator.textContent = 'Clé vérifiée et enregistrée.'; indicator.className = 'status-indicator ok';
    refreshApiCounter();
  } catch (err) {
    indicator.textContent = 'Erreur: ' + String(err); indicator.className = 'status-indicator err';
  } finally { btn.disabled = false; }
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------
window.addEventListener('DOMContentLoaded', async () => {
  // Navigation
  document.querySelectorAll('.nav-btn').forEach(btn => {
    btn.addEventListener('click', () => switchView(btn.dataset.view));
  });

  // Filtre "À réviser"
  el('filter-review').addEventListener('click', () => {
    filterReview = !filterReview;
    el('filter-review').classList.toggle('active', filterReview);
    applyFilterAndSort(); renderGrid(true);
  });

  // Tri
  el('sort-select').addEventListener('change', e => {
    sortMode = e.target.value;
    applyFilterAndSort(); renderGrid(true);
  });

  // Recherche texte
  el('search-input').addEventListener('input', e => {
    currentSearch = e.target.value;
    applyFilterAndSort(); renderGrid(true);
  });

  // Bibliothèque
  el('btn-scan').addEventListener('click', doScan);
  el('btn-fetch-meta').addEventListener('click', doFetchMetadata);

  // Modal jeu
  el('modal-close').addEventListener('click', closeGameModal);
  el('game-modal').addEventListener('click', e => { if (e.target === e.currentTarget) closeGameModal(); });

  // Modal association
  el('associate-close').addEventListener('click', closeAssociateModal);
  el('associate-modal').addEventListener('click', e => { if (e.target === e.currentTarget) closeAssociateModal(); });
  el('btn-associate-search').addEventListener('click', doAssociateSearch);
  el('associate-search').addEventListener('keydown', e => { if (e.key === 'Enter') doAssociateSearch(); });

  // Escape
  document.addEventListener('keydown', e => {
    if (e.key === 'Escape') {
      if (!el('associate-modal').classList.contains('hidden')) closeAssociateModal();
      else closeGameModal();
    }
  });

  // Paramètres
  el('btn-save-key').addEventListener('click', saveApiKey);
  el('api-key-input').addEventListener('keydown', e => { if (e.key === 'Enter') saveApiKey(); });
  el('btn-add-dir').addEventListener('click', async () => {
    try {
      const selected = await openDialog({ directory: true, multiple: false, title: 'Sélectionner un répertoire' });
      if (!selected) return;
      await invoke('add_scan_dir', { path: selected });
      await loadScanDirs();
    } catch (err) { showStatus('Erreur: ' + String(err), 'error'); }
  });

  // Scroll infini
  initScrollObserver();

  // État initial
  try {
    const hasKey = await invoke('has_api_key');
    if (hasKey) {
      el('api-key-status').textContent = 'Une clé API est configurée.';
      el('api-key-status').className = 'status-indicator ok';
    }
  } catch {}

  await Promise.all([loadGames(), loadScanDirs(), refreshApiCounter()]);
});

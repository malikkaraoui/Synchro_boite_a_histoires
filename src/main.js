// LuniiSync V2 — UI deux colonnes
const { invoke } = window.__TAURI__.core;
const { open }   = window.__TAURI__.dialog;
const { listen } = window.__TAURI__.event;

const APP_VERSION = "2.1.3";
// URL de vérification des mises à jour (GitHub releases API)

// ── État ──────────────────────────────────────────────────────────────────────
let deviceMount   = null;
let deviceId      = null;
let appSettings   = { devices: {}, lastAudioFolder: null, theme: "auto" };
let deviceStories = [];       // LuniiStoryEntry[]
let audioFiles    = [];       // AudioFile[]
let pendingIds     = new Set(); // story_id en attente de sync
let pendingDeletes = new Set(); // short_uuid en attente de suppression
let syncing        = false;

// ── DOM ───────────────────────────────────────────────────────────────────────
const $deviceBadge     = document.getElementById("device-badge");
const $devicePath      = document.getElementById("device-path");
const $storageWrap     = document.getElementById("storage-bar-wrap");
const $storageFill     = document.getElementById("storage-bar-fill");
const $storageUsed     = document.getElementById("storage-used-label");
const $storageFree     = document.getElementById("storage-free-label");
const $deviceList      = document.getElementById("device-list");
const $deviceEmpty     = document.getElementById("device-empty");
const $deviceHeader    = document.getElementById("device-list-header");
const $deviceCount     = document.getElementById("device-story-count");
const $pendingLabel    = document.getElementById("pending-count");
const $syncBtn         = document.getElementById("sync-btn");
const $pickBtn         = document.getElementById("pick-btn");
const $folderPath      = document.getElementById("folder-path");
const $folderList      = document.getElementById("folder-list");
const $folderEmpty     = document.getElementById("folder-empty");
const $folderHeader    = document.getElementById("folder-list-header");
const $folderCount     = document.getElementById("folder-file-count");
const $addAllBtn       = document.getElementById("add-all-btn");
const $removeAllBtn    = document.getElementById("remove-all-btn");
const $syncStatusBar   = document.getElementById("sync-status-bar");
const $syncStatusText  = document.getElementById("sync-status-text");
const $syncStatusCount = document.getElementById("sync-status-count");
const $syncToast       = document.getElementById("sync-toast");
const $ejectBtn        = document.getElementById("eject-btn");
const $repairBtn       = document.getElementById("repair-btn");
const $deviceFwLabel   = document.getElementById("device-fw-label");
const $namingModal     = document.getElementById("naming-modal");
const $namingInput     = document.getElementById("naming-input");
const $namingError     = document.getElementById("naming-error");
const $namingSave      = document.getElementById("naming-save");
const $namingSkip      = document.getElementById("naming-skip");
const $logDrawer       = document.getElementById("log-drawer");
const $logOutput       = document.getElementById("log-output");
const $logToggle       = document.getElementById("log-toggle");

// ── Formatage ─────────────────────────────────────────────────────────────────
function fmtSize(bytes) {
  if (bytes === 0) return "—";
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} Ko`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} Mo`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} Go`;
}

function initial(name) {
  return (name || "?")[0].toUpperCase();
}

// ── Splash screen ─────────────────────────────────────────────────────────────
async function runSplash() {
  const $splash   = document.getElementById("splash");
  const $bar      = document.getElementById("splash-progress-fill");
  const $label    = document.getElementById("splash-update-label");
  const $splashV  = document.getElementById("splash-version");
  const $settingsV = document.getElementById("settings-version");

  if ($splashV)  $splashV.textContent  = `v${APP_VERSION}`;
  if ($settingsV) $settingsV.textContent = APP_VERSION;

  // Progression fake + vrai check update en parallèle (minimum 5s d'affichage)
  const splashStart = Date.now();
  let pct = 0;
  const tick = setInterval(() => {
    pct = Math.min(pct + 3, 85);
    $bar.style.width = pct + "%";
  }, 120);

  try {
    const latest = await invoke("check_for_update");
    if (latest && latest !== APP_VERSION) {
      $label.textContent = `Nouvelle version disponible : v${latest} — cliquez pour installer`;
      $label.style.color = "#f0a32a";
      $label.style.cursor = "pointer";
      $label.onclick = () => {
        $label.textContent = "Téléchargement en cours…";
        $label.style.cursor = "default";
        $label.onclick = null;
        invoke("download_and_install_update").catch(e => {
          $label.textContent = `Erreur : ${e}`;
          $label.style.color = "#e55";
        });
      };
    } else {
      $label.textContent = "Application à jour ✓";
      $label.style.color = "#00957f";
    }
  } catch {
    $label.textContent = "Pas de connexion — vérification ignorée";
  }

  clearInterval(tick);
  $bar.style.width = "100%";
  const elapsed = Date.now() - splashStart;
  const remaining = Math.max(0, 5000 - elapsed);
  await new Promise(r => setTimeout(r, remaining + 400));
  $splash.classList.add("fade-out");
  await new Promise(r => setTimeout(r, 400));
  $splash.remove();
}

// ── Dark mode ─────────────────────────────────────────────────────────────────
function applyTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme || "auto");
}

// ── Settings & naming ─────────────────────────────────────────────────────────

function deviceName(id) {
  return (appSettings.devices[id]?.name) || null;
}

function renderDeviceName(id) {
  const name = deviceName(id);
  const $title = document.getElementById("panel-device-title");
  if (name) {
    $title.textContent = name;
    $title.classList.add("device-named");
  } else {
    $title.textContent = "Boîte à histoires";
    $title.classList.remove("device-named");
  }
  // Supprimer l'ancien label s'il existe (migration)
  document.getElementById("device-name-label")?.remove();
}

function showNamingModal(id) {
  $namingInput.value = "";
  $namingError.classList.add("hidden");
  $namingModal.classList.remove("hidden");
  setTimeout(() => $namingInput.focus(), 50);

  $namingSave.onclick = async () => {
    const name = $namingInput.value.trim();
    if (!isValidName(name)) {
      $namingError.classList.remove("hidden");
      return;
    }
    await invoke("save_device_name", { deviceId: id, name });
    appSettings.devices[id] = { name };
    renderDeviceName(id);
    $namingModal.classList.add("hidden");
  };
  $namingSkip.onclick = () => $namingModal.classList.add("hidden");
  $namingInput.onkeydown = (e) => { if (e.key === "Enter") $namingSave.click(); };
}

function isValidName(name) {
  if (!name || name.length === 0 || name.length > 15) return false;
  // Interdit les emojis (blocs Unicode emoji)
  return !/[\u{1F000}-\u{1FFFF}\u{2600}-\u{27FF}\u{FE00}-\u{FEFF}]/u.test(name);
}

// ── Panneau réglages ──────────────────────────────────────────────────────────
const $settingsBtn     = document.getElementById("settings-btn");
const $settingsPanel   = document.getElementById("settings-panel");
const $settingsOverlay = document.getElementById("settings-overlay");
const $settingsClose   = document.getElementById("settings-close");
const $checkUpdateBtn  = document.getElementById("check-update-btn");
const $updateResult    = document.getElementById("update-result");

function openSettings() {
  renderSettingsDevices();
  $settingsPanel.classList.remove("hidden");
  requestAnimationFrame(() => $settingsPanel.classList.add("open"));
  $settingsOverlay.classList.remove("hidden");
}
function closeSettings() {
  $settingsPanel.classList.remove("open");
  $settingsOverlay.classList.add("hidden");
  setTimeout(() => $settingsPanel.classList.add("hidden"), 280);
}
$settingsBtn.addEventListener("click", openSettings);
$settingsClose.addEventListener("click", closeSettings);
$settingsOverlay.addEventListener("click", closeSettings);

document.querySelectorAll('input[name="theme"]').forEach(radio => {
  radio.addEventListener("change", () => {
    appSettings.theme = radio.value;
    applyTheme(radio.value);
  });
});

$checkUpdateBtn.addEventListener("click", async () => {
  $checkUpdateBtn.disabled = true;
  $updateResult.className = "update-result";
  $updateResult.textContent = "Vérification…";
  $updateResult.classList.remove("hidden");
  try {
    const latest = await invoke("check_for_update");
    if (latest && latest !== APP_VERSION) {
      $updateResult.classList.add("update-new");
      $updateResult.textContent = `Nouvelle version disponible : v${latest} — cliquez pour installer`;
      $updateResult.style.cursor = "pointer";
      $updateResult.onclick = () => {
        $updateResult.textContent = "Téléchargement en cours… (l'app va redémarrer)";
        $updateResult.style.cursor = "default";
        $updateResult.onclick = null;
        invoke("download_and_install_update");
      };
    } else {
      $updateResult.classList.add("update-ok");
      $updateResult.textContent = "Application à jour ✓";
    }
  } catch (e) {
    $updateResult.classList.add("update-err");
    $updateResult.textContent = "Pas de connexion Internet";
  }
  $checkUpdateBtn.disabled = false;
});

function renderSettingsDevices() {
  const $list = document.getElementById("settings-devices-list");
  const devices = appSettings.devices || {};
  const ids = Object.keys(devices);
  if (ids.length === 0) {
    $list.innerHTML = '<div class="settings-empty">Aucune boîte enregistrée</div>';
    return;
  }
  $list.innerHTML = "";
  for (const id of ids) {
    const info = devices[id];
    const row = document.createElement("div");
    row.className = "settings-device-row";
    row.innerHTML = `
      <div class="settings-device-info">
        <div class="settings-device-name">${info.name || id}</div>
        <div class="settings-device-sub">${id}</div>
      </div>
      <div class="settings-device-btns">
        <button class="btn-device-action" data-id="${id}" data-action="rename" title="Renommer">✏</button>
        <button class="btn-device-action del" data-id="${id}" data-action="delete" title="Supprimer"><svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" width="13" height="13"><polyline points="3,4 13,4"/><path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1"/><rect x="4" y="4" width="8" height="10" rx="1"/><line x1="6" y1="7" x2="6" y2="11"/><line x1="10" y1="7" x2="10" y2="11"/></svg></button>
      </div>`;
    $list.appendChild(row);
  }
  $list.querySelectorAll(".btn-device-action").forEach(btn => {
    btn.addEventListener("click", () => {
      const id = btn.dataset.id;
      if (btn.dataset.action === "rename") openRenameModal(id);
      else deleteDevice(id);
    });
  });
}

async function deleteDevice(id) {
  delete appSettings.devices[id];
  await invoke("save_device_name", { deviceId: id, name: "" }).catch(() => {});
  renderSettingsDevices();
  if (deviceId === id) renderDeviceName(null);
}

const $renameModal  = document.getElementById("rename-modal");
const $renameInput  = document.getElementById("rename-input");
const $renameError  = document.getElementById("rename-error");
const $renameSave   = document.getElementById("rename-save");
const $renameCancel = document.getElementById("rename-cancel");

function openRenameModal(id) {
  $renameInput.value = appSettings.devices[id]?.name || "";
  $renameError.classList.add("hidden");
  $renameModal.classList.remove("hidden");
  setTimeout(() => $renameInput.focus(), 50);
  $renameSave.onclick = async () => {
    const name = $renameInput.value.trim();
    if (!isValidName(name)) { $renameError.classList.remove("hidden"); return; }
    await invoke("save_device_name", { deviceId: id, name });
    appSettings.devices[id] = { ...(appSettings.devices[id] || {}), name };
    if (deviceId === id) renderDeviceName(id);
    $renameModal.classList.add("hidden");
    renderSettingsDevices();
  };
  $renameCancel.onclick = () => $renameModal.classList.add("hidden");
  $renameInput.onkeydown = e => { if (e.key === "Enter") $renameSave.click(); };
}

// ── Polling device (3s) ───────────────────────────────────────────────────────
async function pollDevice() {
  if (syncing) return;
  try {
    const probe = await invoke("probe_lunii_device");
    if (probe.connected && probe.mount) {
      deviceMount = probe.mount;
      deviceId = probe.deviceId || null;
      $deviceBadge.className = "device-badge badge-connected";
      $deviceBadge.textContent = "Connectée ✓";
      $devicePath.textContent = probe.mount;
      $devicePath.classList.remove("hidden");
      $ejectBtn.classList.remove("hidden");
      $repairBtn.classList.remove("hidden");
      // Affiche le nom ou propose d'en donner un
      if (deviceId) {
        // Migration : purger toutes les vieilles entrées UUID dès qu'une entrée serial existe
        if (deviceId.startsWith("serial-")) {
          const devices = appSettings.devices || {};
          const oldUUIDs = Object.keys(devices).filter(id => !id.startsWith("serial-"));
          if (oldUUIDs.length > 0) {
            if (!deviceName(deviceId)) {
              const oldEntry = Object.entries(devices).find(([id, info]) =>
                !id.startsWith("serial-") && info && info.name
              );
              if (oldEntry) {
                appSettings.devices[deviceId] = { name: oldEntry[1].name, lastFolder: oldEntry[1].lastFolder || "" };
                await invoke("save_device_name", { deviceId, name: oldEntry[1].name });
              }
            }
            for (const id of oldUUIDs) delete appSettings.devices[id];
            await invoke("save_last_folder", { deviceId, folder: appSettings.devices[deviceId]?.lastFolder || "" });
          }
        }
        renderDeviceName(deviceId);
        if (!deviceName(deviceId)) showNamingModal(deviceId);
      }

      // Inventaire
      const inv = await invoke("get_lunii_inventory");
      deviceStories = inv.stories || [];
      renderDeviceList();

      // Infos firmware
      try {
        const info = await invoke("get_device_info", { mount: probe.mount });
        if (info.hwVersion > 0) {
          $deviceFwLabel.textContent = `HW v${info.hwVersion}  ·  FW ${info.fwMajor}.${info.fwMinor}.${info.fwSubminor}`;
          $deviceFwLabel.classList.remove("hidden");
        }
      } catch { /* silencieux */ }

      // Espace disque
      try {
        const st = await invoke("get_storage_info", { mount: probe.mount });
        renderStorage(st);
      } catch { /* silencieux */ }

    } else {
      deviceMount = null;
      deviceStories = [];
      pendingDeletes.clear();
      $deviceBadge.className = "device-badge badge-disconnected";
      $deviceBadge.textContent = "Non connectée";
      $devicePath.classList.add("hidden");
      $ejectBtn.classList.add("hidden");
      $repairBtn.classList.add("hidden");
      $storageWrap.classList.add("hidden");
      $deviceHeader.style.display = "none";
      $deviceList.replaceChildren($deviceEmpty);
      $deviceEmpty.classList.remove("hidden");
      $deviceFwLabel.classList.add("hidden");
      const $t = document.getElementById("panel-device-title");
      if ($t) { $t.textContent = "Boîte à histoires"; $t.classList.remove("device-named"); }
    }
  } catch (e) {
    deviceMount = null;
  }
  refreshFolderBadges();
  updateSyncButton();
}

function renderStorage(st) {
  const pct = st.totalBytes > 0 ? (st.usedBytes / st.totalBytes) * 100 : 0;
  $storageFill.style.width = `${Math.min(pct, 100).toFixed(1)}%`;
  $storageFill.className = "storage-bar-fill" +
    (pct > 90 ? " danger" : pct > 75 ? " warn" : "");
  $storageUsed.textContent = `${fmtSize(st.usedBytes)} utilisés`;
  $storageFree.textContent = `${fmtSize(st.freeBytes)} libres`;
  $storageWrap.classList.remove("hidden");
}

function renderDeviceList() {
  $deviceEmpty.classList.add("hidden");
  $deviceHeader.style.display = "";
  $deviceCount.textContent = `${deviceStories.length} histoire${deviceStories.length !== 1 ? "s" : ""} sur la boîte`;

  const frag = document.createDocumentFragment();
  frag.appendChild($deviceEmpty);  // garder en DOM mais caché

  for (const s of deviceStories) {
    const hasName = !!s.title;
    const displayName = s.title || s.shortUuid;
    const row = document.createElement("div");
    row.className = "story-row";

    const av = document.createElement("div");
    av.className = "story-avatar" + (hasName ? "" : " avatar-unmanaged");
    av.textContent = initial(displayName);
    if (s.coverPath) {
      invoke("get_cover_base64", { path: s.coverPath }).then(dataUrl => {
        if (!dataUrl) return;
        const img = document.createElement("img");
        img.className = "story-cover-img";
        img.src = dataUrl;
        img.alt = displayName;
        av.textContent = "";
        av.appendChild(img);
      }).catch(() => {});
    }
    row.appendChild(av);

    const info = document.createElement("div");
    info.className = "story-info";
    const titleEl = document.createElement("div");
    titleEl.className = "story-name" + (hasName ? "" : " story-name-uuid");
    titleEl.textContent = displayName;
    info.appendChild(titleEl);
    const meta = document.createElement("div");
    meta.className = "story-meta";
    meta.textContent = hasName ? s.shortUuid : "Non géré par LuniiSync";
    info.appendChild(meta);
    row.appendChild(info);

    const sz = document.createElement("div");
    sz.className = "story-size";
    sz.textContent = fmtSize(s.sizeBytes || 0);
    row.appendChild(sz);

    // Bouton supprimer
    const delBtn = document.createElement("button");
    const isMarked = pendingDeletes.has(s.shortUuid);
    delBtn.className = "btn-delete" + (isMarked ? " marked" : "");
    delBtn.title = isMarked ? "Annuler la suppression" : "Supprimer de la boîte";
    delBtn.innerHTML = isMarked ? "↩" : `<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" width="14" height="14"><polyline points="3,4 13,4"/><path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1"/><rect x="4" y="4" width="8" height="10" rx="1"/><line x1="6" y1="7" x2="6" y2="11"/><line x1="10" y1="7" x2="10" y2="11"/></svg>`;
    if (isMarked) row.classList.add("story-row-delete");
    delBtn.addEventListener("click", () => toggleDelete(s.shortUuid, row, delBtn));
    row.appendChild(delBtn);

    frag.appendChild(row);
  }

  $deviceList.replaceChildren(frag);
}

// ── Suppression stories ───────────────────────────────────────────────────────
function toggleDelete(shortUuid, row, btn) {
  if (pendingDeletes.has(shortUuid)) {
    pendingDeletes.delete(shortUuid);
    row.classList.remove("story-row-delete");
    btn.className = "btn-delete";
    btn.title = "Supprimer de la boîte";
    btn.innerHTML = `<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" width="14" height="14"><polyline points="3,4 13,4"/><path d="M5 4V3a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v1"/><rect x="4" y="4" width="8" height="10" rx="1"/><line x1="6" y1="7" x2="6" y2="11"/><line x1="10" y1="7" x2="10" y2="11"/></svg>`;
  } else {
    pendingDeletes.add(shortUuid);
    row.classList.add("story-row-delete");
    btn.className = "btn-delete marked";
    btn.title = "Annuler la suppression";
    btn.innerHTML = "↩";
  }
  updateSyncButton();
}

// ── Sélection dossier ─────────────────────────────────────────────────────────
async function loadFolder(folderPath) {
  $folderPath.textContent = folderPath;
  $folderPath.classList.remove("hidden");
  pendingIds.clear();
  try {
    audioFiles = await invoke("list_audio_files", { folderPath });
    renderFolderList();
  } catch (e) {
    log("err", `Lecture dossier échouée : ${e}`);
  }
  updateSyncButton();
}

$pickBtn.addEventListener("click", async () => {
  const selected = await open({ directory: true, multiple: false });
  if (!selected) return;
  await invoke("save_last_folder", { folder: selected });
  appSettings.lastAudioFolder = selected;
  await loadFolder(selected);
});

function refreshFolderBadges() {
  // Remet à jour les badges "Dans la boîte" sans reconstruire toute la liste
  const deviceIds = new Set(deviceStories.map(s => s.sidecar?.storyId).filter(Boolean));
  document.querySelectorAll(".audio-row[data-story-id]").forEach(row => {
    const id = row.dataset.storyId;
    const tagEl = row.querySelector(".tag-in-box");
    if (deviceIds.has(id)) {
      if (!tagEl) {
        const tag = document.createElement("span");
        tag.className = "tag-in-box";
        tag.textContent = "Dans la boîte";
        row.querySelector(".audio-row-right").prepend(tag);
      }
    } else {
      tagEl?.remove();
    }
  });
}

function renderFolderList() {
  const deviceIds = new Set(deviceStories.map(s => s.sidecar?.storyId).filter(Boolean));

  $folderEmpty.classList.add("hidden");
  $folderHeader.style.display = "";
  $folderCount.textContent = `${audioFiles.length} fichier${audioFiles.length !== 1 ? "s" : ""}`;
  $addAllBtn.classList.remove("hidden");

  const frag = document.createDocumentFragment();
  frag.appendChild($folderEmpty);

  for (const af of audioFiles) {
    const inBox    = deviceIds.has(af.storyId);
    const isQueued = pendingIds.has(af.storyId);

    const row = document.createElement("div");
    row.className = "audio-row";
    row.dataset.storyId = af.storyId;

    const av = document.createElement("div");
    av.className = "audio-avatar" + (isQueued ? " queued" : "");
    av.textContent = initial(af.storyId);
    row.appendChild(av);

    const info = document.createElement("div");
    info.className = "audio-info";
    const titleEl = document.createElement("div");
    titleEl.className = "audio-name";
    titleEl.textContent = af.storyId.replace(/_/g, " ");
    info.appendChild(titleEl);
    const meta = document.createElement("div");
    meta.className = "audio-meta";
    const ext = af.filename.includes(".") ? af.filename.split(".").pop().toUpperCase() : "";
    meta.textContent = `${ext} · ${fmtSize(af.sizeBytes || 0)}`;
    info.appendChild(meta);
    row.appendChild(info);

    const right = document.createElement("div");
    right.className = "audio-row-right";

    if (inBox) {
      const tag = document.createElement("span");
      tag.className = "tag-in-box";
      const dot = document.createElement("span");
      dot.className = "tag-dot";
      tag.appendChild(dot);
      tag.appendChild(document.createTextNode("Déjà présent"));
      right.appendChild(tag);
    }

    const addBtn = document.createElement("button");
    addBtn.className = "btn-add" + (isQueued ? " added" : "");
    if (!isQueued) addBtn.textContent = "+";
    addBtn.title = isQueued ? "Déjà dans la file" : "Ajouter à la synchronisation";
    addBtn.addEventListener("click", () => togglePending(af.storyId, row, av, addBtn));
    right.appendChild(addBtn);

    row.appendChild(right);
    frag.appendChild(row);
  }

  $folderList.replaceChildren(frag);
}

function togglePending(storyId, row, av, btn) {
  if (pendingIds.has(storyId)) {
    pendingIds.delete(storyId);
    av.classList.remove("queued");
    btn.classList.remove("added");
    btn.textContent = "+";
    btn.title = "Ajouter à la synchronisation";
  } else {
    pendingIds.add(storyId);
    av.classList.add("queued");
    btn.classList.add("added");
    btn.textContent = "";
    btn.title = "Retirer de la synchronisation";
  }
  updateSyncButton();
}

// ── Tout ajouter / Tout retirer ───────────────────────────────────────────────
$addAllBtn.addEventListener("click", () => {
  for (const af of audioFiles) pendingIds.add(af.storyId);
  renderFolderList();
  updateSyncButton();
});

$removeAllBtn.addEventListener("click", () => {
  pendingIds.clear();
  renderFolderList();
  updateSyncButton();
});

// ── Bouton sync ───────────────────────────────────────────────────────────────
function updateSyncButton() {
  const hasWork = pendingIds.size > 0 || pendingDeletes.size > 0;
  $syncBtn.disabled = !deviceMount || !hasWork || syncing;
  const parts = [];
  if (pendingIds.size > 0) parts.push(`${pendingIds.size} à ajouter`);
  if (pendingDeletes.size > 0) parts.push(`${pendingDeletes.size} à supprimer`);
  if (parts.length > 0) {
    $pendingLabel.classList.remove("hidden");
    $pendingLabel.textContent = parts.join(" · ");
    if (pendingIds.size > 0) $removeAllBtn.classList.remove("hidden");
    else $removeAllBtn.classList.add("hidden");
  } else {
    $pendingLabel.classList.add("hidden");
    $removeAllBtn.classList.add("hidden");
  }
}

$syncBtn.addEventListener("click", startSync);

// ── Helpers sync status ───────────────────────────────────────────────────────
let _syncTotal = 0;

function showSyncStatus(text, current = 0, total = 0) {
  _syncTotal = total;
  $syncStatusText.textContent = text;
  $syncStatusCount.textContent = total > 0 ? `${current} / ${total}` : "";
  $syncStatusBar.classList.remove("hidden");
}

function hideSyncStatus() {
  $syncStatusBar.classList.add("hidden");
  $syncStatusCount.textContent = "";
}

let _toastTimer = null;
function showToast(msg, kind = "ok", durationMs = 4000) {
  if (_toastTimer) clearTimeout(_toastTimer);
  $syncToast.textContent = msg;
  $syncToast.className = `sync-toast toast-${kind}`;
  $syncToast.classList.remove("hidden");
  _toastTimer = setTimeout(() => $syncToast.classList.add("hidden"), durationMs);
}

function showSyncDone(added, errors, deleted = 0) {
  hideSyncStatus();
  const parts = [];
  if (added > 0)   parts.push(`${added} histoire${added > 1 ? "s" : ""} ajoutée${added > 1 ? "s" : ""} ✓`);
  if (deleted > 0) parts.push(`${deleted} histoire${deleted > 1 ? "s" : ""} supprimée${deleted > 1 ? "s" : ""} ✓`);
  if (errors > 0)  parts.push(`${errors} erreur${errors > 1 ? "s" : ""} ✗`);
  if (parts.length === 0) parts.push("Synchronisation terminée ✓");
  showToast(parts.join("  ·  "), errors > 0 ? "warn" : "ok", 5000);
}

async function startSync() {
  if (!deviceMount || (pendingIds.size === 0 && pendingDeletes.size === 0) || syncing) return;

  // Construire la liste des fichiers sélectionnés (peut être vide si suppressions seules)
  const selectedAudio = audioFiles.filter(a => pendingIds.has(a.storyId));
  const firstAudio = selectedAudio[0] || null;
  const folderPath = firstAudio ? firstAudio.path.substring(0, firstAudio.path.lastIndexOf("/")) : "";
  const selectedFiles = selectedAudio.map(a => a.path);
  const totalFiles = selectedFiles.length;

  const toDelete = [...pendingDeletes];

  syncing = true;
  $syncBtn.disabled = true;
  $syncBtn.classList.add("syncing");
  $syncBtn.querySelector(".btn-icon").textContent = "⟳";
  $logDrawer.classList.remove("hidden");
  $logOutput.replaceChildren();
  showSyncStatus("Démarrage de la synchronisation…");
  log("info", "⏳ Démarrage de la synchronisation…");

  // Suppressions d'abord
  let deleted = 0;
  let doneDeleted = 0;
  for (const uuid of toDelete) {
    try {
      await invoke("remove_orphan_story", { mount: deviceMount, shortUuid: uuid });
      log("ok", `🗑 Supprimé : ${uuid}`);
      deleted++;
    } catch (e) {
      log("err", `Suppression ${uuid} échouée : ${e}`);
    }
  }
  doneDeleted = deleted;
  if (deleted > 0) log("info", `${deleted} histoire(s) supprimée(s).`);

  const unlisten = await listen("sync:line", ({ payload }) => {
    try { handleBridgeMsg(JSON.parse(payload)); }
    catch { log("info", payload); }
  });

  let doneAdded = deleted, doneErrors = 0;
  try {
    if (selectedFiles.length > 0) {
      showSyncStatus("Transfert en cours…", 0, totalFiles);
      await invoke("start_sync", { folderPath, deviceMount, selectedFiles });
    }
    pendingIds.clear();
    pendingDeletes.clear();
    showSyncStatus("Mise à jour de l'inventaire…");
    await pollDevice();
    if (audioFiles.length > 0) renderFolderList();
    if (selectedFiles.length === 0 && deleted > 0) {
      showSyncDone(0, 0, deleted);
    }
  } catch (e) {
    log("err", `Erreur : ${e}`);
    hideSyncStatus();
    showToast(`Erreur : ${e}`, "err", 6000);
  } finally {
    unlisten();
    syncing = false;
    $syncBtn.classList.remove("syncing");
    $syncBtn.querySelector(".btn-icon").textContent = "↺";
    updateSyncButton();
  }
}

const STEP_LABELS = {
  setup:  (m) => m.includes("Clon") ? "⬇ Clonage Lunii.QT…"
               : m.includes("Téléchargement") ? "⬇ Téléchargement studio-pack-generator…"
               : m.includes("prêt") ? "✓ Dépendances prêtes"
               : `⚙ ${m}`,
  scan:   (m) => `🔍 ${m}`,
  import: (m, msg) => msg.current != null
               ? `📦 [${msg.current}/${msg.total}] ${msg.file || m}`
               : `📦 ${m}`,
};

function handleBridgeMsg(msg) {
  switch (msg.type) {
    case "progress": {
      const label = STEP_LABELS[msg.step]
        ? STEP_LABELS[msg.step](msg.message, msg)
        : msg.message;
      log("info", label);
      if (msg.step === "import" && msg.current != null) {
        showSyncStatus(`📦 ${msg.file || "Traitement…"}`, msg.current, msg.total);
      } else if (msg.step === "setup") {
        showSyncStatus(label.replace(/^[^ ]+ /, ""));
      }
      break;
    }
    case "error":
      log("err", `✗ ${msg.message || JSON.stringify(msg)}`);
      doneErrors++;
      break;
    case "done":
      log("ok", `✓ Terminé : ${msg.added ?? 0} ajouté(s), ${msg.errors ?? 0} erreur(s).`);
      showSyncDone(msg.added ?? 0, msg.errors ?? 0, doneDeleted);
      break;
    case "stderr":
      if (msg.message && !msg.message.startsWith("QStandardPaths")) log("warn", msg.message);
      break;
    default:
      log("info", JSON.stringify(msg));
  }
}

// ── Journal ───────────────────────────────────────────────────────────────────
$logToggle.addEventListener("click", () => {
  $logOutput.style.display = $logOutput.style.display === "none" ? "" : "none";
});

function log(level, text) {
  const ts = new Date().toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  const line = document.createElement("div");
  line.className = `log-${level}`;
  line.textContent = `[${ts}] ${text}`;
  $logOutput.appendChild(line);
  $logOutput.scrollTop = $logOutput.scrollHeight;
}

// ── Éjection ──────────────────────────────────────────────────────────────────
$ejectBtn.addEventListener("click", async () => {
  if (!deviceMount || syncing) return;
  const mount = deviceMount;
  try {
    await invoke("eject_device", { mount });
    // Réinitialise l'état immédiatement sans attendre le prochain poll
    deviceMount = null;
    deviceStories = [];
    $deviceBadge.className = "device-badge badge-disconnected";
    $deviceBadge.textContent = "Non connectée";
    $devicePath.classList.add("hidden");
    $ejectBtn.classList.add("hidden");
    $storageWrap.classList.add("hidden");
    $deviceHeader.style.display = "none";
    $deviceList.replaceChildren($deviceEmpty);
    $deviceEmpty.classList.remove("hidden");
    refreshFolderBadges();
    updateSyncButton();
    log("ok", "Boîte éjectée en toute sécurité.");
    $logDrawer.classList.remove("hidden");
  } catch (e) {
    log("err", `Éjection échouée : ${e}`);
    $logDrawer.classList.remove("hidden");
  }
});

// ── Réparation index ──────────────────────────────────────────────────────────
$repairBtn.addEventListener("click", async () => {
  if (!deviceMount || syncing) return;
  $repairBtn.disabled = true;
  $repairBtn.title = "Réparation en cours…";
  log("ok", "Réparation de l'index en cours…");
  $logDrawer.classList.remove("hidden");
  try {
    await invoke("repair_pack_index", { deviceMount });
    log("ok", "Index réparé — redémarre la Lunii pour voir les histoires.");
  } catch (e) {
    log("err", `Réparation échouée : ${e}`);
  } finally {
    $repairBtn.disabled = false;
    $repairBtn.title = "Réparer l'index de la boîte";
  }
});

// ── Init ──────────────────────────────────────────────────────────────────────
(async () => {
  // Splash en parallèle de l'init
  const splashPromise = runSplash();

  appSettings = await invoke("get_app_settings");

  // Appliquer le thème sauvegardé
  const savedTheme = appSettings.theme || "auto";
  applyTheme(savedTheme);
  const themeRadio = document.querySelector(`input[name="theme"][value="${savedTheme}"]`);
  if (themeRadio) themeRadio.checked = true;

  if (appSettings.lastAudioFolder) {
    try { await loadFolder(appSettings.lastAudioFolder); } catch {}
  }

  await splashPromise;

  pollDevice();
  setInterval(pollDevice, 3000);
})();

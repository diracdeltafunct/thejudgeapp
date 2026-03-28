import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface UpdateInfo {
  doc_type: string;
  label: string;
  installed_version: string | null;
  available_version: string;
  url: string;
  update_available: boolean;
  size_bytes: number | null;
}

interface ProgressEvent {
  doc_type: string;
  phase: "downloading" | "parsing" | "importing";
  percent: number;
}

export async function checkForUpdates(): Promise<number> {
  try {
    const updates: UpdateInfo[] = await invoke("check_for_data_updates");
    return updates.filter((u) => u.update_available).length;
  } catch {
    return 0;
  }
}

export function initUpdatesPage(container: HTMLElement): void {
  container.innerHTML = `
    <div class="updates-page">
      <h1>Data Updates</h1>
      <p class="updates-subtitle">Keep your rules documents current without reinstalling the app.</p>
      <div id="updates-list" class="updates-list">
        <div class="update-checking">Checking for updates…</div>
      </div>
    </div>
  `;

  loadUpdates(container);
}

export function initUpdatesSection(container: HTMLElement): void {
  container.innerHTML = `
    <button class="tip-btn" id="check-updates-btn">Check for Updates</button>
    <div id="updates-list" class="updates-list"></div>
  `;

  container.querySelector("#check-updates-btn")!.addEventListener("click", async (e) => {
    const btn = e.currentTarget as HTMLButtonElement;
    btn.disabled = true;
    btn.textContent = "Checking…";
    container.querySelector("#updates-list")!.innerHTML = `<div class="update-checking">Checking for updates…</div>`;
    await loadUpdatesSection(container);
    btn.remove();
  });
}

function updateCardHtml(u: UpdateInfo): string {
  const scryfallError = u.doc_type === "cards" && u.url === "";
  const upToDate = !u.update_available && !scryfallError;
  return `
    <div class="update-card ${u.update_available ? "update-card--available" : "update-card--current"}" data-doc="${u.doc_type}">
      <div class="update-card-header">
        <div>
          <div class="update-card-label">${u.label}</div>
          <div class="update-card-versions">
            <span class="update-installed">Installed: ${u.installed_version ?? "none"}</span>
            ${u.update_available ? `<span class="update-arrow">→</span><span class="update-available">${u.available_version}</span>` : ""}
          </div>
        </div>
        ${scryfallError
          ? `<span class="update-current-badge" style="color:var(--text-muted)">Cannot check</span>`
          : upToDate
          ? `<span class="update-current-badge">Up to date</span>`
          : `<div class="update-btn-group">
              ${u.size_bytes ? `<span class="update-size">${formatSize(u.size_bytes)}</span>` : ""}
              <button class="update-btn" data-doc="${u.doc_type}" data-url="${escHtml(u.url)}" data-version="${escHtml(u.available_version)}">Update</button>
            </div>`
        }
      </div>
      <div class="update-progress hidden" id="progress-${u.doc_type}">
        <div class="update-progress-bar">
          <div class="update-progress-fill" id="progress-fill-${u.doc_type}" style="width:0%"></div>
        </div>
        <div class="update-progress-info">
          <span class="update-progress-label" id="progress-label-${u.doc_type}">Starting…</span>
          <button class="update-cancel-btn" id="cancel-${u.doc_type}">Cancel</button>
        </div>
      </div>
      <div class="update-status" id="status-${u.doc_type}"></div>
    </div>`;
}

async function loadUpdatesSection(container: HTMLElement): Promise<void> {
  const list = container.querySelector("#updates-list")!;

  let updates: UpdateInfo[];
  try {
    updates = await invoke("check_for_data_updates");
  } catch {
    list.innerHTML = `<div class="update-none">Could not reach update server.</div>`;
    return;
  }

  const alwaysShow = new Set(["cards", "rulings", "riftbound_cards"]);
  const visible = updates.filter(
    (u) => u.update_available || alwaysShow.has(u.doc_type),
  );

  if (!visible.length) {
    list.innerHTML = `<div class="update-none">All data is up to date.</div>`;
    return;
  }

  list.innerHTML = visible.map(updateCardHtml).join("");

  container.querySelectorAll<HTMLButtonElement>(".update-btn").forEach((btn) => {
    btn.addEventListener("click", () => applyUpdate(btn, container));
  });
}

async function loadUpdates(container: HTMLElement): Promise<void> {
  const list = container.querySelector("#updates-list")!;

  let updates: UpdateInfo[];
  try {
    updates = await invoke("check_for_data_updates");
  } catch (err) {
    list.innerHTML = `<div class="update-error">Could not reach update server.<br><small>${err}</small></div>`;
    return;
  }

  if (!updates.length) {
    list.innerHTML = `<div class="update-none">No update information available.</div>`;
    return;
  }

  list.innerHTML = updates.map(updateCardHtml).join("");

  container.querySelectorAll<HTMLButtonElement>(".update-btn").forEach((btn) => {
    btn.addEventListener("click", () => applyUpdate(btn, container));
  });
}

async function applyUpdate(btn: HTMLButtonElement, container: HTMLElement): Promise<void> {
  const docType = btn.dataset.doc!;
  const url = btn.dataset.url!;
  const manifestVersion = btn.dataset.version!;
  const statusEl = container.querySelector<HTMLElement>(`#status-${docType}`)!;
  const progressEl = container.querySelector<HTMLElement>(`#progress-${docType}`)!;
  const fillEl = container.querySelector<HTMLElement>(`#progress-fill-${docType}`)!;
  const labelEl = container.querySelector<HTMLElement>(`#progress-label-${docType}`)!;
  const cancelBtn = container.querySelector<HTMLButtonElement>(`#cancel-${docType}`)!;

  btn.style.display = "none";
  progressEl.classList.remove("hidden");
  statusEl.className = "update-status";
  statusEl.textContent = "";

  const phaseLabels: Record<string, string> = {
    downloading: "Downloading…",
    parsing: "Parsing…",
    importing: "Importing…",
  };

  const unlisten = await listen<ProgressEvent>("update-progress", (event) => {
    if (event.payload.doc_type !== docType) return;
    fillEl.style.width = `${event.payload.percent}%`;
    labelEl.textContent = phaseLabels[event.payload.phase] ?? event.payload.phase;
  });

  cancelBtn.addEventListener("click", () => {
    invoke("cancel_update");
    labelEl.textContent = "Cancelling…";
    cancelBtn.disabled = true;
  }, { once: true });

  try {
    const newVersion: string = await invoke("apply_data_update", { docType, url, manifestVersion });
    progressEl.classList.add("hidden");
    btn.textContent = "Updated";
    btn.classList.add("update-btn--done");
    btn.style.display = "";
    statusEl.textContent = `Updated to ${newVersion}`;
    statusEl.className = "update-status update-status--success";
    window.dispatchEvent(new CustomEvent("data-updated"));
  } catch (err) {
    progressEl.classList.add("hidden");
    const wasCancelled = String(err).toLowerCase().includes("cancelled");
    btn.disabled = false;
    btn.style.display = "";
    if (wasCancelled) {
      btn.textContent = "Update";
      statusEl.textContent = "Update cancelled.";
      statusEl.className = "update-status update-status--error";
    } else {
      btn.textContent = "Retry";
      statusEl.textContent = `Error: ${err}`;
      statusEl.className = "update-status update-status--error";
    }
  } finally {
    unlisten();
  }
}

export function formatSize(bytes: number | null): string {
  if (bytes === null) return "";
  if (bytes >= 1_000_000) return `~${Math.round(bytes / 1_000_000)} MB`;
  if (bytes >= 1_000) return `~${Math.round(bytes / 1_000)} KB`;
  return `${bytes} B`;
}

export function escHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

import { invoke } from "@tauri-apps/api/core";

interface UpdateInfo {
  doc_type: string;
  label: string;
  installed_version: string | null;
  available_version: string;
  url: string;
  update_available: boolean;
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

  list.innerHTML = updates
    .map(
      (u) => `
    <div class="update-card ${u.update_available ? "update-card--available" : "update-card--current"}" data-doc="${u.doc_type}">
      <div class="update-card-header">
        <div>
          <div class="update-card-label">${u.label}</div>
          <div class="update-card-versions">
            <span class="update-installed">Installed: ${u.installed_version ?? "none"}</span>
            ${u.update_available ? `<span class="update-arrow">→</span><span class="update-available">${u.available_version}</span>` : ""}
          </div>
        </div>
        ${
          u.update_available
            ? `<button class="update-btn" data-doc="${u.doc_type}" data-url="${escHtml(u.url)}" data-version="${escHtml(u.available_version)}">Update</button>`
            : `<span class="update-current-badge">Up to date</span>`
        }
      </div>
      <div class="update-status" id="status-${u.doc_type}"></div>
    </div>
  `,
    )
    .join("");

  container.querySelectorAll<HTMLButtonElement>(".update-btn").forEach((btn) => {
    btn.addEventListener("click", () => applyUpdate(btn, container));
  });
}

async function applyUpdate(btn: HTMLButtonElement, container: HTMLElement): Promise<void> {
  const docType = btn.dataset.doc!;
  const url = btn.dataset.url!;
  const version = btn.dataset.version!;
  const statusEl = container.querySelector<HTMLElement>(`#status-${docType}`)!;

  btn.disabled = true;
  btn.textContent = "Updating…";
  const isCards = docType === "cards";
  statusEl.textContent = isCards
    ? "Downloading card data (~250 MB), this may take a few minutes…"
    : "Downloading and importing…";
  statusEl.className = "update-status update-status--progress";

  try {
    const newVersion: string = await invoke("apply_data_update", {
      docType,
      url,
      version,
    });
    btn.textContent = "Updated";
    btn.classList.add("update-btn--done");
    statusEl.textContent = `Updated to ${newVersion}`;
    statusEl.className = "update-status update-status--success";
    // Refresh the badge count in the nav
    window.dispatchEvent(new CustomEvent("data-updated"));
  } catch (err) {
    btn.disabled = false;
    btn.textContent = "Retry";
    statusEl.textContent = `Error: ${err}`;
    statusEl.className = "update-status update-status--error";
  }
}

function escHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

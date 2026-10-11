import { invoke } from "@tauri-apps/api/core";

export interface AppUpdateInfo {
  installed_version: string;
  available_version: string;
  url: string;
  update_available: boolean;
}

const PROMPTED_THIS_LAUNCH = "app-update-prompted-this-launch";

export async function getAppUpdate(): Promise<AppUpdateInfo | null> {
  try {
    return await invoke<AppUpdateInfo | null>("check_for_app_update");
  } catch {
    return null;
  }
}

async function openStore(url: string): Promise<void> {
  await invoke("open_custom_tab", { url });
}

export async function showAppUpdatePromptOnce(): Promise<void> {
  if (sessionStorage.getItem(PROMPTED_THIS_LAUNCH)) return;

  const update = await getAppUpdate();
  if (!update?.update_available) return;

  // Set this before displaying the prompt so route changes or repeated boot
  // callbacks cannot show it twice during the same application process.
  sessionStorage.setItem(PROMPTED_THIS_LAUNCH, "true");

  document.querySelector("#app-update-overlay")?.remove();
  const overlay = document.createElement("div");
  overlay.id = "app-update-overlay";
  overlay.className = "app-update-overlay";
  overlay.innerHTML = `
    <div class="app-update-dialog" role="dialog" aria-modal="true" aria-labelledby="app-update-title">
      <div class="app-update-icon" aria-hidden="true">↑</div>
      <h2 id="app-update-title">App update available</h2>
      <p>Version ${escapeHtml(update.available_version)} is available on Google Play.</p>
      <div class="app-update-actions">
        <button type="button" class="app-update-later">Later</button>
        <button type="button" class="app-update-now">Update now</button>
      </div>
      <div class="app-update-error" aria-live="polite"></div>
    </div>`;
  document.body.appendChild(overlay);

  const close = () => overlay.remove();
  overlay.querySelector<HTMLButtonElement>(".app-update-later")!.addEventListener("click", close);
  overlay.addEventListener("click", (event) => {
    if (event.target === overlay) close();
  });
  overlay.querySelector<HTMLButtonElement>(".app-update-now")!.addEventListener("click", async (event) => {
    const button = event.currentTarget as HTMLButtonElement;
    button.disabled = true;
    try {
      await openStore(update.url);
      close();
    } catch {
      button.disabled = false;
      overlay.querySelector<HTMLElement>(".app-update-error")!.textContent =
        "Could not open Google Play. Please try again from Settings.";
    }
  });
  overlay.querySelector<HTMLButtonElement>(".app-update-now")!.focus();
}

export async function initAppUpdateSetting(container: HTMLElement): Promise<void> {
  container.innerHTML = `<div class="update-checking">Checking app version…</div>`;
  const update = await getAppUpdate();
  if (!container.isConnected) return;

  if (!update) {
    container.innerHTML = `<div class="update-none">Could not check the Play Store version.</div>`;
    return;
  }

  container.innerHTML = `
    <div class="update-card ${update.update_available ? "update-card--available" : "update-card--current"}">
      <div class="update-card-header">
        <div>
          <div class="update-card-label">The Judge App</div>
          <div class="update-card-versions">
            <span class="update-installed">Installed: ${escapeHtml(update.installed_version)}</span>
            ${update.update_available ? `<span class="update-arrow">→</span><span class="update-available">${escapeHtml(update.available_version)}</span>` : ""}
          </div>
        </div>
        ${update.update_available
          ? `<button type="button" class="update-btn app-store-update-btn">Update</button>`
          : `<span class="update-current-badge">Up to date</span>`}
      </div>
      <div class="update-status app-store-update-status" aria-live="polite"></div>
    </div>`;

  container.querySelector<HTMLButtonElement>(".app-store-update-btn")?.addEventListener("click", async (event) => {
    const button = event.currentTarget as HTMLButtonElement;
    button.disabled = true;
    try {
      await openStore(update.url);
    } catch {
      container.querySelector<HTMLElement>(".app-store-update-status")!.textContent =
        "Could not open Google Play.";
    } finally {
      button.disabled = false;
    }
  });
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

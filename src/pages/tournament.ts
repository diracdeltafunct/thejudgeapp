import { invoke } from "@tauri-apps/api/core";

const STORAGE_KEY = "tournaments";

interface Tournament {
  id: string;
  name: string;
  event_software: string;
  purple_fox: string | null;
  schedule: string | null;
  tracking_sheet: string | null;
  created_at: string;
}

function loadTournaments(): Tournament[] {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
  } catch {
    return [];
  }
}

function saveTournaments(tournaments: Tournament[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(tournaments));
}

function openInApp(url: string): void {
  invoke("open_custom_tab", { url });
}

export function initNewTournament(container: HTMLElement): void {
  container.innerHTML = `
    <div class="tournament-form-page">
      <h1>New Tournament</h1>
      <form id="new-tournament-form" class="tournament-form">
        <div class="form-group">
          <label for="tournament-name">Name</label>
          <input type="text" id="tournament-name" name="name" placeholder="e.g. FNM April 2025" required />
        </div>
        <div class="form-group">
          <label for="event-software">Event Software</label>
          <input type="url" id="event-software" name="event_software" placeholder="https://" required />
        </div>
        <div class="form-group">
          <label for="purple-fox">
            Purple Fox
            <span class="label-optional">optional</span>
          </label>
          <input type="url" id="purple-fox" name="purple_fox" placeholder="https://" />
        </div>
        <div class="form-group">
          <label for="schedule">Schedule <span class="label-hint">Google Drive</span> <span class="label-optional">optional</span></label>
          <input type="url" id="schedule" name="schedule" placeholder="https://docs.google.com/..." />
        </div>
        <div class="form-group">
          <label for="tracking-sheet">Tracking Sheet 1 <span class="label-hint">Google Drive</span> <span class="label-optional">optional</span></label>
          <input type="url" id="tracking-sheet" name="tracking_sheet" placeholder="https://docs.google.com/..." />
        </div>
        <button type="submit" class="form-submit">Create Tournament</button>
      </form>
    </div>
  `;

  container.querySelector("#new-tournament-form")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const tournament: Tournament = {
      id: crypto.randomUUID(),
      name: (form.elements.namedItem("name") as HTMLInputElement).value.trim(),
      event_software: (form.elements.namedItem("event_software") as HTMLInputElement).value,
      purple_fox: (form.elements.namedItem("purple_fox") as HTMLInputElement).value || null,
      schedule: (form.elements.namedItem("schedule") as HTMLInputElement).value || null,
      tracking_sheet: (form.elements.namedItem("tracking_sheet") as HTMLInputElement).value || null,
      created_at: new Date().toISOString(),
    };

    const tournaments = loadTournaments();
    tournaments.push(tournament);
    saveTournaments(tournaments);

    window.location.hash = "#/tournament/active";
  });
}

export function initActiveTournaments(container: HTMLElement): void {
  const tournaments = loadTournaments();

  if (!tournaments.length) {
    container.innerHTML = `
      <div class="tournament-list-page">
        <h1>Active Tournaments</h1>
        <p class="empty-state">No tournaments yet. <a href="#/tournament/new">Add one.</a></p>
      </div>
    `;
    return;
  }

  const cards = tournaments.map((t) => `
    <div class="tournament-card" data-id="${escHtml(t.id)}">
      <div class="tournament-card-header">
        <div class="tournament-card-name">${escHtml(t.name)}</div>
        <button class="tournament-delete" data-id="${escHtml(t.id)}" data-name="${escHtml(t.name)}" aria-label="Delete tournament">✕</button>
      </div>
      <div class="tournament-card-links">
        <button class="tournament-link" data-url="${escHtml(t.event_software)}" data-title="Event Software">Event Software</button>
        ${t.purple_fox ? `<button class="tournament-link" data-url="${escHtml(t.purple_fox)}" data-title="Purple Fox">Purple Fox</button>` : ""}
        ${t.schedule ? `<button class="tournament-link" data-url="${escHtml(t.schedule)}" data-title="Schedule">Schedule</button>` : ""}
        ${t.tracking_sheet ? `<button class="tournament-link" data-url="${escHtml(t.tracking_sheet)}" data-title="Tracking Sheet">Tracking Sheet</button>` : ""}
      </div>
    </div>
  `).join("");

  container.innerHTML = `
    <div class="tournament-list-page">
      <h1>Active Tournaments</h1>
      <div class="tournament-list">${cards}</div>
    </div>
  `;

  container.querySelectorAll<HTMLButtonElement>(".tournament-link").forEach((btn) => {
    btn.addEventListener("click", () => {
      openInApp(btn.dataset.url!);
    });
  });

  container.querySelectorAll<HTMLButtonElement>(".tournament-delete").forEach((btn) => {
    btn.addEventListener("click", () => {
      const name = btn.dataset.name!;
      const id = btn.dataset.id!;
      showConfirm(`Delete "${name}"?`, () => {
        const updated = loadTournaments().filter((t) => t.id !== id);
        saveTournaments(updated);
        initActiveTournaments(container);
      });
    });
  });
}

function showConfirm(message: string, onConfirm: () => void): void {
  const overlay = document.createElement("div");
  overlay.className = "confirm-overlay";
  overlay.innerHTML = `
    <div class="confirm-dialog">
      <p class="confirm-message">${escHtml(message)}</p>
      <div class="confirm-actions">
        <button class="confirm-cancel">Cancel</button>
        <button class="confirm-ok">Delete</button>
      </div>
    </div>
  `;

  const close = () => document.body.removeChild(overlay);
  overlay.querySelector(".confirm-cancel")!.addEventListener("click", close);
  overlay.querySelector(".confirm-ok")!.addEventListener("click", () => {
    close();
    onConfirm();
  });
  overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });

  document.body.appendChild(overlay);
}

function escHtml(str: string): string {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

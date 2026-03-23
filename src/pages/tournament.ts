import { invoke } from "@tauri-apps/api/core";

const STORAGE_KEY = "tournaments";

interface Tournament {
  id: string;
  name: string;
  event_software: string;
  purple_fox: string | null;
  schedule: string | null;
  tracking_sheet: string | null;
  discord: string | null;
  notes: string | null;
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
        <div class="form-group">
          <label for="discord">Discord Channel <span class="label-optional">optional</span></label>
          <input type="text" id="discord" name="discord" placeholder="https://discord.com/channels/..." />
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
      discord: (form.elements.namedItem("discord") as HTMLInputElement).value || null,
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
        ${t.discord ? `<button class="tournament-link" data-url="${escHtml(t.discord)}" data-title="Discord">Discord</button>` : ""}
      </div>
      <div class="tournament-card-footer">
        <button class="tournament-camera" data-id="${escHtml(t.id)}" aria-label="Photo album">&#128247;</button>
        <button class="tournament-notes" data-id="${escHtml(t.id)}" aria-label="Notes">${t.notes ? "&#128221;" : "&#128203;"}</button>
        <button class="tournament-settings" data-id="${escHtml(t.id)}" aria-label="Edit tournament">&#9881;</button>
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

  container.querySelectorAll<HTMLButtonElement>(".tournament-camera").forEach((btn) => {
    btn.addEventListener("click", () => {
      window.location.hash = `#/tournament/album/${btn.dataset.id}`;
    });
  });

  container.querySelectorAll<HTMLButtonElement>(".tournament-notes").forEach((btn) => {
    btn.addEventListener("click", () => {
      window.location.hash = `#/tournament/notes/${btn.dataset.id}`;
    });
  });

  container.querySelectorAll<HTMLButtonElement>(".tournament-settings").forEach((btn) => {
    btn.addEventListener("click", () => {
      window.location.hash = `#/tournament/edit/${btn.dataset.id}`;
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

export function initEditTournament(container: HTMLElement, id: string): void {
  const tournament = loadTournaments().find((t) => t.id === id);
  if (!tournament) {
    window.location.hash = "#/tournament/active";
    return;
  }

  container.innerHTML = `
    <div class="tournament-form-page">
      <h1>Edit Tournament</h1>
      <form id="edit-tournament-form" class="tournament-form">
        <div class="form-group">
          <label for="tournament-name">Name</label>
          <input type="text" id="tournament-name" name="name" placeholder="e.g. FNM April 2025" required value="${escHtml(tournament.name)}" />
        </div>
        <div class="form-group">
          <label for="event-software">Event Software</label>
          <input type="url" id="event-software" name="event_software" placeholder="https://" required value="${escHtml(tournament.event_software)}" />
        </div>
        <div class="form-group">
          <label for="purple-fox">
            Purple Fox
            <span class="label-optional">optional</span>
          </label>
          <input type="url" id="purple-fox" name="purple_fox" placeholder="https://" value="${escHtml(tournament.purple_fox ?? "")}" />
        </div>
        <div class="form-group">
          <label for="schedule">Schedule <span class="label-hint">Google Drive</span> <span class="label-optional">optional</span></label>
          <input type="url" id="schedule" name="schedule" placeholder="https://docs.google.com/..." value="${escHtml(tournament.schedule ?? "")}" />
        </div>
        <div class="form-group">
          <label for="tracking-sheet">Tracking Sheet 1 <span class="label-hint">Google Drive</span> <span class="label-optional">optional</span></label>
          <input type="url" id="tracking-sheet" name="tracking_sheet" placeholder="https://docs.google.com/..." value="${escHtml(tournament.tracking_sheet ?? "")}" />
        </div>
        <div class="form-group">
          <label for="discord">Discord Channel <span class="label-optional">optional</span></label>
          <input type="text" id="discord" name="discord" placeholder="https://discord.com/channels/..." value="${escHtml(tournament.discord ?? "")}" />
        </div>
        <button type="submit" class="form-submit">Save</button>
      </form>
    </div>
  `;

  container.querySelector("#edit-tournament-form")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const updated: Tournament = {
      ...tournament,
      name: (form.elements.namedItem("name") as HTMLInputElement).value.trim(),
      event_software: (form.elements.namedItem("event_software") as HTMLInputElement).value,
      purple_fox: (form.elements.namedItem("purple_fox") as HTMLInputElement).value || null,
      schedule: (form.elements.namedItem("schedule") as HTMLInputElement).value || null,
      tracking_sheet: (form.elements.namedItem("tracking_sheet") as HTMLInputElement).value || null,
      discord: (form.elements.namedItem("discord") as HTMLInputElement).value || null,
    };

    const tournaments = loadTournaments();
    saveTournaments(tournaments.map((t) => (t.id === id ? updated : t)));

    window.location.hash = "#/tournament/active";
  });
}

export function initTournamentNotes(container: HTMLElement, id: string): void {
  const tournament = loadTournaments().find((t) => t.id === id);
  if (!tournament) {
    window.location.hash = "#/tournament/active";
    return;
  }

  let saveTimer: ReturnType<typeof setTimeout> | null = null;

  container.innerHTML = `
    <div class="notes-page">
      <div class="notes-header">
        <h1>${escHtml(tournament.name)}</h1>
        <button class="notes-export-btn" id="notes-export" aria-label="Export notes">&#8679; Export</button>
      </div>
      <textarea class="notes-textarea" id="notes-textarea" placeholder="Write notes here...">${escHtml(tournament.notes ?? "")}</textarea>
      <div class="notes-status" id="notes-status"></div>
    </div>
  `;

  const textarea = container.querySelector<HTMLTextAreaElement>("#notes-textarea")!;
  const status = container.querySelector<HTMLElement>("#notes-status")!;

  textarea.addEventListener("input", () => {
    if (saveTimer) clearTimeout(saveTimer);
    status.textContent = "Saving…";
    saveTimer = setTimeout(() => {
      const tournaments = loadTournaments();
      saveTournaments(
        tournaments.map((t) =>
          t.id === id ? { ...t, notes: textarea.value || null } : t,
        ),
      );
      status.textContent = "Saved";
      setTimeout(() => { status.textContent = ""; }, 1500);
    }, 600);
  });

  container.querySelector("#notes-export")!.addEventListener("click", () => {
    const text = textarea.value.trim();
    if (!text) return;
    const content = `${tournament.name}\n${"=".repeat(tournament.name.length)}\n\n${text}`;
    const filename = `${tournament.name.replace(/[^a-z0-9]/gi, "_")}_notes.txt`;

    const download = () => {
      const blob = new Blob([content], { type: "text/plain" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
    };

    if (navigator.share) {
      navigator.share({ title: `${tournament.name} Notes`, text: content }).catch(download);
    } else {
      download();
    }
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

import { invoke } from "@tauri-apps/api/core";
import { initTimerCard, clearAllTimerIntervals, deleteTimerState, getDefaultRoundTime } from "./timer.js";

const STORAGE_KEY = "tournaments";

interface LinkEntry {
  url: string;
  label: string;
}

interface Tournament {
  id: string;
  name: string;
  event_software: string | null;
  purple_fox: string | null;
  schedule: LinkEntry[];
  tracking_sheet: LinkEntry[];
  discord: LinkEntry[];
  notes: string | null;
  created_at: string;
}

function normalizeTournament(t: any): Tournament {
  const toEntries = (v: any): LinkEntry[] => {
    if (Array.isArray(v)) {
      return v.filter(Boolean).map((item) =>
        typeof item === "string" ? { url: item, label: "" } : item,
      );
    }
    if (typeof v === "string" && v) return [{ url: v, label: "" }];
    return [];
  };
  return {
    ...t,
    schedule: toEntries(t.schedule),
    tracking_sheet: toEntries(t.tracking_sheet),
    discord: toEntries(t.discord),
  };
}

function loadTournaments(): Tournament[] {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
    return raw.map(normalizeTournament);
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

// ── Multi-field helpers ───────────────────────────────────────────────────────

function buildMultiField(
  label: string,
  fieldName: string,
  entries: LinkEntry[],
  inputType: string,
  placeholder: string,
  defaultName: string,
): string {
  const rows = entries.length > 0 ? entries : [{ url: "", label: "" }];
  const showLabels = rows.length > 1;
  return `
    <div class="form-group multi-field" data-field="${fieldName}" data-default-name="${escHtml(defaultName)}">
      <label>${label} <span class="label-optional">optional</span></label>
      ${rows
        .map(
          (entry, i) => {
            const labelValue = entry.label || (showLabels ? `${defaultName} ${i + 1}` : "");
            return `
        <div class="multi-field-row">
          <div class="multi-field-inputs">
            <input type="${inputType}" name="${fieldName}_url" placeholder="${placeholder}" value="${escHtml(entry.url)}" />
            <input type="text" name="${fieldName}_label" class="multi-field-label" placeholder="Label" value="${escHtml(labelValue)}"${showLabels ? "" : ' style="display:none"'} />
          </div>
          ${
            i === 0
              ? `<button type="button" class="multi-add-btn" aria-label="Add another">+</button>`
              : `<button type="button" class="multi-remove-btn" aria-label="Remove">🗑</button>`
          }
        </div>`;
          },
        )
        .join("")}
    </div>`;
}

function updateLabelVisibility(fieldGroup: HTMLElement): void {
  const rows = fieldGroup.querySelectorAll<HTMLElement>(".multi-field-row");
  const show = rows.length > 1;
  const defaultName = fieldGroup.dataset.defaultName ?? "";
  rows.forEach((row, i) => {
    const labelInput = row.querySelector<HTMLInputElement>(".multi-field-label");
    if (!labelInput) return;
    labelInput.style.display = show ? "" : "none";
    if (show && !labelInput.value.trim()) {
      labelInput.value = `${defaultName} ${i + 1}`;
    }
  });
}

function attachMultiFieldListeners(container: HTMLElement): void {
  container.querySelectorAll<HTMLButtonElement>(".multi-add-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const fieldGroup = btn.closest<HTMLElement>(".multi-field")!;
      const firstUrl = fieldGroup.querySelector<HTMLInputElement>(`input[name$="_url"]`)!;
      const defaultName = fieldGroup.dataset.defaultName ?? "";
      const newIndex = fieldGroup.querySelectorAll(".multi-field-row").length + 1;
      const newRow = document.createElement("div");
      newRow.className = "multi-field-row";
      newRow.innerHTML = `
        <div class="multi-field-inputs">
          <input type="${firstUrl.type}" name="${firstUrl.name}" placeholder="${firstUrl.placeholder}" />
          <input type="text" name="${firstUrl.name.replace("_url", "_label")}" class="multi-field-label" placeholder="Label" value="${escHtml(`${defaultName} ${newIndex}`)}" />
        </div>
        <button type="button" class="multi-remove-btn" aria-label="Remove">🗑</button>`;
      const rows = fieldGroup.querySelectorAll(".multi-field-row");
      rows[rows.length - 1].insertAdjacentElement("afterend", newRow);
      updateLabelVisibility(fieldGroup);
      newRow.querySelector<HTMLButtonElement>(".multi-remove-btn")!.addEventListener("click", () => {
        newRow.remove();
        updateLabelVisibility(fieldGroup);
      });
    });
  });

  container.querySelectorAll<HTMLButtonElement>(".multi-remove-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const fieldGroup = btn.closest<HTMLElement>(".multi-field")!;
      btn.closest(".multi-field-row")!.remove();
      updateLabelVisibility(fieldGroup);
    });
  });
}

function collectMultiField(container: HTMLElement, fieldName: string): LinkEntry[] {
  return Array.from(
    container.querySelectorAll<HTMLElement>(`.multi-field[data-field="${fieldName}"] .multi-field-row`),
  )
    .map((row) => ({
      url: (row.querySelector<HTMLInputElement>(`input[name="${fieldName}_url"]`)?.value ?? "").trim(),
      label: (row.querySelector<HTMLInputElement>(`input[name="${fieldName}_label"]`)?.value ?? "").trim(),
    }))
    .filter((e) => e.url.length > 0);
}

function formatTimerInit(id: string): string {
  // Render initial display without importing full timer logic into template strings.
  // The real display is set by initTimerCard immediately after render.
  try {
    const raw = localStorage.getItem("timer_" + id);
    if (raw) {
      const s = JSON.parse(raw);
      const elapsed = s.startedAt !== null
        ? s.elapsedSecs + (Date.now() - s.startedAt) / 1000
        : s.elapsedSecs;
      const remaining = s.durationSecs - elapsed;
      const overtime = remaining < 0;
      const abs = Math.abs(Math.floor(remaining));
      const mins = Math.floor(abs / 60);
      const secs = abs % 60;
      return `${overtime ? "+" : ""}${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
    }
  } catch { /* ignore */ }
  const mins = getDefaultRoundTime();
  return `${String(mins).padStart(2, "0")}:00`;
}

// ── Pages ─────────────────────────────────────────────────────────────────────

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
          <label for="event-software">Event Software <span class="label-optional">optional</span></label>
          <input type="url" id="event-software" name="event_software" placeholder="https://" />
        </div>
        <div class="form-group">
          <label for="purple-fox">
            Purple Fox
            <span class="label-optional">optional</span>
          </label>
          <input type="url" id="purple-fox" name="purple_fox" placeholder="https://" />
        </div>
        ${buildMultiField("Schedule <span class=\"label-hint\">Google Drive</span>", "schedule", [], "url", "https://docs.google.com/...", "Schedule")}
        ${buildMultiField("Tracking Sheet <span class=\"label-hint\">Google Drive</span>", "tracking_sheet", [], "url", "https://docs.google.com/...", "Tracking Sheet")}
        ${buildMultiField("Discord Channel", "discord", [], "text", "https://discord.com/channels/...", "Discord")}
        <button type="submit" class="form-submit">Create Tournament</button>
      </form>
    </div>
  `;

  attachMultiFieldListeners(container);

  container.querySelector("#new-tournament-form")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const tournament: Tournament = {
      id: crypto.randomUUID(),
      name: (form.elements.namedItem("name") as HTMLInputElement).value.trim(),
      event_software: (form.elements.namedItem("event_software") as HTMLInputElement).value || null,
      purple_fox: (form.elements.namedItem("purple_fox") as HTMLInputElement).value || null,
      schedule: collectMultiField(container, "schedule"),
      tracking_sheet: collectMultiField(container, "tracking_sheet"),
      discord: collectMultiField(container, "discord"),
      notes: null,
      created_at: new Date().toISOString(),
    };

    const tournaments = loadTournaments();
    tournaments.push(tournament);
    saveTournaments(tournaments);

    window.location.hash = "#/tournament/active";
  });
}

export function initActiveTournaments(container: HTMLElement): void {
  clearAllTimerIntervals();
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

  const entryBtn = (entry: LinkEntry, defaultLabel: string) =>
    `<button class="tournament-link" data-url="${escHtml(entry.url)}" data-title="${escHtml(entry.label || defaultLabel)}">${escHtml(entry.label || defaultLabel)}</button>`;

  const entryBtns = (entries: LinkEntry[], baseLabel: string) =>
    entries
      .map((e, i) => entryBtn(e, entries.length > 1 ? `${baseLabel} ${i + 1}` : baseLabel))
      .join("");

  const cards = tournaments.map((t) => `
    <div class="tournament-card" data-id="${escHtml(t.id)}">
      <div class="tournament-card-header">
        <div class="tournament-card-name">${escHtml(t.name)}</div>
        <button class="tournament-delete" data-id="${escHtml(t.id)}" data-name="${escHtml(t.name)}" aria-label="Delete tournament">✕</button>
      </div>
      <div class="tournament-card-links">
        ${t.event_software ? `<button class="tournament-link" data-url="${escHtml(t.event_software)}" data-title="Event Software">Event Software</button>` : ""}
        ${t.purple_fox ? `<button class="tournament-link" data-url="${escHtml(t.purple_fox)}" data-title="Purple Fox">Purple Fox</button>` : ""}
        ${entryBtns(t.schedule, "Schedule")}
        ${entryBtns(t.tracking_sheet, "Tracking Sheet")}
        ${entryBtns(t.discord, "Discord")}
      </div>
      <div class="tournament-timer">
        <div class="timer-display" title="Double-click or hold to edit">${formatTimerInit(t.id)}</div>
        <div class="timer-controls">
          ${t.purple_fox ? `<button class="timer-btn timer-sync-btn" data-url="${escHtml(t.purple_fox)}" aria-label="Sync with Purple Fox" title="Sync with Purple Fox">⟳</button>` : ""}
          <button class="timer-btn timer-start-btn" aria-label="Start timer">▶</button>
          <button class="timer-btn timer-reset-btn" aria-label="Reset timer">↺</button>
        </div>
        <div class="timer-edit-overlay" hidden>
          <div class="timer-edit-box">
            <div class="timer-edit-digits">
              <div class="timer-edit-col">
                <button class="digit-btn digit-inc" data-idx="0">+</button>
                <div class="digit-val" data-idx="0">0</div>
                <button class="digit-btn digit-dec" data-idx="0">-</button>
              </div>
              <div class="timer-edit-col">
                <button class="digit-btn digit-inc" data-idx="1">+</button>
                <div class="digit-val" data-idx="1">0</div>
                <button class="digit-btn digit-dec" data-idx="1">-</button>
              </div>
              <div class="timer-edit-sep">:</div>
              <div class="timer-edit-col">
                <button class="digit-btn digit-inc" data-idx="2">+</button>
                <div class="digit-val" data-idx="2">0</div>
                <button class="digit-btn digit-dec" data-idx="2">-</button>
              </div>
              <div class="timer-edit-col">
                <button class="digit-btn digit-inc" data-idx="3">+</button>
                <div class="digit-val" data-idx="3">0</div>
                <button class="digit-btn digit-dec" data-idx="3">-</button>
              </div>
            </div>
            <div class="timer-edit-actions">
              <button class="timer-edit-cancel">Cancel</button>
              <button class="timer-edit-ok">OK</button>
            </div>
          </div>
        </div>
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

  // Initialise timers for each card
  tournaments.forEach((t) => {
    const card = container.querySelector<HTMLElement>(`.tournament-card[data-id="${t.id}"]`);
    if (card) initTimerCard(t.id, t.name, card);
  });

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
        deleteTimerState(id);
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
          <label for="event-software">Event Software <span class="label-optional">optional</span></label>
          <input type="url" id="event-software" name="event_software" placeholder="https://" value="${escHtml(tournament.event_software ?? "")}" />
        </div>
        <div class="form-group">
          <label for="purple-fox">
            Purple Fox
            <span class="label-optional">optional</span>
          </label>
          <input type="url" id="purple-fox" name="purple_fox" placeholder="https://" value="${escHtml(tournament.purple_fox ?? "")}" />
        </div>
        ${buildMultiField("Schedule <span class=\"label-hint\">Google Drive</span>", "schedule", tournament.schedule, "url", "https://docs.google.com/...", "Schedule")}
        ${buildMultiField("Tracking Sheet <span class=\"label-hint\">Google Drive</span>", "tracking_sheet", tournament.tracking_sheet, "url", "https://docs.google.com/...", "Tracking Sheet")}
        ${buildMultiField("Discord Channel", "discord", tournament.discord, "text", "https://discord.com/channels/...", "Discord")}
        <button type="submit" class="form-submit">Save</button>
      </form>
    </div>
  `;

  attachMultiFieldListeners(container);

  container.querySelector("#edit-tournament-form")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    const updated: Tournament = {
      ...tournament,
      name: (form.elements.namedItem("name") as HTMLInputElement).value.trim(),
      event_software: (form.elements.namedItem("event_software") as HTMLInputElement).value || null,
      purple_fox: (form.elements.namedItem("purple_fox") as HTMLInputElement).value || null,
      schedule: collectMultiField(container, "schedule"),
      tracking_sheet: collectMultiField(container, "tracking_sheet"),
      discord: collectMultiField(container, "discord"),
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

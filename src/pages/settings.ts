import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { initUpdatesSection } from "./updates.js";
import {
  getTheme, setTheme, type Theme,
  getAccent, setAccent, ACCENT_COLORS,
  getFontSize, setFontSize, type FontSize,
  getPackSize, setPackSize, type PackSize,
  getGame, setGame, type Game,
} from "../theme.js";
import {
  getDefaultRoundTime, setDefaultRoundTime,
  getAlarmSound, setAlarmSound, playAlarm, ALARM_SOUND_OPTIONS,
} from "./timer.js";

export function initSettingsPage(container: HTMLElement): void {
  const currentTheme = getTheme();
  const currentAccent = getAccent();
  const currentFontSize = getFontSize();
  const currentPackSize = getPackSize();
  const currentGame = getGame();

  container.innerHTML = `
    <div class="settings-page">
      <h1>Settings</h1>

      <div class="settings-section">
        <div class="settings-section-title">Game</div>
        <div class="theme-options">
          ${(["mtg", "riftbound"] as Game[]).map((g) => `
            <button class="theme-option ${currentGame === g ? "theme-option--active" : ""}" data-game="${g}"${currentGame === g ? ` style="--active-color:${currentAccent.value}"` : ""}>
              <span class="theme-option-label">${g === "mtg" ? "Magic: The Gathering" : "Riftbound"}</span>
            </button>
          `).join("")}
        </div>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Appearance</div>
        <div class="theme-options">
          ${(["dark", "light", "system"] as Theme[]).map((t) => `
            <button class="theme-option ${currentTheme === t ? "theme-option--active" : ""}" data-theme="${t}"${currentTheme === t ? ` style="--active-color:${currentAccent.value}"` : ""}>
              <span class="theme-option-icon">${themeIcon(t)}</span>
              <span class="theme-option-label">${t.charAt(0).toUpperCase() + t.slice(1)}</span>
            </button>
          `).join("")}
        </div>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Accent Color</div>
        <div class="accent-options">
          ${ACCENT_COLORS.map((c) => `
            <button
              class="accent-swatch ${currentAccent.value === c.value ? "accent-swatch--active" : ""}"
              data-accent="${c.value}"
              title="${c.label}"
              style="--swatch: ${c.value}"
            ></button>
          `).join("")}
        </div>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Font Size</div>
        <div class="theme-options">
          ${(["small", "medium", "large"] as FontSize[]).map((s) => `
            <button class="theme-option ${currentFontSize === s ? "theme-option--active" : ""}" data-font-size="${s}"${currentFontSize === s ? ` style="--active-color:${currentAccent.value}"` : ""}>
              <span class="font-size-preview font-size-preview--${s}">Aa</span>
              <span class="theme-option-label">${s.charAt(0).toUpperCase() + s.slice(1)}</span>
            </button>
          `).join("")}
        </div>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Draft</div>
        <div class="settings-row">
          <span class="settings-row-label">Pack size</span>
          <div class="theme-options">
            ${([14, 15] as PackSize[]).map((s) => `
              <button class="theme-option ${currentPackSize === s ? "theme-option--active" : ""}" data-pack-size="${s}"${currentPackSize === s ? ` style="--active-color:${currentAccent.value}"` : ""}>
                <span class="theme-option-label">${s} cards</span>
              </button>
            `).join("")}
          </div>
        </div>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Tournament</div>
        <div class="settings-row">
          <label class="settings-row-label" for="default-round-time">Default round time</label>
          <div class="settings-number-wrap">
            <input type="number" id="default-round-time" class="settings-number-input" min="1" max="180" value="${getDefaultRoundTime()}" />
            <span class="settings-number-unit">min</span>
          </div>
        </div>
        <div class="settings-row">
          <label class="settings-row-label" for="alarm-sound">Alarm sound</label>
          <div class="settings-alarm-wrap">
            <select id="alarm-sound" class="settings-select">
              ${ALARM_SOUND_OPTIONS.map(o => `<option value="${o.value}"${getAlarmSound() === o.value ? " selected" : ""}>${o.label}</option>`).join("")}
            </select>
            <button id="alarm-preview-btn" class="settings-preview-btn">Preview</button>
          </div>
        </div>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Data Updates</div>
        <div id="settings-updates-container"></div>
      </div>

      <div class="settings-section settings-about">
        <div class="settings-section-title">About</div>
        <div class="about-app-name">The Judge App <span id="about-version" class="about-version"></span></div>
        <p class="about-description">An offline-first companion for Magic: The Gathering judges. CR, MTR, IPG, card search, deck counting, and tournament tools at your fingertips.</p>
        <p class="about-contact">Questions or feedback? Find me on Discord: <span class="about-discord">diracdeltafunct</span></p>
        <p class="about-thanks">Special thanks to the Azorius Senate for testing and design input.</p>
        <button class="tip-btn" id="about-kofi-btn">Support development on Ko-fi</button>
      </div>

      <div class="settings-section">
        <div class="settings-section-title">Release Notes</div>
        <pre id="settings-release-notes" class="release-notes-text"></pre>
      </div>
    </div>
  `;

  function setActiveColor(btn: HTMLElement): void {
    btn.style.setProperty("--active-color", getAccent().value);
  }
  function clearActiveColor(btn: Element): void {
    (btn as HTMLElement).style.removeProperty("--active-color");
  }
  function refreshAllActiveColors(): void {
    container.querySelectorAll<HTMLElement>(".theme-option--active").forEach(setActiveColor);
  }

  // Theme
  container.querySelectorAll<HTMLButtonElement>(".theme-option[data-theme]").forEach((btn) => {
    btn.addEventListener("click", () => {
      setTheme(btn.dataset.theme as Theme);
      container.querySelectorAll(".theme-option[data-theme]").forEach((b) => {
        b.classList.toggle("theme-option--active", b === btn);
        if (b === btn) setActiveColor(btn); else clearActiveColor(b);
      });
    });
  });

  // Accent
  container.querySelectorAll<HTMLButtonElement>(".accent-swatch").forEach((btn) => {
    btn.addEventListener("click", () => {
      const color = ACCENT_COLORS.find((c) => c.value === btn.dataset.accent)!;
      setAccent(color);
      container.querySelectorAll(".accent-swatch").forEach((b) =>
        b.classList.toggle("accent-swatch--active", b === btn),
      );
      refreshAllActiveColors();
    });
  });

  // Font size
  container.querySelectorAll<HTMLButtonElement>(".theme-option[data-font-size]").forEach((btn) => {
    btn.addEventListener("click", () => {
      setFontSize(btn.dataset.fontSize as FontSize);
      container.querySelectorAll(".theme-option[data-font-size]").forEach((b) => {
        b.classList.toggle("theme-option--active", b === btn);
        if (b === btn) setActiveColor(btn); else clearActiveColor(b);
      });
    });
  });

  // Pack size
  container.querySelectorAll<HTMLButtonElement>(".theme-option[data-pack-size]").forEach((btn) => {
    btn.addEventListener("click", () => {
      setPackSize(Number(btn.dataset.packSize) as PackSize);
      container.querySelectorAll(".theme-option[data-pack-size]").forEach((b) => {
        b.classList.toggle("theme-option--active", b === btn);
        if (b === btn) setActiveColor(btn); else clearActiveColor(b);
      });
    });
  });

  // Game
  container.querySelectorAll<HTMLButtonElement>(".theme-option[data-game]").forEach((btn) => {
    btn.addEventListener("click", () => {
      setGame(btn.dataset.game as Game);
      container.querySelectorAll(".theme-option[data-game]").forEach((b) => {
        b.classList.toggle("theme-option--active", b === btn);
        if (b === btn) setActiveColor(btn); else clearActiveColor(b);
      });
      window.dispatchEvent(new CustomEvent("game-changed"));
    });
  });

  // Default round time
  container.querySelector<HTMLInputElement>("#default-round-time")?.addEventListener("change", (e) => {
    const val = parseInt((e.target as HTMLInputElement).value, 10);
    if (!isNaN(val) && val >= 1) setDefaultRoundTime(val);
  });

  // Alarm sound
  container.querySelector<HTMLSelectElement>("#alarm-sound")?.addEventListener("change", (e) => {
    setAlarmSound((e.target as HTMLSelectElement).value as any);
  });
  container.querySelector<HTMLButtonElement>("#alarm-preview-btn")?.addEventListener("click", () => {
    playAlarm();
  });

  // Ko-fi
  container.querySelector("#about-kofi-btn")?.addEventListener("click", () => {
    invoke("open_custom_tab", { url: "https://ko-fi.com/thejudgeapp" });
  });

  // Version (async, fills in when ready)
  getVersion().then((v) => {
    const el = container.querySelector("#about-version");
    if (el) el.textContent = `v${v}`;
  }).catch(() => {});

  // Release notes
  invoke<string>("get_release_notes").then((notes) => {
    const el = container.querySelector("#settings-release-notes");
    if (el) el.textContent = notes;
  }).catch(() => {});

  initUpdatesSection(
    container.querySelector<HTMLElement>("#settings-updates-container")!,
  );
}

function themeIcon(theme: Theme): string {
  switch (theme) {
    case "dark":
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>`;
    case "light":
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>`;
    case "system":
      return `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>`;
  }
}

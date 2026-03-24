import { invoke } from "@tauri-apps/api/core";

interface CardResult {
  name: string;
  oracle_text: string | null;
  mana_cost: string | null;
  type_line: string | null;
  set_code: string | null;
  set_name: string | null;
  colors: string | null;
  legalities: string | null;
  image_url: string | null;
}

interface Ruling {
  source: string | null;
  published_at: string | null;
  comment: string;
}

interface CardDetail extends CardResult {
  rulings: Ruling[];
}

interface SetInfo {
  code: string;
  name: string;
}

const COLOR_OPTIONS = [
  { code: "W", name: "White" },
  { code: "U", name: "Blue" },
  { code: "B", name: "Black" },
  { code: "R", name: "Red" },
  { code: "G", name: "Green" },
];

function getSavedColors(): string[] {
  try { return JSON.parse(sessionStorage.getItem("card-search-colors") || "[]"); } catch { return []; }
}

export function initCardSearch(container: HTMLElement): void {
  const input = container.querySelector<HTMLInputElement>("#card-search");
  const results = container.querySelector<HTMLDivElement>("#card-results");
  const clearBtn = container.querySelector<HTMLButtonElement>("#card-search-clear");
  const colorFilter = container.querySelector<HTMLElement>("#color-filter")!;
  const colorAddBtn = container.querySelector<HTMLButtonElement>("#color-add-btn")!;
  const colorDropdown = container.querySelector<HTMLElement>("#color-dropdown")!;
  const mvInput = container.querySelector<HTMLInputElement>("#mv-input")!;
  const mvOp = container.querySelector<HTMLSelectElement>("#mv-op")!;
  const mvClear = container.querySelector<HTMLButtonElement>("#mv-clear")!;
  const setInput = container.querySelector<HTMLInputElement>("#set-input")!;
  const setClear = container.querySelector<HTMLButtonElement>("#set-clear")!;
  const setDropdown = container.querySelector<HTMLElement>("#set-dropdown")!;
  const setFilterEl = container.querySelector<HTMLElement>("#set-filter")!;
  if (!input || !results) return;

  const selectedColors: string[] = getSavedColors();
  let allSets: SetInfo[] = [];
  // selectedSet tracks the confirmed set code (null = no filter active)
  let selectedSet: string | null = sessionStorage.getItem("card-search-set");
  // Prevents the pending debounced input callback from clearing a just-confirmed selection
  let setJustSelected = false;

  const getManaFilter = () => ({
    manaValue: mvInput.value !== "" ? parseInt(mvInput.value, 10) : null,
    manaOp: mvOp.value,
  });

  const triggerSearch = () => {
    const { manaValue, manaOp } = getManaFilter();
    handleSearch(input, results, selectedColors, manaValue, manaOp, selectedSet);
  };

  const updateClear = () => {
    clearBtn?.classList.toggle("hidden", input.value === "");
  };

  // Restore saved set input display value
  const savedSetDisplay = sessionStorage.getItem("card-search-set-display");
  if (savedSetDisplay) {
    setInput.value = savedSetDisplay;
    setClear.classList.remove("hidden");
  }

  // Restore saved mana value
  const savedMv = sessionStorage.getItem("card-search-mv");
  const savedMvOp = sessionStorage.getItem("card-search-mv-op");
  if (savedMv !== null) {
    mvInput.value = savedMv;
    mvClear.classList.remove("hidden");
  }
  if (savedMvOp) mvOp.value = savedMvOp;

  // Restore saved colors
  selectedColors.forEach((code) => addColorChip(code, colorFilter, colorDropdown, triggerSearch));
  updateColorDropdown(colorDropdown, selectedColors);

  // Restore saved query
  const saved = sessionStorage.getItem("card-search-query");
  if (saved) {
    input.value = saved;
    updateClear();
    triggerSearch();
  }

  input.addEventListener("input", debounce(() => {
    sessionStorage.setItem("card-search-query", input.value);
    updateClear();
    triggerSearch();
  }, 250));

  clearBtn?.addEventListener("click", () => {
    input.value = "";
    sessionStorage.removeItem("card-search-query");
    results.innerHTML = "";
    clearBtn.classList.add("hidden");
    input.focus();
    triggerSearch();
  });

  // Mana value filter
  mvInput.addEventListener("input", debounce(() => {
    sessionStorage.setItem("card-search-mv", mvInput.value);
    mvClear.classList.toggle("hidden", mvInput.value === "");
    triggerSearch();
  }, 300));

  mvOp.addEventListener("change", () => {
    sessionStorage.setItem("card-search-mv-op", mvOp.value);
    if (mvInput.value !== "") triggerSearch();
  });

  mvClear.addEventListener("click", () => {
    mvInput.value = "";
    sessionStorage.removeItem("card-search-mv");
    mvClear.classList.add("hidden");
    triggerSearch();
  });

  // Set filter — load sets lazily on first focus
  const loadSets = async () => {
    if (allSets.length === 0) {
      try { allSets = await invoke<SetInfo[]>("get_sets"); } catch { allSets = []; }
    }
  };

  const renderSetDropdown = (matches: SetInfo[]) => {
    if (matches.length === 0) { setDropdown.classList.add("hidden"); return; }
    setDropdown.innerHTML = matches.slice(0, 8).map((s) =>
      `<button class="set-opt" data-code="${escHtml(s.code)}">${escHtml(s.name)} <span class="set-opt-code">${escHtml(s.code)}</span></button>`
    ).join("");
    setDropdown.classList.remove("hidden");
    setDropdown.querySelectorAll<HTMLButtonElement>(".set-opt").forEach((btn) => {
      btn.addEventListener("mousedown", (e) => {
        e.preventDefault(); // keep focus on input
        const code = btn.dataset.code!;
        const name = allSets.find((s) => s.code === code)?.name ?? code;
        setInput.value = name;
        setClear.classList.remove("hidden");
        setDropdown.classList.add("hidden");
        selectedSet = code;
        setJustSelected = true;
        sessionStorage.setItem("card-search-set", code);
        sessionStorage.setItem("card-search-set-display", name);
        triggerSearch();
      });
    });
  };

  setInput.addEventListener("focus", loadSets);

  setInput.addEventListener("input", debounce(async () => {
    if (setJustSelected) { setJustSelected = false; return; }
    await loadSets();
    const q = setInput.value.trim().toLowerCase();
    selectedSet = null;
    sessionStorage.removeItem("card-search-set");
    setClear.classList.toggle("hidden", setInput.value === "");
    if (!q) { setDropdown.classList.add("hidden"); triggerSearch(); return; }
    const matches = allSets.filter((s) =>
      s.name.toLowerCase().includes(q) || s.code.toLowerCase().includes(q)
    );
    renderSetDropdown(matches);
    triggerSearch();
  }, 200));

  setInput.addEventListener("blur", () => {
    setTimeout(() => setDropdown.classList.add("hidden"), 150);
  });

  setClear.addEventListener("click", () => {
    setInput.value = "";
    selectedSet = null;
    sessionStorage.removeItem("card-search-set");
    sessionStorage.removeItem("card-search-set-display");
    setClear.classList.add("hidden");
    setDropdown.classList.add("hidden");
    triggerSearch();
  });

  document.addEventListener("click", (e) => {
    if (!setFilterEl.isConnected) return;
    if (!setFilterEl.contains(e.target as Node)) setDropdown.classList.add("hidden");
  });

  // Toggle color dropdown
  colorAddBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    colorDropdown.classList.toggle("hidden");
  });

  // Select a color from the dropdown
  colorDropdown.querySelectorAll<HTMLButtonElement>(".color-opt").forEach((btn) => {
    btn.addEventListener("click", () => {
      const code = btn.dataset.color!;
      if (!selectedColors.includes(code)) {
        selectedColors.push(code);
        sessionStorage.setItem("card-search-colors", JSON.stringify(selectedColors));
        addColorChip(code, colorFilter, colorDropdown, triggerSearch);
        updateColorDropdown(colorDropdown, selectedColors);
        triggerSearch();
      }
      colorDropdown.classList.add("hidden");
    });
  });

  // Close dropdown when clicking outside
  document.addEventListener("click", (e) => {
    if (!colorFilter.isConnected) return;
    if (!colorFilter.contains(e.target as Node)) {
      colorDropdown.classList.add("hidden");
    }
  });
}

function addColorChip(
  code: string,
  container: HTMLElement,
  dropdown: HTMLElement,
  onRemove: () => void,
): void {
  const colorName = COLOR_OPTIONS.find((c) => c.code === code)?.name ?? code;
  const chip = document.createElement("span");
  chip.className = "color-chip";
  chip.dataset.color = code;
  chip.innerHTML = `${colorName} <button class="chip-remove" aria-label="Remove ${colorName}">×</button>`;

  const addWrap = container.querySelector(".color-add-wrap")!;
  container.insertBefore(chip, addWrap);

  chip.querySelector<HTMLButtonElement>(".chip-remove")!.addEventListener("click", () => {
    chip.remove();
    const saved = getSavedColors().filter((c) => c !== code);
    sessionStorage.setItem("card-search-colors", JSON.stringify(saved));
    updateColorDropdown(dropdown, saved);
    onRemove();
  });
}

function updateColorDropdown(dropdown: HTMLElement, selected: string[]): void {
  dropdown.querySelectorAll<HTMLButtonElement>(".color-opt").forEach((btn) => {
    btn.disabled = selected.includes(btn.dataset.color!);
  });
}

async function handleSearch(
  input: HTMLInputElement,
  results: HTMLDivElement,
  colors: string[],
  manaValue: number | null,
  manaOp: string,
  set: string | null,
): Promise<void> {
  const query = input.value.trim();
  if (query.length < 2 && colors.length === 0 && manaValue === null && !set) {
    results.innerHTML = "";
    return;
  }

  try {
    const cards = await invoke<CardResult[]>("search_cards", {
      query,
      colors,
      manaValue,
      manaOp: manaValue !== null ? manaOp : null,
      set: set ?? null,
    });
    renderCards(results, cards);
  } catch (e) {
    results.innerHTML = `<p class="empty-state">Failed to search: ${e}</p>`;
  }
}

function renderCards(target: HTMLElement, cards: CardResult[]): void {
  if (!cards.length) {
    target.innerHTML = `<p class="empty-state">No cards found.</p>`;
    return;
  }

  target.innerHTML = cards
    .map((card) => {
      const setInfo = card.set_name || card.set_code ? `${card.set_name ?? ""} ${card.set_code ? `(${card.set_code})` : ""}`.trim() : "";
      const colors = formatColors(card.colors);
      const href = `#/card/${encodeURIComponent(card.name)}`;
      return `
        <a class="card-result" href="${href}">
          <div class="card-meta">
            <div class="card-title">
              <span class="card-name">${escHtml(card.name)}</span>
              ${card.mana_cost ? `<span class="card-mana">${escHtml(card.mana_cost)}</span>` : ""}
            </div>
            ${card.type_line ? `<div class="card-type">${escHtml(card.type_line)}</div>` : ""}
            ${setInfo ? `<div class="card-set">${escHtml(setInfo)}</div>` : ""}
            ${colors ? `<div class="card-colors">${colors}</div>` : ""}
            ${card.oracle_text ? `<div class="card-text">${escHtml(card.oracle_text)}</div>` : ""}
          </div>
        </a>
      `;
    })
    .join("");
}

export async function initCardDetail(container: HTMLElement, name: string): Promise<void> {
  container.innerHTML = `<p class="empty-state">Loading...</p>`;

  try {
    const card = await invoke<CardDetail | null>("get_card", { name });
    if (!card) {
      container.innerHTML = `<p class="empty-state">Card not found.</p>`;
      return;
    }
    renderCardDetail(container, card);
  } catch (e) {
    container.innerHTML = `<p class="empty-state">Failed to load card: ${e}</p>`;
  }
}

let activeLegalityTip: HTMLDivElement | null = null;

function removeLegalityTip(): void {
  activeLegalityTip?.remove();
  activeLegalityTip = null;
}

function renderCardDetail(container: HTMLElement, card: CardDetail): void {
  const setInfo = card.set_name || card.set_code
    ? `${card.set_name ?? ""} ${card.set_code ? `(${card.set_code})` : ""}`.trim()
    : "";
  const colors = formatColors(card.colors);
  const legalities = formatLegalities(card.legalities);

  const rulingRows = card.rulings.length > 0
    ? card.rulings.map((r) => `
      <div class="ruling">
        ${r.published_at ? `<div class="ruling-date">${escHtml(r.published_at)}</div>` : ""}
        <div class="ruling-text">${escHtml(r.comment)}</div>
      </div>
    `).join("")
    : `<div class="ruling ruling-empty">No rulings available for this card.</div>`;

  container.innerHTML = `
    <div class="card-detail">
      <a href="#/cards" class="back-link">← Back to search</a>
      <div class="card-detail-header">
        ${card.image_url ? `<div class="card-image-slot"><button class="load-image-btn" data-url="${escHtml(card.image_url)}" data-name="${escHtml(card.name)}">Load image</button></div>` : ""}
        <div class="card-detail-info">
          <div class="card-title">
            <span class="card-name">${escHtml(card.name)}</span>
            ${card.mana_cost ? `<span class="card-mana">${escHtml(card.mana_cost)}</span>` : ""}
          </div>
          ${card.type_line ? `<div class="card-type">${escHtml(card.type_line)}</div>` : ""}
          ${setInfo ? `<div class="card-set">${escHtml(setInfo)}</div>` : ""}
          ${colors ? `<div class="card-colors">${colors}</div>` : ""}
          ${card.oracle_text ? `<div class="card-oracle-text">${escHtml(card.oracle_text)}</div>` : ""}
          ${legalities ? `<hr class="card-section-divider" /><div class="card-legalities"><h3>Legalities</h3>${legalities}</div>` : ""}
        </div>
      </div>
      <div class="card-rulings-section"><hr class="card-section-divider" /><h3 class="card-rulings-heading">Rulings</h3><div class="card-rulings">${rulingRows}</div></div>
    </div>
  `;

  container.querySelector<HTMLButtonElement>(".load-image-btn")?.addEventListener("click", (e) => {
    const btn = e.currentTarget as HTMLButtonElement;
    const slot = btn.parentElement!;
    const img = document.createElement("img");
    img.src = btn.dataset.url!;
    img.alt = btn.dataset.name!;
    img.className = "card-detail-image";
    slot.replaceChildren(img);
  });

  const statusLabels: Record<string, string> = {
    legal: "Legal",
    not_legal: "Not Legal",
    banned: "Banned",
    restricted: "Restricted",
  };

  container.querySelectorAll<HTMLElement>(".legality-tag").forEach((tag) => {
    tag.addEventListener("click", (e) => {
      e.stopPropagation();
      removeLegalityTip();
      const status = tag.dataset.status ?? "";
      const tip = document.createElement("div");
      tip.className = "legality-tooltip";
      tip.textContent = statusLabels[status] ?? status;
      document.body.appendChild(tip);
      activeLegalityTip = tip;
      const r = tag.getBoundingClientRect();
      tip.style.left = `${r.left + r.width / 2 - tip.offsetWidth / 2}px`;
      tip.style.top = `${r.top - tip.offsetHeight - 6}px`;
      document.addEventListener("click", removeLegalityTip, { once: true });
    });
  });
}

function formatLegalities(legalitiesJson: string | null): string {
  if (!legalitiesJson) return "";
  try {
    const parsed = JSON.parse(legalitiesJson) as Record<string, string> | [string, string][];
    const entries: [string, string][] = Array.isArray(parsed)
      ? parsed as [string, string][]
      : Object.entries(parsed) as [string, string][];
    if (!entries.length) return "";
    return `<div class="legalities-grid">${entries.map(([format, status]) =>
      `<span class="legality-tag legality-${escHtml(status.replace(/_/g, "-"))}" data-status="${escHtml(status)}">${escHtml(format)}</span>`
    ).join("")}</div>`;
  } catch {
    return "";
  }
}

function formatColors(colorsJson: string | null): string {
  if (!colorsJson) return "";
  try {
    const colors = JSON.parse(colorsJson) as string[];
    if (!Array.isArray(colors) || colors.length === 0) return "";
    return colors.map((c) => c.toUpperCase()).join(" ");
  } catch {
    return "";
  }
}

function escHtml(str: string): string {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function debounce<T extends unknown[]>(
  fn: (...args: T) => void,
  ms: number,
): (...args: T) => void {
  let timer: ReturnType<typeof setTimeout>;
  return (...args: T) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

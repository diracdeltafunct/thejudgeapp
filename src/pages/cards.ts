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

export function initCardSearch(container: HTMLElement): void {
  const input = container.querySelector<HTMLInputElement>("#card-search");
  const results = container.querySelector<HTMLDivElement>("#card-results");
  if (!input || !results) return;

  const saved = sessionStorage.getItem("card-search-query");
  if (saved) {
    input.value = saved;
    handleSearch(input, results);
  }

  input.addEventListener("input", debounce(() => {
    sessionStorage.setItem("card-search-query", input.value);
    handleSearch(input, results);
  }, 250));
}

async function handleSearch(
  input: HTMLInputElement,
  results: HTMLDivElement,
): Promise<void> {
  const query = input.value.trim();
  if (query.length < 2) {
    results.innerHTML = "";
    return;
  }

  try {
    const cards = await invoke<CardResult[]>("search_cards", { query });
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
          ${legalities ? `<div class="card-legalities"><h3>Legalities</h3>${legalities}</div>` : ""}
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
}

function formatLegalities(legalitiesJson: string | null): string {
  if (!legalitiesJson) return "";
  try {
    const map = JSON.parse(legalitiesJson) as Record<string, string>;
    const entries = Object.entries(map).filter(([, v]) => v === "legal");
    if (!entries.length) return "";
    return `<div class="legalities-grid">${entries.map(([format]) =>
      `<span class="legality-tag">${escHtml(format)}</span>`
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

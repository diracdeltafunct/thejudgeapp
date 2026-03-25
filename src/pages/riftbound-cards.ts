import { invoke } from "@tauri-apps/api/core";
import { replaceRbIcons } from "../rb-icons.js";

interface RiftboundCardResult {
  id: string;
  name: string;
  card_type: string | null;
  card_set: string | null;
  rarity: string | null;
  domain: string | null;
  energy: number | null;
}

interface RiftboundCardDetail {
  id: string;
  name: string;
  energy: number | null;
  might: number | null;
  power: number | null;
  domain: string | null;
  card_type: string | null;
  rarity: string | null;
  card_set: string | null;
  collector_number: number | null;
  image_url: string | null;
  ability: string | null;
  errata_text: string | null;
  errata_old_text: string | null;
}

interface Filters {
  query: string;
  cardType: string;
  cardSet: string;
  rarity: string;
  domain: string;
  errata: string;
  energyMin: string;
  energyMax: string;
  powerMin: string;
  powerMax: string;
}

let searchTimeout: ReturnType<typeof setTimeout> | null = null;

function saveFilters(f: Filters): void {
  sessionStorage.setItem("rb-filters", JSON.stringify(f));
}

function loadFilters(): Filters {
  try {
    return JSON.parse(sessionStorage.getItem("rb-filters") ?? "{}");
  } catch {
    return {} as Filters;
  }
}

function readFilters(container: HTMLElement): Filters {
  return {
    query: (container.querySelector<HTMLInputElement>("#rb-card-search")?.value ?? ""),
    cardType: (container.querySelector<HTMLSelectElement>("#rb-filter-type")?.value ?? ""),
    cardSet: (container.querySelector<HTMLSelectElement>("#rb-filter-set")?.value ?? ""),
    rarity: (container.querySelector<HTMLSelectElement>("#rb-filter-rarity")?.value ?? ""),
    domain: (container.querySelector<HTMLSelectElement>("#rb-filter-domain")?.value ?? ""),
    errata: (container.querySelector<HTMLSelectElement>("#rb-filter-errata")?.value ?? ""),
    energyMin: (container.querySelector<HTMLInputElement>("#rb-energy-min")?.value ?? ""),
    energyMax: (container.querySelector<HTMLInputElement>("#rb-energy-max")?.value ?? ""),
    powerMin: (container.querySelector<HTMLInputElement>("#rb-power-min")?.value ?? ""),
    powerMax: (container.querySelector<HTMLInputElement>("#rb-power-max")?.value ?? ""),
  };
}

function restoreFilters(container: HTMLElement, f: Filters): void {
  const set = (id: string, val: string | undefined) => {
    if (!val) return;
    const el = container.querySelector<HTMLInputElement | HTMLSelectElement>(id);
    if (el) el.value = val;
  };
  set("#rb-card-search", f.query);
  set("#rb-filter-type", f.cardType);
  set("#rb-filter-set", f.cardSet);
  set("#rb-filter-rarity", f.rarity);
  set("#rb-filter-domain", f.domain);
  set("#rb-filter-errata", f.errata);
  set("#rb-energy-min", f.energyMin);
  set("#rb-energy-max", f.energyMax);
  set("#rb-power-min", f.powerMin);
  set("#rb-power-max", f.powerMax);
}

export function initRiftboundCardSearch(container: HTMLElement): void {
  const input = container.querySelector<HTMLInputElement>("#rb-card-search");
  const results = container.querySelector<HTMLDivElement>("#rb-card-results");
  const clearBtn = container.querySelector<HTMLButtonElement>("#rb-card-search-clear");
  if (!input || !results) return;

  const saved = loadFilters();
  restoreFilters(container, saved);
  clearBtn?.classList.toggle("hidden", !input.value);

  const triggerSearch = () => {
    const f = readFilters(container);
    saveFilters(f);
    clearBtn?.classList.toggle("hidden", !f.query);
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => doSearch(f, results), 250);
  };

  input.addEventListener("input", triggerSearch);

  container.querySelectorAll<HTMLSelectElement>(".rb-select").forEach((el) => {
    el.addEventListener("change", triggerSearch);
  });
  container.querySelectorAll<HTMLInputElement>(".rb-range-input").forEach((el) => {
    el.addEventListener("input", triggerSearch);
  });

  clearBtn?.addEventListener("click", () => {
    input.value = "";
    triggerSearch();
  });

  results.addEventListener("click", (e) => {
    const row = (e.target as Element).closest<HTMLElement>("[data-name]");
    if (row?.dataset.name) {
      window.location.hash = `#/riftbound-card/${encodeURIComponent(row.dataset.name)}`;
    }
  });

  // Run search immediately if filters were saved
  const hasAny = Object.values(saved).some(Boolean);
  if (hasAny) doSearch(saved, results);
}

async function doSearch(f: Filters, results: HTMLElement): Promise<void> {
  const hasAny = Object.values(f).some(Boolean);
  if (!hasAny) {
    results.innerHTML = "";
    return;
  }

  results.innerHTML = `<div class="search-loading">Searching…</div>`;
  try {
    const cards = await invoke<RiftboundCardResult[]>("search_riftbound_cards", {
      query: f.query,
      cardType: f.cardType || null,
      cardSet: f.cardSet || null,
      rarity: f.rarity || null,
      domain: f.domain || null,
      energyMin: f.energyMin !== "" ? parseInt(f.energyMin, 10) : null,
      energyMax: f.energyMax !== "" ? parseInt(f.energyMax, 10) : null,
      powerMin: f.powerMin !== "" ? parseInt(f.powerMin, 10) : null,
      powerMax: f.powerMax !== "" ? parseInt(f.powerMax, 10) : null,
      hasErrata: f.errata !== "" ? f.errata === "true" : null,
    });
    if (cards.length === 0) {
      results.innerHTML = `<div class="search-empty">No cards found.</div>`;
      return;
    }
    results.innerHTML = cards
      .map(
        (c) => `
      <div class="card-row" data-name="${escAttr(c.name)}">
        <span class="card-name">${escHtml(c.name)}</span>
        <span class="card-meta">${[
          c.card_type,
          c.card_set,
          c.domain,
          c.energy != null ? c.energy + "E" : null,
        ]
          .filter(Boolean)
          .map(escHtml)
          .join(" · ")}</span>
      </div>`,
      )
      .join("");
  } catch (err) {
    results.innerHTML = `<div class="search-error">Error: ${escHtml(String(err))}</div>`;
  }
}

export function initRiftboundCardDetail(
  container: HTMLElement,
  name: string,
): void {
  container.innerHTML = `<div class="card-loading">Loading…</div>`;
  invoke<RiftboundCardDetail | null>("get_riftbound_card", { name })
    .then((card) => {
      if (!card) {
        container.innerHTML = `<div class="card-not-found">Card not found: ${escHtml(name)}</div>`;
        return;
      }
      renderDetail(container, card);
    })
    .catch((err) => {
      container.innerHTML = `<div class="card-error">Error: ${escHtml(String(err))}</div>`;
    });
}

function renderDetail(container: HTMLElement, card: RiftboundCardDetail): void {
  const stats: string[] = [];
  if (card.energy != null) stats.push(`Energy: ${card.energy}`);
  if (card.might != null) stats.push(`Might: ${card.might}`);
  if (card.power != null) stats.push(`Power: ${card.power}`);

  const abilityHtml = card.errata_text
    ? `<div class="card-ability">
         <div class="card-errata-badge">Errata'd</div>
         <div class="card-ability-text">${replaceRbIcons(escHtml(card.errata_text).replace(/\n/g, "<br>"))}</div>
         ${card.errata_old_text
           ? `<hr class="card-errata-divider">
              <div class="card-errata-badge card-errata-badge--original">Original</div>
              <div class="card-errata-old">${replaceRbIcons(escHtml(card.errata_old_text).replace(/\n/g, "<br>"))}</div>`
           : ""}
       </div>`
    : card.ability
      ? `<div class="card-ability">
           <div class="card-ability-text">${replaceRbIcons(escHtml(card.ability).replace(/\n/g, "<br>"))}</div>
         </div>`
      : "";

  container.innerHTML = `
    <div class="card-detail rb-card-detail">
      <button class="back-btn" id="rb-back-btn">← Back</button>
      <div class="card-detail-layout">
        ${
          card.image_url
            ? `<div class="card-image-wrap" id="rb-card-image-wrap">
                 <button class="card-image-load-btn" id="rb-load-image-btn">Load Image</button>
               </div>`
            : ""
        }
        <div class="card-detail-info">
          <h2 class="card-detail-name">${escHtml(card.name)}</h2>
          ${card.card_type ? `<div class="card-type-line">${escHtml(card.card_type)}</div>` : ""}
          ${card.domain ? `<div class="card-domain">Domain: ${escHtml(card.domain)}</div>` : ""}
          ${stats.length ? `<div class="card-stats">${stats.map(escHtml).join(" · ")}</div>` : ""}
          ${card.card_set ? `<div class="card-set-info">${escHtml(card.card_set)}${card.rarity ? " · " + escHtml(card.rarity) : ""}${card.collector_number != null ? " #" + card.collector_number : ""}</div>` : ""}
          ${abilityHtml}
        </div>
      </div>
    </div>
  `;

  container.querySelector("#rb-back-btn")?.addEventListener("click", () => {
    history.back();
  });

  if (card.image_url) {
    container.querySelector("#rb-load-image-btn")?.addEventListener("click", () => {
      const wrap = container.querySelector<HTMLElement>("#rb-card-image-wrap")!;
      wrap.innerHTML = `<img class="card-image" src="${escAttr(card.image_url!)}" alt="${escAttr(card.name)}" />`;
    });
  }
}

function escHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function escAttr(s: string): string {
  return s.replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

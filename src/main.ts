import { invoke } from "@tauri-apps/api/core";
import { initRulesViewer } from "./pages/rules-viewer.js";
import { initCardSearch, initCardDetail } from "./pages/cards.js";
import {
  initRiftboundCardSearch,
  initRiftboundCardDetail,
} from "./pages/riftbound-cards.js";
import { initDeckCounter } from "./pages/deck-counter.js";
import {
  initNewTournament,
  initActiveTournaments,
  initEditTournament,
  initTournamentNotes,
} from "./pages/tournament.js";
import { checkForUpdates } from "./pages/updates.js";
import { initSettingsPage } from "./pages/settings.js";
import { initDraftGuide } from "./pages/draft-guide.js";
import { initTournamentAlbum } from "./pages/tournament-album.js";
import { initQuickReference } from "./pages/quick-reference.js";
import {
  applyTheme,
  getTheme,
  applyAccent,
  getAccent,
  applyFontSize,
  getFontSize,
  getGame,
} from "./theme.js";

applyTheme(getTheme());
applyAccent(getAccent());
applyFontSize(getFontSize());

// Inject Android system nav bar height as a CSS variable.
// env(safe-area-inset-bottom) is unreliable in Android WebViews, so we read
// it from the native bridge (__SafeArea__) injected by MainActivity.kt.
function applyAndroidSafeArea() {
  const bridge = (window as any).__SafeArea__;
  if (bridge) {
    const px: number = bridge.getBottomInset() / window.devicePixelRatio;
    document.documentElement.style.setProperty("--safe-bottom", `${px}px`);
  }
}
document.addEventListener("DOMContentLoaded", applyAndroidSafeArea);
window.addEventListener("resize", applyAndroidSafeArea);

type DocType =
  | "cr"
  | "mtr"
  | "ipg"
  | "riftbound_cr"
  | "riftbound_tr"
  | "riftbound_ep";

const ALL_DOC_TYPES: DocType[] = [
  "cr",
  "mtr",
  "ipg",
  "riftbound_cr",
  "riftbound_tr",
  "riftbound_ep",
];

function applyGameToNav(): void {
  const game = getGame();
  document.querySelectorAll<HTMLElement>(".subnav-mtg").forEach((el) => {
    el.classList.toggle("hidden", game !== "mtg");
  });
  document.querySelectorAll<HTMLElement>(".subnav-riftbound").forEach((el) => {
    el.classList.toggle("hidden", game !== "riftbound");
  });
  // Cards nav shows for both games; update href to point at the right page
  const cardsNav = document.querySelector<HTMLAnchorElement>(
    ".nav-link[data-page='cards']",
  );
  if (cardsNav) {
    cardsNav.href = game === "riftbound" ? "#/riftbound-cards" : "#/cards";
    cardsNav.classList.remove("hidden");
  }
}

const app = document.getElementById("app")!;

const pages: Record<string, () => string> = {
  landing: () => `
    <div class="page landing-page">
      <h1 class="landing-title">The Judge App</h1>
      <p class="landing-tagline">Your companion for running Magic: The Gathering and Riftbound events.</p>
      <div class="landing-about">
        <p>TheJudgeApp is designed to be a tool to help judges run their events more efficiently. You should have access to the MTR, CR, IPG, and all oracle text for cards at the touch of your fingers. The app is designed to work offline with internet access only needed if you are requesting images or for external event software.</p>
        <p>Manage multiple tournaments with our new My Tournament manager!</p>
        <p>Reach out to me on discord (diracdeltafunct) with any feature requests or bugs.</p>
        <p>Special thanks to the Azorius Senate for testing and design input.</p>
      </div>
      <div class="landing-tip">
        <p class="tip-message">If you can spare a dollar to help support development and server costs I greatly appreciate it! The app is free for everyone regardless of your support. We all work to support our community.</p>
        <button id="kofi-tip-btn" class="tip-btn">Tip the developer</button>
      </div>
    </div>
  `,
  rules: () => `<div class="page rules-page" id="rules-container"></div>`,
  cards: () => `
    <div class="page cards-page">
      <h1>Card Search</h1>
      <div class="search-container">
        <input type="text" id="card-search" placeholder="Search by card name..." />
        <button class="search-clear hidden" id="card-search-clear" aria-label="Clear">×</button>
      </div>
      <div class="search-filters">
        <div class="color-filter" id="color-filter">
          <div class="color-add-wrap">
            <button class="color-add-btn" id="color-add-btn">+ Color ▾</button>
            <div class="color-dropdown hidden" id="color-dropdown">
              <button class="color-opt" data-color="W">White</button>
              <button class="color-opt" data-color="U">Blue</button>
              <button class="color-opt" data-color="B">Black</button>
              <button class="color-opt" data-color="R">Red</button>
              <button class="color-opt" data-color="G">Green</button>
            </div>
          </div>
        </div>
        <div class="mv-filter" id="mv-filter">
          <span class="filter-label">MV</span>
          <select class="mv-op" id="mv-op">
            <option value="eq">=</option>
            <option value="lt">&lt;</option>
            <option value="lte">&le;</option>
            <option value="gt">&gt;</option>
            <option value="gte">&ge;</option>
          </select>
          <input type="number" class="mv-input" id="mv-input" min="0" max="99" placeholder="—" />
          <button class="mv-clear hidden" id="mv-clear" aria-label="Clear mana value">×</button>
        </div>
        <div class="set-filter" id="set-filter">
          <span class="filter-label">Set</span>
          <div class="set-input-wrap">
            <input type="text" class="set-input" id="set-input" placeholder="Name or code…" autocomplete="off" spellcheck="false" />
            <button class="set-clear hidden" id="set-clear" aria-label="Clear set">×</button>
            <div class="set-dropdown hidden" id="set-dropdown"></div>
          </div>
        </div>
      </div>
      <div id="card-results" class="card-results"></div>
    </div>
  `,
  card: () =>
    `<div class="page card-detail-page" id="card-detail-container"></div>`,
  "riftbound-cards": () => `
    <div class="page cards-page">
      <h1>Card Search</h1>
      <div class="search-container">
        <input type="text" id="rb-card-search" placeholder="Search name or ability…" autocomplete="off" />
        <button class="search-clear hidden" id="rb-card-search-clear" aria-label="Clear">×</button>
      </div>
      <div class="search-filters rb-filters">
        <div class="rb-filter-row">
          <select id="rb-filter-type" class="rb-select">
            <option value="">Any Type</option>
            <option>Legend</option>
            <option>Unit</option>
            <option>Spell</option>
            <option>Gear</option>
            <option>Rune</option>
            <option>Battlefield</option>
          </select>
          <select id="rb-filter-set" class="rb-select">
            <option value="">Any Set</option>
            <option>Origins</option>
            <option>Spiritforged</option>
            <option value="Proving Grounds">Proving Grounds</option>
          </select>
          <select id="rb-filter-rarity" class="rb-select">
            <option value="">Any Rarity</option>
            <option>Common</option>
            <option>Uncommon</option>
            <option>Rare</option>
            <option>Epic</option>
            <option>Showcase</option>
          </select>
          <select id="rb-filter-domain" class="rb-select">
            <option value="">Any Domain</option>
            <option>Chaos</option>
            <option>Order</option>
            <option>Fury</option>
            <option>Calm</option>
            <option>Mind</option>
            <option>Body</option>
          </select>
          <select id="rb-filter-errata" class="rb-select">
            <option value="">Any Errata</option>
            <option value="true">Has Errata</option>
            <option value="false">No Errata</option>
          </select>
        </div>
        <div class="rb-filter-row rb-range-row">
          <span class="filter-label">Energy</span>
          <input type="number" id="rb-energy-min" class="rb-range-input" min="0" max="20" placeholder="Min" />
          <span class="rb-range-sep">–</span>
          <input type="number" id="rb-energy-max" class="rb-range-input" min="0" max="20" placeholder="Max" />
          <span class="filter-label rb-range-label">Power</span>
          <input type="number" id="rb-power-min" class="rb-range-input" min="0" max="20" placeholder="Min" />
          <span class="rb-range-sep">–</span>
          <input type="number" id="rb-power-max" class="rb-range-input" min="0" max="20" placeholder="Max" />
        </div>
      </div>
      <div id="rb-card-results" class="card-results"></div>
    </div>
  `,
  "riftbound-card": () =>
    `<div class="page card-detail-page" id="rb-card-detail-container"></div>`,
  "deck-counter": () =>
    `<div class="page deck-counter-page" id="deck-counter-container"></div>`,
  tournament: () => `<div class="page" id="tournament-container"></div>`,
  settings: () => `<div class="page" id="settings-container"></div>`,
  "draft-guide": () =>
    `<div class="page draft-guide-page" id="draft-guide-container"></div>`,
  "quick-reference": () => `<div id="quick-reference-container"></div>`,
  tools: () => `
    <div class="page tools-page">
      <h1>Tools</h1>
      <div class="tools-grid">
        <a class="tool-card" href="#/deck-counter">
          <h2>Deck Counter</h2>
          <p>Count and verify deck contents</p>
        </a>
        <a class="tool-card" href="#/draft-guide">
          <h2>Draft Calling Guide</h2>
          <p>Step-by-step draft procedure</p>
        </a>
        <a class="tool-card" href="#/quick-reference">
          <h2>Quick Reference</h2>
        </a>
      </div>
    </div>
  `,
};

let openTournamentSubnavOnNavigate = false;

function closeSubnav(): void {
  document.getElementById("rules-subnav")!.classList.add("hidden");
  document.getElementById("tournament-subnav")!.classList.add("hidden");
}

async function navigate(): Promise<void> {
  const hash = window.location.hash.slice(2) || "landing";
  const parts = hash.split("/");
  const page = parts[0];
  const subpage = parts[1]; // e.g. "cr", "mtr", "ipg"

  closeSubnav();
  if (openTournamentSubnavOnNavigate) {
    openTournamentSubnavOnNavigate = false;
    document.getElementById("tournament-subnav")!.classList.remove("hidden");
  }

  const render = pages[page] ?? pages.landing;
  app.classList.toggle("full-page", page === "draft-guide");
  app.innerHTML = render();

  // Update footer active state: rules sub-routes and card detail count as "cards"
  const activePage =
    page === "rules"
      ? "rules"
      : page === "card" ||
          page === "cards" ||
          page === "riftbound-cards" ||
          page === "riftbound-card"
        ? "cards"
        : page === "deck-counter" ||
            page === "draft-guide" ||
            page === "quick-reference"
          ? "tools"
          : page;
  document
    .querySelectorAll<HTMLElement>(".nav-link[data-page]")
    .forEach((link) => {
      link.classList.toggle("active", link.dataset.page === activePage);
    });

  // Update subnav active state
  document.querySelectorAll<HTMLElement>(".subnav-link").forEach((link) => {
    link.classList.toggle("active", link.dataset.doc === subpage);
  });

  if (page === "rules" && ALL_DOC_TYPES.includes(subpage as DocType)) {
    initRulesViewer(
      document.getElementById("rules-container")!,
      subpage as DocType,
      parts[2],
    );
  } else if (page === "cards") {
    initCardSearch(document.querySelector(".cards-page") as HTMLElement);
  } else if (page === "riftbound-cards") {
    initRiftboundCardSearch(
      document.querySelector(".cards-page") as HTMLElement,
    );
  } else if (page === "riftbound-card" && subpage) {
    initRiftboundCardDetail(
      document.getElementById("rb-card-detail-container")!,
      decodeURIComponent(subpage),
    );
  } else if (page === "deck-counter") {
    initDeckCounter(document.getElementById("deck-counter-container")!);
  } else if (page === "card" && subpage) {
    initCardDetail(
      document.getElementById("card-detail-container")!,
      decodeURIComponent(subpage),
    );
  } else if (page === "tournament") {
    const el = document.getElementById("tournament-container")!;
    if (subpage === "new") {
      initNewTournament(el);
    } else if (subpage === "edit" && parts[2]) {
      initEditTournament(el, parts[2]);
    } else if (subpage === "notes" && parts[2]) {
      initTournamentNotes(el, parts[2]);
    } else if (subpage === "album" && parts[2]) {
      const t = JSON.parse(localStorage.getItem("tournaments") ?? "[]").find(
        (x: { id: string; name: string }) => x.id === parts[2],
      );
      initTournamentAlbum(el, parts[2], t?.name ?? "Album");
    } else {
      initActiveTournaments(el);
    }
  } else if (page === "settings") {
    initSettingsPage(document.getElementById("settings-container")!);
  } else if (page === "draft-guide") {
    initDraftGuide(document.getElementById("draft-guide-container")!);
  } else if (page === "quick-reference") {
    initQuickReference(
      document.getElementById("quick-reference-container")!,
      getGame(),
    );
  }

  document.getElementById("kofi-tip-btn")?.addEventListener("click", () => {
    invoke("open_custom_tab", { url: "https://ko-fi.com/thejudgeapp" });
  });
}

function setUpdateBadge(count: number): void {
  const badge = document.getElementById("updates-badge");
  if (!badge) return;
  if (count > 0) {
    badge.textContent = String(count);
    badge.classList.remove("hidden");
  } else {
    badge.classList.add("hidden");
  }
}

// Check for updates in the background after the app loads and refresh badge
// when the user applies an update from the updates page.
async function refreshUpdateBadge(): Promise<void> {
  const count = await checkForUpdates();
  setUpdateBadge(count);
}

// Rules toggle button — open subnav so user picks which doc they want
document.getElementById("rules-toggle")!.addEventListener("click", () => {
  document.getElementById("rules-subnav")!.classList.remove("hidden");
  document.getElementById("tournament-subnav")!.classList.add("hidden");
});

// Tournament toggle button — navigate to active tournaments and show subnav
document.getElementById("tournament-toggle")!.addEventListener("click", () => {
  openTournamentSubnavOnNavigate = true;
  window.location.hash = "#/tournament/active";
});

// Subnav links — close subnav on click (navigation happens via href)
document.getElementById("rules-subnav")!.addEventListener("click", (e) => {
  if ((e.target as Element).closest(".subnav-link")) closeSubnav();
});
document.getElementById("tournament-subnav")!.addEventListener("click", (e) => {
  if ((e.target as Element).closest(".subnav-link")) closeSubnav();
});

// Close subnav when clicking outside it and the toggles
document.addEventListener("click", (e) => {
  const rulesSubnav = document.getElementById("rules-subnav")!;
  const rulesToggle = document.getElementById("rules-toggle")!;
  const tournamentSubnav = document.getElementById("tournament-subnav")!;
  const tournamentToggle = document.getElementById("tournament-toggle")!;
  if (
    !rulesSubnav.contains(e.target as Node) &&
    !rulesToggle.contains(e.target as Node) &&
    !tournamentSubnav.contains(e.target as Node) &&
    !tournamentToggle.contains(e.target as Node)
  ) {
    closeSubnav();
  }
});

// After a successful in-app update, refresh the badge
window.addEventListener("data-updated", refreshUpdateBadge);

// Re-render subnav when user switches game in settings
window.addEventListener("game-changed", () => {
  applyGameToNav();
});

window.addEventListener("hashchange", navigate);
window.addEventListener("DOMContentLoaded", () => {
  applyGameToNav();
  navigate();
});

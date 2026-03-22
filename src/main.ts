import { invoke } from "@tauri-apps/api/core";
import { initRulesViewer } from "./pages/rules-viewer.js";
import { initCardSearch, initCardDetail } from "./pages/cards.js";
import { initDeckCounter } from "./pages/deck-counter.js";
import {
  initNewTournament,
  initActiveTournaments,
  initEditTournament,
} from "./pages/tournament.js";
import { checkForUpdates } from "./pages/updates.js";
import { initSettingsPage } from "./pages/settings.js";
import { applyTheme, getTheme, applyAccent, getAccent, applyFontSize, getFontSize } from "./theme.js";

applyTheme(getTheme());
applyAccent(getAccent());
applyFontSize(getFontSize());

type DocType = "cr" | "mtr" | "ipg";

const app = document.getElementById("app")!;

const pages: Record<string, () => string> = {
  landing: () => `
    <div class="page landing-page">
      <h1 class="landing-title">The Judge App</h1>
      <p class="landing-tagline">Your companion for running Magic: The Gathering events.</p>
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
      </div>
      <div id="card-results" class="card-results"></div>
    </div>
  `,
  card: () =>
    `<div class="page card-detail-page" id="card-detail-container"></div>`,
  "deck-counter": () =>
    `<div class="page deck-counter-page" id="deck-counter-container"></div>`,
  tournament: () => `<div class="page" id="tournament-container"></div>`,
  settings: () => `<div class="page" id="settings-container"></div>`,
  tools: () => `
    <div class="page tools-page">
      <h1>Tools</h1>
      <div class="tools-grid">
        <a class="tool-card" href="#/deck-counter">
          <h2>Deck Counter</h2>
          <p>Count and verify deck contents</p>
        </a>
        <div class="tool-card">
          <h2>Draft Calling Guide</h2>
          <p>Step-by-step draft procedure</p>
        </div>
        <div class="tool-card">
          <h2>Quick Reference</h2>
          <p>Common penalties and fixes</p>
        </div>
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
  app.innerHTML = render();

  // Update footer active state: rules sub-routes and card detail count as "cards"
  const activePage =
    page === "rules"
      ? "rules"
      : page === "card"
        ? "cards"
        : page === "deck-counter"
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

  if (page === "rules" && (["cr", "mtr", "ipg"] as const).includes(subpage as DocType)) {
    initRulesViewer(document.getElementById("rules-container")!, subpage as DocType);
  } else if (page === "cards") {
    initCardSearch(document.querySelector(".cards-page") as HTMLElement);
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
    } else {
      initActiveTournaments(el);
    }
  } else if (page === "settings") {
    initSettingsPage(document.getElementById("settings-container")!);
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
  document.getElementById("rules-subnav")!.classList.toggle("hidden");
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

window.addEventListener("hashchange", navigate);
window.addEventListener("DOMContentLoaded", () => {
  navigate();
  // Background update check — runs after the page loads without blocking UI
  setTimeout(refreshUpdateBadge, 2000);
});

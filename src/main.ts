import { initRulesViewer } from "./pages/rules-viewer.js";
import { initCardSearch, initCardDetail } from "./pages/cards.js";
import { initDeckCounter } from "./pages/deck-counter.js";
import { initNewTournament, initActiveTournaments } from "./pages/tournament.js";

type DocType = "cr" | "mtr" | "ipg";

const app = document.getElementById("app")!;

const pages: Record<string, () => string> = {
  landing: () => `
    <div class="page landing-page">
      <h1 class="landing-title">The Judge App</h1>
      <p class="landing-tagline">Your companion for Magic: The Gathering competitive rules.</p>
      <div class="landing-about">
        <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.</p>
        <p>Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.</p>
        <p>Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.</p>
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

  if (page === "rules") {
    const docType: DocType = (["cr", "mtr", "ipg"] as const).includes(
      subpage as DocType,
    )
      ? (subpage as DocType)
      : "cr";
    initRulesViewer(document.getElementById("rules-container")!, docType);
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
    } else {
      initActiveTournaments(el);
    }
  }
}

// Rules toggle button — show/hide subnav, don't navigate
document.getElementById("rules-toggle")!.addEventListener("click", () => {
  document.getElementById("tournament-subnav")!.classList.add("hidden");
  document.getElementById("rules-subnav")!.classList.toggle("hidden");
});

// Tournament toggle button — show/hide subnav, don't navigate
document.getElementById("tournament-toggle")!.addEventListener("click", () => {
  document.getElementById("rules-subnav")!.classList.add("hidden");
  document.getElementById("tournament-subnav")!.classList.toggle("hidden");
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

window.addEventListener("hashchange", navigate);
window.addEventListener("DOMContentLoaded", navigate);

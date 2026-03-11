import { initRulesViewer } from "./pages/rules-viewer.js";

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
  tools: () => `
    <div class="page tools-page">
      <h1>Tools</h1>
      <div class="tools-grid">
        <div class="tool-card">
          <h2>Deck Counter</h2>
          <p>Count and verify deck contents</p>
        </div>
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
}

async function navigate(): Promise<void> {
  const hash = window.location.hash.slice(2) || "landing";
  const parts = hash.split("/");
  const page = parts[0];
  const subpage = parts[1]; // e.g. "cr", "mtr", "ipg"

  closeSubnav();

  const render = pages[page] ?? pages.landing;
  app.innerHTML = render();

  // Update footer active state: rules sub-routes count as "rules"
  const activePage = page === "rules" ? "rules" : page;
  document.querySelectorAll<HTMLElement>(".nav-link[data-page]").forEach((link) => {
    link.classList.toggle("active", link.dataset.page === activePage);
  });

  // Update subnav active state
  document.querySelectorAll<HTMLElement>(".subnav-link").forEach((link) => {
    link.classList.toggle("active", link.dataset.doc === subpage);
  });

  if (page === "rules") {
    const docType: DocType = (["cr", "mtr", "ipg"] as const).includes(subpage as DocType)
      ? (subpage as DocType)
      : "cr";
    initRulesViewer(document.getElementById("rules-container")!, docType);
  }
}

// Rules toggle button — show/hide subnav, don't navigate
document.getElementById("rules-toggle")!.addEventListener("click", () => {
  document.getElementById("rules-subnav")!.classList.toggle("hidden");
});

// Subnav links — close subnav on click (navigation happens via href)
document.getElementById("rules-subnav")!.addEventListener("click", (e) => {
  if ((e.target as Element).closest(".subnav-link")) closeSubnav();
});

// Close subnav when clicking outside it and the toggle
document.addEventListener("click", (e) => {
  const subnav = document.getElementById("rules-subnav")!;
  const toggle = document.getElementById("rules-toggle")!;
  if (!subnav.contains(e.target as Node) && !toggle.contains(e.target as Node)) {
    closeSubnav();
  }
});

window.addEventListener("hashchange", navigate);
window.addEventListener("DOMContentLoaded", navigate);

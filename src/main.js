import { invoke } from "@tauri-apps/api/core";
import { initRulesViewer } from "./pages/rules-viewer.js";

const app = document.getElementById("app");

const pages = {
  home: () => `
    <div class="page home-page">
      <h1>The Judge App</h1>
      <p class="subtitle">MTG Judge Utility Tool</p>
      <div class="home-grid">
        <a href="#/cards" class="home-card">
          <h2>Card Search</h2>
          <p>Oracle text lookup</p>
        </a>
        <a href="#/tools" class="home-card">
          <h2>Tools</h2>
          <p>Deck counter, draft guide</p>
        </a>
      </div>
      <div id="status" class="status"></div>
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

function closeSubnav() {
  document.getElementById("rules-subnav").classList.add("hidden");
}

async function navigate() {
  const hash = window.location.hash.slice(2) || "home";
  const parts = hash.split("/");
  const page = parts[0];
  const subpage = parts[1]; // e.g. "cr", "mtr", "ipg"

  closeSubnav();

  const render = pages[page] || pages.home;
  app.innerHTML = render();

  // Update footer active state: rules sub-routes count as "rules"
  const activePage = page === "rules" ? "rules" : page;
  document.querySelectorAll(".nav-link[data-page]").forEach((link) => {
    link.classList.toggle("active", link.dataset.page === activePage);
  });

  // Update subnav active state
  document.querySelectorAll(".subnav-link").forEach((link) => {
    link.classList.toggle("active", link.dataset.doc === subpage);
  });

  if (page === "home") initHome();
  if (page === "rules") {
    const docType = ["cr", "mtr", "ipg"].includes(subpage) ? subpage : "cr";
    initRulesViewer(document.getElementById("rules-container"), docType);
  }
}

async function initHome() {
  const status = document.getElementById("status");
  try {
    const msg = await invoke("greet", { name: "Judge" });
    status.textContent = msg;
  } catch {
    status.textContent = "Backend connected";
  }
}

// Rules toggle button — show/hide subnav, don't navigate
document.getElementById("rules-toggle").addEventListener("click", () => {
  const subnav = document.getElementById("rules-subnav");
  subnav.classList.toggle("hidden");
});

// Subnav links — close subnav on click (navigation happens via href)
document.getElementById("rules-subnav").addEventListener("click", (e) => {
  if (e.target.closest(".subnav-link")) closeSubnav();
});

// Close subnav when clicking outside it and the toggle
document.addEventListener("click", (e) => {
  const subnav = document.getElementById("rules-subnav");
  const toggle = document.getElementById("rules-toggle");
  if (!subnav.contains(e.target) && !toggle.contains(e.target)) {
    closeSubnav();
  }
});

window.addEventListener("hashchange", navigate);
window.addEventListener("DOMContentLoaded", navigate);

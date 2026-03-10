import { invoke } from "@tauri-apps/api/core";

const app = document.getElementById("app");

const pages = {
  home: () => `
    <div class="page home-page">
      <h1>The Judge App</h1>
      <p class="subtitle">MTG Judge Utility Tool</p>
      <div class="home-grid">
        <a href="#/rules" class="home-card">
          <h2>Rules</h2>
          <p>CR, MTR, IPG</p>
        </a>
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
  rules: () => `
    <div class="page rules-page">
      <h1>Rules</h1>
      <div class="doc-tabs">
        <button class="tab active" data-doc="cr">Comprehensive Rules</button>
        <button class="tab" data-doc="mtr">Tournament Rules</button>
        <button class="tab" data-doc="ipg">Infraction Procedure Guide</button>
      </div>
      <div class="search-container">
        <input type="text" id="rules-search" placeholder="Search rules..." />
      </div>
      <div id="rules-content" class="rules-content">
        <p>Rules data not yet loaded. Import will be available soon.</p>
      </div>
    </div>
  `,
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

function navigate() {
  const hash = window.location.hash.slice(2) || "home";
  const page = hash.split("/")[0];
  const render = pages[page] || pages.home;
  app.innerHTML = render();

  document.querySelectorAll(".nav-link").forEach((link) => {
    link.classList.toggle("active", link.dataset.page === page);
  });

  if (page === "home") {
    initHome();
  }
}

async function initHome() {
  const status = document.getElementById("status");
  try {
    const msg = await invoke("greet", { name: "Judge" });
    status.textContent = msg;
  } catch (e) {
    status.textContent = "Backend connected";
  }
}

window.addEventListener("hashchange", navigate);
window.addEventListener("DOMContentLoaded", navigate);

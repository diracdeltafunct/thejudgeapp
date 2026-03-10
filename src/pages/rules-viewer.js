import { invoke } from "@tauri-apps/api/core";

// Navigation history stack: each entry is { type, data }
// type: 'toc' | 'section' | 'search'
const history = [];
let toc = [];

export async function initRulesViewer(container) {
  container.innerHTML = `
    <div class="rules-viewer">
      <div class="rules-toolbar">
        <button id="rv-back" class="back-btn" disabled>&#8592; Back</button>
        <span id="rv-breadcrumb" class="breadcrumb">Comprehensive Rules</span>
      </div>
      <div class="search-container">
        <input type="text" id="rv-search" placeholder="Search rules..." autocomplete="off" spellcheck="false" />
        <div id="rv-search-results" class="search-results hidden"></div>
      </div>
      <div id="rv-content" class="rv-content"></div>
    </div>
  `;

  document.getElementById("rv-back").addEventListener("click", navigateBack);
  document.getElementById("rv-search").addEventListener("input", debounce(handleSearch, 300));
  document.getElementById("rv-content").addEventListener("click", handleContentClick);
  document.getElementById("rv-search-results").addEventListener("click", handleSearchResultClick);
  document.addEventListener("click", handleOutsideClick);

  try {
    toc = await invoke("get_toc");
    renderToc();
  } catch {
    document.getElementById("rv-content").innerHTML =
      `<p class="empty-state">Rules not loaded.<br>Run <code>cargo run --bin update_cr</code> to import the CR.</p>`;
  }
}

// ── Rendering ────────────────────────────────────────────────────────────────

function renderToc() {
  const content = document.getElementById("rv-content");
  // Top-level sections are single-digit numbers
  const sections = toc.filter((e) => /^\d$/.test(e.number));

  content.innerHTML = `
    <div class="toc-list">
      ${sections
        .map((s) => {
          const subsections = toc.filter(
            (e) => e.number.length === 3 && e.number.startsWith(s.number)
          );
          return `
          <div class="toc-section">
            <div class="toc-section-title">${s.number}. ${escHtml(s.title)}</div>
            <div class="toc-subsections">
              ${subsections
                .map(
                  (sub) =>
                    `<button class="toc-entry" data-number="${sub.number}">
                      <span class="entry-number">${sub.number}</span>
                      <span class="entry-title">${escHtml(sub.title)}</span>
                    </button>`
                )
                .join("")}
            </div>
          </div>`;
        })
        .join("")}
    </div>
  `;

  setBreadcrumb("Comprehensive Rules");
  setBackEnabled(history.length > 0);
}

async function renderSection(prefix) {
  const content = document.getElementById("rv-content");
  content.innerHTML = `<p class="loading">Loading...</p>`;

  let rules;
  try {
    rules = await invoke("get_rule_section", { prefix });
  } catch (e) {
    content.innerHTML = `<p class="empty-state">Failed to load section ${prefix}: ${e}</p>`;
    return;
  }

  if (!rules.length) {
    content.innerHTML = `<p class="empty-state">No rules found for section ${prefix}.</p>`;
    return;
  }

  content.innerHTML = rules
    .map((rule) => {
      if (rule.title) {
        // Section or subsection header
        const tag = rule.number.length <= 3 ? "h2" : "h3";
        return `<${tag} class="rule-header" id="R${rule.number}">${rule.number}. ${escHtml(rule.title)}</${tag}>`;
      }
      return `
        <div class="rule-entry" id="R${rule.number}">
          <span class="rule-number">${rule.number}</span>
          <span class="rule-body">${rule.body_html}</span>
        </div>`;
    })
    .join("\n");

  // Find the section header title for the breadcrumb
  const header = rules.find((r) => r.title);
  setBreadcrumb(header ? `${header.number}. ${header.title}` : `Section ${prefix}`);
  setBackEnabled(true);
  content.scrollTop = 0;
}

function renderSearchResults(results) {
  const box = document.getElementById("rv-search-results");
  if (!results.length) {
    box.innerHTML = `<p class="empty-state">No results.</p>`;
    box.classList.remove("hidden");
    return;
  }
  box.innerHTML = results
    .map(
      (r) => `
      <button class="search-result-item" data-number="${r.number}">
        <span class="result-number">${r.number}</span>
        <span class="result-snippet">${r.snippet}</span>
      </button>`
    )
    .join("");
  box.classList.remove("hidden");
}

// ── Navigation ───────────────────────────────────────────────────────────────

function pushHistory(entry) {
  history.push(entry);
}

function navigateBack() {
  history.pop(); // Remove current
  const prev = history[history.length - 1];
  if (!prev || prev.type === "toc") {
    history.length = 0;
    renderToc();
  } else if (prev.type === "section") {
    renderSection(prev.data);
  }
}

async function navigateToRule(ruleNumber) {
  // Extract section prefix (e.g. "704" from "704.5k", "1" from "1")
  const prefix = ruleNumber.includes(".")
    ? ruleNumber.split(".")[0]
    : ruleNumber;

  pushHistory({ type: "section", data: prefix });
  await renderSection(prefix);

  // Scroll to the specific rule anchor after render
  const anchor = document.getElementById(`R${ruleNumber}`);
  if (anchor) {
    anchor.scrollIntoView({ behavior: "smooth", block: "start" });
    anchor.classList.add("highlight");
    setTimeout(() => anchor.classList.remove("highlight"), 2000);
  }
}

// ── Event handlers ───────────────────────────────────────────────────────────

function handleContentClick(e) {
  // Rule cross-reference links (e.g. <a href="#R704.5k">)
  const link = e.target.closest("a.rule-ref");
  if (link) {
    e.preventDefault();
    const ruleNum = link.getAttribute("href").slice(2); // strip "#R"
    pushHistory({ type: "rule", data: ruleNum });
    navigateToRule(ruleNum);
    return;
  }

  // TOC subsection buttons
  const tocEntry = e.target.closest(".toc-entry");
  if (tocEntry) {
    const num = tocEntry.dataset.number;
    pushHistory({ type: "toc" });
    pushHistory({ type: "section", data: num });
    renderSection(num);
    return;
  }
}

function handleSearchResultClick(e) {
  const searchItem = e.target.closest(".search-result-item");
  if (!searchItem) return;
  const num = searchItem.dataset.number;
  closeSearch();
  pushHistory({ type: "section", data: num.split(".")[0] });
  navigateToRule(num);
}

function handleOutsideClick(e) {
  const searchContainer = document.querySelector(".search-container");
  if (searchContainer && !searchContainer.contains(e.target)) {
    closeSearch();
  }
}

function closeSearch() {
  const box = document.getElementById("rv-search-results");
  const input = document.getElementById("rv-search");
  if (box) box.classList.add("hidden");
  if (input) input.value = "";
}

async function handleSearch(e) {
  const query = e.target.value.trim();
  const box = document.getElementById("rv-search-results");

  if (query.length < 2) {
    box.classList.add("hidden");
    return;
  }

  try {
    const results = await invoke("search_rules", {
      query,
      docType: "cr",
    });
    renderSearchResults(results);
  } catch {
    box.classList.add("hidden");
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function setBreadcrumb(text) {
  document.getElementById("rv-breadcrumb").textContent = text;
}

function setBackEnabled(enabled) {
  document.getElementById("rv-back").disabled = !enabled;
}

function escHtml(str) {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function debounce(fn, ms) {
  let timer;
  return (...args) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

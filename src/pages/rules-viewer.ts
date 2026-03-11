import { invoke } from "@tauri-apps/api/core";

interface TocEntry {
  number: string;
  title: string;
  doc_type: string;
}

interface RuleEntry {
  number: string;
  title: string | null;
  body_html: string | null;
  doc_type: string;
}

interface SearchResult {
  number: string;
  snippet: string;
  doc_type: string;
}

type DocType = "cr" | "mtr" | "ipg";

type HistoryEntry =
  | { type: "toc" }
  | { type: "section"; data: string; docType: DocType }
  | { type: "rule"; data: string; docType: DocType };

// Navigation history stack
const history: HistoryEntry[] = [];
let toc: TocEntry[] = [];
let currentDocType: DocType = "cr";

export async function initRulesViewer(container: HTMLElement, initialDocType: DocType = "cr"): Promise<void> {
  currentDocType = initialDocType;
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

  document.getElementById("rv-back")!.addEventListener("click", navigateBack);
  document.getElementById("rv-search")!.addEventListener("input", debounce(handleSearch, 300));
  document.getElementById("rv-content")!.addEventListener("click", handleContentClick);
  document.getElementById("rv-search-results")!.addEventListener("click", handleSearchResultClick);
  document.addEventListener("click", handleOutsideClick);

  try {
    toc = await invoke<TocEntry[]>("get_toc");
    renderToc();
  } catch {
    document.getElementById("rv-content")!.innerHTML =
      `<p class="empty-state">Rules not loaded.<br>Run <code>cargo run --bin update_cr</code> to import the CR.</p>`;
  }
}

// ── Rendering ────────────────────────────────────────────────────────────────

function renderToc(): void {
  const content = document.getElementById("rv-content")!;
  const entries = toc.filter((e) => e.doc_type === currentDocType);

  if (!entries.length) {
    const label = currentDocType === "cr" ? "CR" : currentDocType === "mtr" ? "MTR" : "IPG";
    const bin = currentDocType === "cr" ? "update_cr" : currentDocType === "mtr" ? "update_mtr" : "update_ipg";
    content.innerHTML = `<p class="empty-state">No ${label} data loaded.<br>Run <code>cargo run --bin ${bin}</code> to import.</p>`;
    setBreadcrumb(currentDocType === "cr" ? "Comprehensive Rules" : currentDocType === "mtr" ? "Tournament Rules" : "Infraction Procedure Guide");
    setBackEnabled(false);
    return;
  }

  const isTopLevel =
    currentDocType === "cr"
      ? (e: TocEntry) => /^\d$/.test(e.number)
      : (e: TocEntry) => /^\d+$/.test(e.number);

  const isSubsection =
    currentDocType === "cr"
      ? (e: TocEntry) => /^\d{3}$/.test(e.number)
      : (e: TocEntry) => /^\d+\.\d+$/.test(e.number);

  const sections = entries.filter(isTopLevel);

  content.innerHTML = `
    <div class="toc-list">
      ${sections
        .map((s) => {
          const subsections = entries.filter(
            (e) => isSubsection(e) && e.number.startsWith(s.number + (currentDocType === "cr" ? "" : "."))
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

  setBreadcrumb(currentDocType === "cr" ? "Comprehensive Rules" : currentDocType === "mtr" ? "Tournament Rules" : "Infraction Procedure Guide");
  setBackEnabled(history.length > 0);
}

async function renderSection(prefix: string, docType: DocType = currentDocType): Promise<void> {
  const content = document.getElementById("rv-content")!;
  content.innerHTML = `<p class="loading">Loading...</p>`;

  let rules: RuleEntry[];
  try {
    rules = await invoke<RuleEntry[]>("get_rule_section", { prefix, docType });
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
        const tag = rule.number.length <= 3 ? "h2" : "h3";
        const body = rule.body_html
          ? `<div class="rule-body">${rule.body_html}</div>`
          : "";
        return `<${tag} class="rule-header" id="R${rule.number}">${rule.number}. ${escHtml(rule.title)}</${tag}>${body}`;
      }
      return `
        <div class="rule-entry" id="R${rule.number}">
          <span class="rule-number">${rule.number}</span>
          <span class="rule-body">${rule.body_html}</span>
        </div>`;
    })
    .join("\n");

  const header = rules.find((r) => r.title);
  setBreadcrumb(header ? `${header.number}. ${header.title}` : `Section ${prefix}`);
  setBackEnabled(true);
  content.scrollTop = 0;
}

function renderSearchResults(results: SearchResult[]): void {
  const box = document.getElementById("rv-search-results")!;
  if (!results.length) {
    box.innerHTML = `<p class="empty-state">No results.</p>`;
    box.classList.remove("hidden");
    return;
  }
  box.innerHTML = results
    .map(
      (r) => `
      <button class="search-result-item" data-number="${r.number}" data-doc-type="${r.doc_type}">
        <span class="result-number">${r.doc_type.toUpperCase()} ${r.number}</span>
        <span class="result-snippet">${r.snippet}</span>
      </button>`
    )
    .join("");
  box.classList.remove("hidden");
}

// ── Navigation ───────────────────────────────────────────────────────────────

function pushHistory(entry: HistoryEntry): void {
  history.push(entry);
}

function navigateBack(): void {
  history.pop();
  const prev = history[history.length - 1];
  if (!prev || prev.type === "toc") {
    history.length = 0;
    renderToc();
  } else if (prev.type === "section") {
    currentDocType = prev.docType;
    renderSection(prev.data, prev.docType);
  }
}

async function navigateToRule(ruleNumber: string, docType: DocType = currentDocType): Promise<void> {
  const prefix = ruleNumber.split(".")[0];
  currentDocType = docType;
  pushHistory({ type: "section", data: prefix, docType });
  await renderSection(prefix, docType);

  const anchor = document.getElementById(`R${ruleNumber}`);
  if (anchor) {
    anchor.scrollIntoView({ behavior: "smooth", block: "start" });
    anchor.classList.add("highlight");
    setTimeout(() => anchor.classList.remove("highlight"), 2000);
  }
}

// ── Event handlers ───────────────────────────────────────────────────────────

function handleContentClick(e: MouseEvent): void {
  const link = (e.target as Element).closest("a.rule-ref");
  if (link) {
    e.preventDefault();
    const ruleNum = link.getAttribute("href")!.slice(2);
    pushHistory({ type: "rule", data: ruleNum, docType: currentDocType });
    navigateToRule(ruleNum, currentDocType);
    return;
  }

  const tocEntry = (e.target as Element).closest(".toc-entry") as HTMLElement | null;
  if (tocEntry) {
    const num = tocEntry.dataset.number!;
    pushHistory({ type: "toc" });
    pushHistory({ type: "section", data: num, docType: currentDocType });
    renderSection(num, currentDocType);
    return;
  }
}

function handleSearchResultClick(e: MouseEvent): void {
  const searchItem = (e.target as Element).closest(".search-result-item") as HTMLElement | null;
  if (!searchItem) return;
  const num = searchItem.dataset.number!;
  const docType = searchItem.dataset.docType as DocType;
  closeSearch();
  if (docType !== currentDocType) {
    currentDocType = docType;
  }
  pushHistory({ type: "section", data: num.split(".")[0], docType });
  navigateToRule(num, docType);
}

function handleOutsideClick(e: MouseEvent): void {
  const searchContainer = document.querySelector(".search-container");
  if (searchContainer && !searchContainer.contains(e.target as Node)) {
    closeSearch();
  }
}

function closeSearch(): void {
  const box = document.getElementById("rv-search-results");
  const input = document.getElementById("rv-search") as HTMLInputElement | null;
  if (box) box.classList.add("hidden");
  if (input) input.value = "";
}

async function handleSearch(e: Event): Promise<void> {
  const query = (e.target as HTMLInputElement).value.trim();
  const box = document.getElementById("rv-search-results")!;

  if (query.length < 2) {
    box.classList.add("hidden");
    return;
  }

  try {
    const results = await invoke<SearchResult[]>("search_rules", {
      query,
      docType: currentDocType,
    });
    renderSearchResults(results);
  } catch {
    box.classList.add("hidden");
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function setBreadcrumb(text: string): void {
  document.getElementById("rv-breadcrumb")!.textContent = text;
}

function setBackEnabled(enabled: boolean): void {
  (document.getElementById("rv-back") as HTMLButtonElement).disabled = !enabled;
}

function escHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

function debounce<T extends unknown[]>(fn: (...args: T) => void, ms: number): (...args: T) => void {
  let timer: ReturnType<typeof setTimeout>;
  return (...args: T) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

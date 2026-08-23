import { invoke } from "@tauri-apps/api/core";
import castingRaw from "../data/magic_casting_process.txt?raw";
import copyableRaw from "../data/magic_copyable_characteristics.txt?raw";
import layersRaw from "../data/magic_layers.txt?raw";
import bundledRiftboundRaw from "../data/riftbound_quick_reference.txt?raw";

interface Section {
  title: string;
  crRule: string | null;
  lines: string[];
  links?: { label: string; url: string }[];
}

let riftboundReferencePromise: Promise<string> | null = null;

export function preloadRiftboundQuickReference(): Promise<string> {
  if (!riftboundReferencePromise) {
    riftboundReferencePromise = invoke<string>(
      "sync_riftbound_quick_reference",
    ).catch(() => bundledRiftboundRaw);
  }
  return riftboundReferencePromise;
}

export function parseSection(raw: string): {
  crRule: string | null;
  lines: string[];
} {
  const crMatch = raw.match(/<insert link to CR ([\d.]+) here>/i);
  const crRule = crMatch ? crMatch[1] : null;
  const lines = raw
    .split("\n")
    .map((l) => l.trimEnd())
    .filter((l) => !l.match(/^<.*link to CR/i))
    .join("\n")
    .trim()
    .split("\n");
  return { crRule, lines };
}

export function parseLinkSection(
  raw: string,
): { label: string; url: string }[] {
  return raw
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0)
    .flatMap((l) => {
      const colon = l.indexOf(": ");
      if (colon === -1) return [];
      const url = safeExternalUrl(l.slice(colon + 2).trim());
      if (!url) return [];
      return [
        { label: l.slice(0, colon).trim(), url },
      ];
    });
}

export function parseRiftboundQuickReference(raw: string): Section[] {
  const sections: Section[] = [];
  let title: string | null = null;
  let body: string[] = [];

  const finishSection = () => {
    if (!title) return;
    const crLine = body.find((line) => line.trim().startsWith("@cr:"));
    const candidateCrRule = crLine?.split(":", 2)[1]?.trim() || "";
    const crRule = /^\d+(?:\.\d+)*\.?$/.test(candidateCrRule)
      ? candidateCrRule
      : null;
    const isLinks = body.some((line) => line.trim() === "@links");
    const content = body
      .filter((line) => !/^@(cr:|links$)/i.test(line.trim()))
      .join("\n")
      .trim();
    sections.push({
      title,
      crRule,
      lines: isLinks ? [] : content.split("\n"),
      links: isLinks ? parseLinkSection(content) : undefined,
    });
  };

  for (const line of raw.replace(/\r\n/g, "\n").split("\n")) {
    if (line.startsWith("## ")) {
      finishSection();
      title = line.slice(3).trim();
      body = [];
    } else if (title) {
      body.push(line);
    }
  }
  finishSection();
  return sections;
}

export function renderLines(lines: string[]): string {
  return lines
    .map((line) => {
      if (!line.trim()) return "";
      const indent = line.match(/^(\s+)/)?.[1].length ?? 0;
      const indentClass = indent >= 4 ? " qr-sub" : "";
      return `<div class="qr-line${indentClass}">${renderInlineFormatting(line.trim())}</div>`;
    })
    .join("");
}

function renderLinks(links: { label: string; url: string }[]): string {
  return links
    .map(
      (link) =>
        `<button class="qr-link-btn" data-url="${escapeHtml(link.url)}">${escapeHtml(link.label)}</button>`,
    )
    .join("");
}

function renderSection(section: Section, index: number): string {
  const crLink = section.crRule
    ? `<a class="qr-cr-link" href="#/rules/cr/${section.crRule.replace(/\.$/, "")}" onclick="event.stopPropagation()">CR ${section.crRule}</a>`
    : "";
  const content = section.links
    ? renderLinks(section.links)
    : renderLines(section.lines);
  return `
    <div class="qr-section" id="qr-section-${index}">
      <button class="qr-section-header" data-index="${index}">
        <span class="qr-title">${escapeHtml(section.title)}</span>
        <span class="qr-header-right">
          ${crLink}
          <span class="qr-chevron">&#9660;</span>
        </span>
      </button>
      <div class="qr-content hidden">${content}</div>
    </div>
  `;
}

function safeExternalUrl(raw: string): string | null {
  try {
    const url = new URL(raw);
    return url.protocol === "https:" || url.protocol === "http:" ? raw : null;
  } catch {
    return null;
  }
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

function renderInlineFormatting(value: string): string {
  return escapeHtml(value)
    .replace(/&lt;(\/?)strong&gt;/gi, "<$1strong>")
    .replace(/&lt;(\/?)i&gt;/gi, "<$1i>");
}

const magicSections: Section[] = [
  { title: "Casting Process", ...parseSection(castingRaw) },
  { title: "Copyable Characteristics", ...parseSection(copyableRaw) },
  { title: "Layers", ...parseSection(layersRaw) },
];

export function initQuickReference(container: HTMLElement, game: string): void {
  if (game === "riftbound") {
    container.innerHTML = `<div class="page quick-reference-page"><h1>Quick Reference</h1><p class="loading">Loading...</p></div>`;
    void preloadRiftboundQuickReference().then((raw) => {
      if (container.isConnected) {
        renderQuickReference(container, parseRiftboundQuickReference(raw));
      }
    });
    return;
  }

  renderQuickReference(container, magicSections);
}

function renderQuickReference(container: HTMLElement, sections: Section[]): void {
  container.innerHTML = `
    <div class="page quick-reference-page">
      <h1>Quick Reference</h1>
      ${
        sections.length === 0
          ? `<p class="empty-state">No quick reference available.</p>`
          : sections.map((s, i) => renderSection(s, i)).join("")
      }
    </div>
  `;

  container
    .querySelectorAll<HTMLButtonElement>(".qr-section-header")
    .forEach((btn) => {
      btn.addEventListener("click", () => {
        const section = btn.closest(".qr-section")!;
        const content = section.querySelector(".qr-content")!;
        const chevron = btn.querySelector(".qr-chevron")!;
        const open = !content.classList.contains("hidden");
        content.classList.toggle("hidden", open);
        chevron.classList.toggle("qr-chevron-open", !open);
        section.classList.toggle("open", !open);
      });
    });

  container
    .querySelectorAll<HTMLButtonElement>(".qr-link-btn")
    .forEach((btn) => {
      btn.addEventListener("click", () => {
        const url = safeExternalUrl(btn.dataset.url ?? "");
        if (url) invoke("open_custom_tab", { url });
      });
    });
}

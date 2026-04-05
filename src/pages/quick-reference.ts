import { invoke } from "@tauri-apps/api/core";
import castingRaw from "../data/magic_casting_process.txt?raw";
import copyableRaw from "../data/magic_copyable_characteristics.txt?raw";
import layersRaw from "../data/magic_layers.txt?raw";
import riftboundBannedListRaw from "../data/riftbound_banned_list.txt?raw";
import riftboundHotFeprRaw from "../data/riftbound_HOT_FEPR.txt?raw";
import riftboundLinksRaw from "../data/riftbound_relevant_links.txt?raw";
import riftboundStartOfGameRaw from "../data/riftbound_start_of_game_procedure.txt?raw";
import riftboundStartOfTurnRaw from "../data/riftbound_start_of_turn.txt?raw";

interface Section {
  title: string;
  crRule: string | null;
  lines: string[];
  links?: { label: string; url: string }[];
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
      return [
        { label: l.slice(0, colon).trim(), url: l.slice(colon + 2).trim() },
      ];
    });
}

export function renderLines(lines: string[]): string {
  return lines
    .map((line) => {
      if (!line.trim()) return "";
      const indent = line.match(/^(\s+)/)?.[1].length ?? 0;
      const indentClass = indent >= 4 ? " qr-sub" : "";
      return `<div class="qr-line${indentClass}">${line.trim()}</div>`;
    })
    .join("");
}

function renderLinks(links: { label: string; url: string }[]): string {
  return links
    .map(
      (link) =>
        `<button class="qr-link-btn" data-url="${link.url}">${link.label}</button>`,
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
        <span class="qr-title">${section.title}</span>
        <span class="qr-header-right">
          ${crLink}
          <span class="qr-chevron">&#9660;</span>
        </span>
      </button>
      <div class="qr-content hidden">${content}</div>
    </div>
  `;
}

const magicSections: Section[] = [
  { title: "Casting Process", ...parseSection(castingRaw) },
  { title: "Copyable Characteristics", ...parseSection(copyableRaw) },
  { title: "Layers", ...parseSection(layersRaw) },
];

// Add riftbound_*.txt imports above and new entries here as more are created.
const riftboundSections: Section[] = [
  {
    title: "Start of Game Procedure",
    ...parseSection(riftboundStartOfGameRaw),
  },
  { title: "Start of Turn", ...parseSection(riftboundStartOfTurnRaw) },
  { title: "Banned List", ...parseSection(riftboundBannedListRaw) },
  { title: "HOT FEPR", ...parseSection(riftboundHotFeprRaw) },
  {
    title: "Relevant Links",
    crRule: null,
    lines: [],
    links: parseLinkSection(riftboundLinksRaw),
  },
];

export function initQuickReference(container: HTMLElement, game: string): void {
  const sections = game === "riftbound" ? riftboundSections : magicSections;

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
        invoke("open_custom_tab", { url: btn.dataset.url });
      });
    });
}

import castingRaw from "../../resources/casting_process?raw";
import copyableRaw from "../../resources/copyable_characteristics.txt?raw";
import layersRaw from "../../resources/layers.txt?raw";

interface Section {
  title: string;
  crRule: string | null;
  lines: string[];
}

function parseSection(raw: string): { crRule: string | null; lines: string[] } {
  const crMatch = raw.match(/<insert link to CR ([\d.]+) here>/);
  const crRule = crMatch ? crMatch[1] : null;
  const lines = raw
    .split("\n")
    .map((l) => l.trimEnd())
    .filter((l) => !l.match(/^<insert link/))
    .join("\n")
    .trim()
    .split("\n");
  return { crRule, lines };
}

function renderLines(lines: string[]): string {
  return lines
    .map((line) => {
      if (!line.trim()) return "";
      const indent = line.match(/^(\s+)/)?.[1].length ?? 0;
      const indentClass = indent >= 4 ? " qr-sub" : "";
      return `<div class="qr-line${indentClass}">${line.trim()}</div>`;
    })
    .join("");
}

function renderSection(section: Section, index: number): string {
  const crLink = section.crRule
    ? `<a class="qr-cr-link" href="#/rules/cr/${section.crRule.replace(/\.$/, "")}" onclick="event.stopPropagation()">CR ${section.crRule}</a>`
    : "";
  return `
    <div class="qr-section" id="qr-section-${index}">
      <button class="qr-section-header" data-index="${index}">
        <span class="qr-title">${section.title}</span>
        <span class="qr-header-right">
          ${crLink}
          <span class="qr-chevron">&#9660;</span>
        </span>
      </button>
      <div class="qr-content hidden">${renderLines(section.lines)}</div>
    </div>
  `;
}

export function initQuickReference(container: HTMLElement): void {
  const casting = parseSection(castingRaw);
  const copyable = parseSection(copyableRaw);
  const layers = parseSection(layersRaw);

  const sections: Section[] = [
    { title: "Casting Process", crRule: casting.crRule, lines: casting.lines },
    { title: "Copyable Characteristics", crRule: copyable.crRule, lines: copyable.lines },
    { title: "Layers", crRule: layers.crRule, lines: layers.lines },
  ];

  container.innerHTML = `
    <div class="page quick-reference-page">
      <h1>Quick Reference</h1>
      ${sections.map((s, i) => renderSection(s, i)).join("")}
    </div>
  `;

  container.querySelectorAll<HTMLButtonElement>(".qr-section-header").forEach((btn) => {
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
}

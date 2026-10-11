import fs from "node:fs";
import path from "node:path";

// Emit an apply_patch patch so source updates can be reviewed before applying.
// Usage: node scripts/import-riftbound-data.mjs /path/to/orulings
const source = process.argv[2];
if (!source) throw new Error("Supply the orulings directory");
const edits = [];
function update(file, content) {
  const existing = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : null;
  if (existing === content) return;
  if (content.length + (existing?.length ?? 0) > 700000) {
    if (existing !== null) edits.push(`*** Delete File: ${file}`);
    const lines = content.trimEnd().split("\n");
    let chunk = [];
    let size = 0;
    let previous = null;
    const flush = () => {
      if (!chunk.length) return;
      edits.push(previous === null
        ? `*** Add File: ${file}\n${chunk.map(line => `+${line}`).join("\n")}`
        : `*** Update File: ${file}\n@@\n ${previous}\n${chunk.map(line => `+${line}`).join("\n")}\n*** End of File`);
      previous = chunk.at(-1);
      chunk = [];
      size = 0;
    };
    for (const line of lines) {
      if (size + line.length > 300000) flush();
      chunk.push(line);
      size += line.length + 1;
    }
    flush();
    return;
  }
  edits.push(existing === null
    ? `*** Add File: ${file}\n${content.trimEnd().split("\n").map(line => `+${line}`).join("\n")}`
    : `*** Update File: ${file}\n@@\n${existing.trimEnd().split("\n").map(line => `-${line}`).join("\n")}\n${content.trimEnd().split("\n").map(line => `+${line}`).join("\n")}`);
}
const manifest = JSON.parse(fs.readFileSync("data-manifest.json", "utf8"));
for (const kind of ["cr", "tr"]) {
  const dir = path.join(source, "rules_source", `${kind}sections_october_2026`);
  const metadata = JSON.parse(fs.readFileSync(path.join(dir, "metadata.json"), "utf8"));
  const sections = fs.readdirSync(dir).filter(name => /^\d+\.json$/.test(name)).sort()
    .map(name => {
      const section = JSON.parse(fs.readFileSync(path.join(dir, name), "utf8"));
      update(`src-tauri/src/riftbound_data/${kind}/${name}`, JSON.stringify(section, null, 2) + "\n");
      return section;
    });
  const version = metadata.last_updated.replaceAll("-", "");
  update(`data/riftbound_${kind}.json`, JSON.stringify(sections.filter(section => kind !== "tr" || section.section !== "700")) + "\n");
  manifest[`riftbound_${kind}`].version = version;
  if (kind === "tr") {
    update("data/riftbound_ep.json", JSON.stringify(sections.filter(section => section.section === "700")) + "\n");
    manifest.riftbound_ep.version = version;
  }
}
const cards = JSON.parse(fs.readFileSync(path.join(source, "scripts", "riftbound_cards_with_errata.json"), "utf8"));
const ids = new Map();
for (const card of cards) {
  if (!card.id || !card.name) throw new Error(`Invalid card: ${card.id}`);
  for (const field of ["ability", "errata_text", "errata_old_text"]) {
    if (typeof card[field] === "string") card[field] = card[field].split("Related Articles")[0].trim();
  }
  if (ids.has(card.id) && JSON.stringify(ids.get(card.id)) !== JSON.stringify(card)) {
    throw new Error(`Conflicting duplicate card: ${card.id}`);
  }
  ids.set(card.id, card);
}
update("data/riftbound_cards.json", JSON.stringify([...ids.values()], null, 2) + "\n");
update("data-manifest.json", JSON.stringify(manifest, null, 2) + "\n");
const selected = process.argv[3];
if (selected === "list") {
  process.stdout.write(JSON.stringify(edits.map((edit, index) => ({ index, file: edit.split("\n")[0], size: edit.length }))));
} else {
  const chosen = selected === undefined ? edits : [edits[Number(selected)]];
  process.stdout.write(`*** Begin Patch\n${chosen.join("\n")}\n*** End Patch\n`);
}

import type { Game } from "./theme.js";

export type DocType =
  | "cr"
  | "mtr"
  | "ipg"
  | "jar"
  | "riftbound_cr"
  | "riftbound_tr"
  | "riftbound_ep";

export const ALL_DOC_TYPES: DocType[] = [
  "cr",
  "mtr",
  "ipg",
  "jar",
  "riftbound_cr",
  "riftbound_tr",
  "riftbound_ep",
];

export function isRulesHash(hash: string): boolean {
  const parts = hash.replace(/^#\//, "").split("/");
  return parts[0] === "rules" && ALL_DOC_TYPES.includes(parts[1] as DocType);
}

export function normalizeRulesDocForGame(docType: DocType, game: Game): DocType {
  if (game === "riftbound") {
    switch (docType) {
      case "cr":
        return "riftbound_cr";
      case "mtr":
        return "riftbound_tr";
      case "ipg":
      case "jar":
        return "riftbound_ep";
      default:
        return docType;
    }
  }

  switch (docType) {
    case "riftbound_cr":
      return "cr";
    case "riftbound_tr":
      return "mtr";
    case "riftbound_ep":
      return "ipg";
    default:
      return docType;
  }
}

export function normalizeRulesHashForGame(hash: string, game: Game): string {
  if (!isRulesHash(hash)) return hash;
  const parts = hash.replace(/^#\//, "").split("/");
  const originalDoc = parts[1] as DocType;
  const normalizedDoc = normalizeRulesDocForGame(originalDoc, game);
  const suffix = normalizedDoc === originalDoc ? parts.slice(2).join("/") : "";
  return `#/rules/${normalizedDoc}${suffix ? `/${suffix}` : ""}`;
}

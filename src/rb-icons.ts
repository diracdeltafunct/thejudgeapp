// Maps :rb_token: patterns to icon filenames in /rb_icons/
const RB_TOKEN_MAP: Record<string, string> = {
  rb_rune_rainbow: "rune.webp",
  rb_rune_calm:    "calm.webp",
  rb_rune_fury:    "fury.webp",
  rb_rune_mind:    "mind.webp",
  rb_rune_body:    "body.webp",
  rb_rune_chaos:   "chaos.webp",
  rb_rune_order:   "order.webp",
  rb_might:        "might.webp",
  rb_exhaust:      "exhaust.webp",
};
for (let i = 0; i <= 12; i++) {
  RB_TOKEN_MAP[`rb_energy_${i}`] = `${i}.svg`;
}

// Maps lowercase [Keyword] names to icon filenames in /rb_icons/
const KEYWORD_MAP: Record<string, string> = {
  "accelerate":   "ACCELERATE.webp",
  "action":       "ACTION.webp",
  "add":          "ADD.webp",
  "assault 2":    "ASSAULT 2.webp",
  "assault 3":    "ASSAULT 3.webp",
  "assault":      "ASSAULT.webp",
  "deathknell":   "DEATHKNELL.webp",
  "deflect":      "DEFLECT.webp",
  "equip":        "EQUIP.webp",
  "ganking":      "GANKING.webp",
  "hidden":       "HIDDEN.webp",
  "legion":       "LEGION.webp",
  "mighty":       "MIGHTY.webp",
  "quick-draw":   "QUICK-DRAW.webp",
  "repeat":       "REPEAT.webp",
  "reaction":     "Reaction.webp",
  "shield 2":     "SHIELD 2.webp",
  "shield 3":     "SHIELD 3.webp",
  "shield 5":     "SHIELD 5.webp",
  "shield":       "SHIELD.webp",
  "tank":         "TANK.webp",
  "temporary":    "TEMPORARY.webp",
  "vision":       "VISION.webp",
  "weaponmaster": "WEAPONMASTER.webp",
  "unique":       "unique.webp",
};

function iconImg(file: string, alt: string): string {
  const src = `/rb_icons/${encodeURIComponent(file)}`;
  return `<img src="${src}" alt="${alt}" title="${alt}" class="rb-keyword-icon" loading="lazy">`;
}

/**
 * Replace [Keyword] and :rb_token: patterns in card ability text with inline icons.
 * The input must already be HTML-escaped; this returns HTML with <img> tags injected.
 */
export function replaceRbIcons(text: string): string {
  // Replace :rb_token: patterns
  text = text.replace(/:(\w+):/g, (_match, token: string) => {
    const file = RB_TOKEN_MAP[token.toLowerCase()];
    return file ? iconImg(file, token) : _match;
  });

  // Replace [Keyword] patterns (keyword may include spaces and numbers)
  text = text.replace(/\[([^\]]+)\]/g, (_match, keyword: string) => {
    const lower = keyword.toLowerCase();
    // Try exact match first, then strip trailing number (e.g. "Shield 3" -> "Shield")
    const file = KEYWORD_MAP[lower]
      ?? KEYWORD_MAP[lower.replace(/\s*\d+$/, "")];
    return file ? iconImg(file, keyword) : _match;
  });

  return text;
}

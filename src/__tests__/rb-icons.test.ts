import { describe, it, expect } from "vitest";
import { replaceRbIcons } from "../rb-icons.js";

describe("replaceRbIcons", () => {
  it("replaces known :rb_token: patterns with img tags", () => {
    const result = replaceRbIcons(":rb_exhaust:");
    expect(result).toContain("<img");
    expect(result).toContain("rb_icons/exhaust.webp");
  });

  it("replaces energy token patterns", () => {
    const result = replaceRbIcons(":rb_energy_3:");
    expect(result).toContain("rb_icons/3.svg");
  });

  it("leaves unknown :tokens: unchanged", () => {
    const result = replaceRbIcons(":unknown_token:");
    expect(result).toBe(":unknown_token:");
  });

  it("replaces known [Keyword] patterns with img tags", () => {
    const result = replaceRbIcons("[Assault]");
    expect(result).toContain("<img");
    expect(result).toContain("ASSAULT.webp");
  });

  it("replaces [Keyword] case-insensitively", () => {
    const result = replaceRbIcons("[assault]");
    expect(result).toContain("ASSAULT.webp");
    const result2 = replaceRbIcons("[ASSAULT]");
    expect(result2).toContain("ASSAULT.webp");
  });

  it("replaces numbered keywords like [Shield 3]", () => {
    const result = replaceRbIcons("[Shield 3]");
    // Space is URL-encoded in the src attribute
    expect(result).toContain("SHIELD%203.webp");
  });

  it("falls back to base keyword if numbered variant missing", () => {
    // Shield 9 doesn't exist but Shield does
    const result = replaceRbIcons("[Shield 9]");
    expect(result).toContain("SHIELD.webp");
  });

  it("leaves unknown [Keywords] unchanged", () => {
    const result = replaceRbIcons("[UnknownKeyword]");
    expect(result).toBe("[UnknownKeyword]");
  });

  it("replaces multiple tokens in one string", () => {
    const result = replaceRbIcons(":rb_exhaust: and [Assault]");
    expect(result).toContain("exhaust.webp");
    expect(result).toContain("ASSAULT.webp");
  });

  it("img tags include alt attribute", () => {
    const result = replaceRbIcons(":rb_might:");
    expect(result).toContain('alt="rb_might"');
  });

  it("passes through plain text unchanged", () => {
    expect(replaceRbIcons("no tokens here")).toBe("no tokens here");
  });
});

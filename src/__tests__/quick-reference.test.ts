import { describe, it, expect } from "vitest";
import { parseSection, parseLinkSection, renderLines } from "../pages/quick-reference.js";

describe("parseSection", () => {
  it("extracts no CR rule from plain text", () => {
    const { crRule, lines } = parseSection("Step one\nStep two");
    expect(crRule).toBeNull();
    expect(lines).toContain("Step one");
    expect(lines).toContain("Step two");
  });

  it("extracts CR rule number", () => {
    const { crRule } = parseSection("Do a thing\n<insert link to CR 601.2 here>\nMore text");
    expect(crRule).toBe("601.2");
  });

  it("removes the CR link placeholder from lines", () => {
    const { lines } = parseSection("Step one\n<insert link to CR 601.2 here>\nStep two");
    expect(lines.some((l) => l.includes("<insert"))).toBe(false);
    expect(lines).toContain("Step one");
    expect(lines).toContain("Step two");
  });

  it("handles empty input", () => {
    const { crRule, lines } = parseSection("");
    expect(crRule).toBeNull();
    expect(lines.every((l) => l === "")).toBe(true);
  });

  it("is case-insensitive for CR placeholder", () => {
    const { crRule } = parseSection("<INSERT LINK TO CR 116.1 HERE>");
    expect(crRule).toBe("116.1");
  });
});

describe("parseLinkSection", () => {
  it("parses label: url pairs", () => {
    const links = parseLinkSection("Scryfall: https://scryfall.com\nWotC: https://magic.wizards.com");
    expect(links).toHaveLength(2);
    expect(links[0]).toEqual({ label: "Scryfall", url: "https://scryfall.com" });
    expect(links[1]).toEqual({ label: "WotC", url: "https://magic.wizards.com" });
  });

  it("ignores lines without ': '", () => {
    const links = parseLinkSection("no colon here\nFoo: https://foo.com");
    expect(links).toHaveLength(1);
    expect(links[0].label).toBe("Foo");
  });

  it("skips blank lines", () => {
    const links = parseLinkSection("\n\nFoo: https://foo.com\n\n");
    expect(links).toHaveLength(1);
  });

  it("returns empty array for empty input", () => {
    expect(parseLinkSection("")).toHaveLength(0);
  });

  it("handles url with colons in it", () => {
    const links = parseLinkSection("Site: https://example.com/path?a=1");
    expect(links[0].url).toBe("https://example.com/path?a=1");
  });
});

describe("renderLines", () => {
  it("wraps each non-empty line in a qr-line div", () => {
    const result = renderLines(["Step one", "Step two"]);
    expect(result).toContain('<div class="qr-line">Step one</div>');
    expect(result).toContain('<div class="qr-line">Step two</div>');
  });

  it("returns empty string for blank lines", () => {
    const result = renderLines(["", "  "]);
    expect(result).toBe("");
  });

  it("adds qr-sub class for lines with 4+ leading spaces", () => {
    const result = renderLines(["    indented line"]);
    expect(result).toContain("qr-sub");
  });

  it("does not add qr-sub for lines with fewer than 4 spaces", () => {
    const result = renderLines(["  two spaces"]);
    expect(result).not.toContain("qr-sub");
  });

  it("trims line content in output", () => {
    const result = renderLines(["    padded content"]);
    expect(result).toContain(">padded content<");
  });
});

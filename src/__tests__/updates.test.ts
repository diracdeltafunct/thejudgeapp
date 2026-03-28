import { describe, it, expect } from "vitest";
import { formatSize, escHtml } from "../pages/updates.js";

describe("formatSize", () => {
  it("returns empty string for null", () => {
    expect(formatSize(null)).toBe("");
  });

  it("formats bytes under 1000", () => {
    expect(formatSize(500)).toBe("500 B");
    expect(formatSize(0)).toBe("0 B");
    expect(formatSize(999)).toBe("999 B");
  });

  it("formats kilobytes", () => {
    expect(formatSize(1000)).toBe("~1 KB");
    expect(formatSize(5500)).toBe("~6 KB");
    expect(formatSize(999_999)).toBe("~1000 KB");
  });

  it("formats megabytes", () => {
    expect(formatSize(1_000_000)).toBe("~1 MB");
    expect(formatSize(4_200_000)).toBe("~4 MB");
    expect(formatSize(50_000_000)).toBe("~50 MB");
  });
});

describe("escHtml (updates)", () => {
  it("escapes ampersand", () => {
    expect(escHtml("a & b")).toBe("a &amp; b");
  });

  it("escapes less-than and greater-than", () => {
    expect(escHtml("<b>bold</b>")).toBe("&lt;b&gt;bold&lt;/b&gt;");
  });

  it("escapes double quotes", () => {
    expect(escHtml('"hello"')).toBe("&quot;hello&quot;");
  });

  it("leaves plain text unchanged", () => {
    expect(escHtml("just text")).toBe("just text");
  });

  it("escapes multiple specials in one string", () => {
    expect(escHtml('<a href="x">link & text</a>')).toBe(
      '&lt;a href=&quot;x&quot;&gt;link &amp; text&lt;/a&gt;',
    );
  });
});

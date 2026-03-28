import { describe, it, expect, vi } from "vitest";
import { formatLegalities, formatColors, debounce } from "../pages/cards.js";

describe("formatLegalities", () => {
  it("returns empty for null", () => {
    expect(formatLegalities(null)).toBe("");
  });

  it("returns empty for empty string", () => {
    expect(formatLegalities("")).toBe("");
  });

  it("returns empty for invalid JSON", () => {
    expect(formatLegalities("not-json")).toBe("");
  });

  it("returns empty for empty object", () => {
    expect(formatLegalities("{}")).toBe("");
  });

  it("parses object-form legalities", () => {
    const input = JSON.stringify({ standard: "legal", modern: "not_legal" });
    const result = formatLegalities(input);
    expect(result).toContain("standard");
    expect(result).toContain("modern");
    expect(result).toContain("legality-legal");
    expect(result).toContain("legality-not-legal"); // underscores → hyphens
  });

  it("parses array-form legalities", () => {
    const input = JSON.stringify([["pioneer", "legal"], ["legacy", "banned"]]);
    const result = formatLegalities(input);
    expect(result).toContain("pioneer");
    expect(result).toContain("legacy");
    expect(result).toContain("legality-banned");
  });

  it("wraps output in legalities-grid div", () => {
    const input = JSON.stringify({ standard: "legal" });
    expect(formatLegalities(input)).toContain('class="legalities-grid"');
  });
});

describe("formatColors", () => {
  it("returns empty for null", () => {
    expect(formatColors(null)).toBe("");
  });

  it("returns empty for empty array", () => {
    expect(formatColors("[]")).toBe("");
  });

  it("returns empty for invalid JSON", () => {
    expect(formatColors("not-json")).toBe("");
  });

  it("uppercases color codes", () => {
    expect(formatColors(JSON.stringify(["w", "u"]))).toBe("W U");
  });

  it("handles already-uppercase codes", () => {
    expect(formatColors(JSON.stringify(["R", "G", "B"]))).toBe("R G B");
  });

  it("returns empty for non-array JSON", () => {
    expect(formatColors('"just a string"')).toBe("");
  });
});

describe("debounce", () => {
  it("delays function invocation", async () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 200);

    debounced("a");
    expect(fn).not.toHaveBeenCalled();

    vi.advanceTimersByTime(200);
    expect(fn).toHaveBeenCalledWith("a");
    vi.useRealTimers();
  });

  it("only fires once for rapid calls", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 100);

    debounced(1);
    debounced(2);
    debounced(3);

    vi.advanceTimersByTime(100);
    expect(fn).toHaveBeenCalledTimes(1);
    expect(fn).toHaveBeenCalledWith(3);
    vi.useRealTimers();
  });

  it("fires again after delay if called a second time", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    const debounced = debounce(fn, 50);

    debounced("first");
    vi.advanceTimersByTime(50);
    debounced("second");
    vi.advanceTimersByTime(50);

    expect(fn).toHaveBeenCalledTimes(2);
    vi.useRealTimers();
  });
});

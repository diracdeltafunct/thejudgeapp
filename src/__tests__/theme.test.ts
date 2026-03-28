import { describe, it, expect, beforeEach } from "vitest";
import {
  getTheme, setTheme,
  getFontSize, setFontSize,
  getPackSize, setPackSize,
  getAccent, setAccent,
  getGame, setGame,
  ACCENT_COLORS,
} from "../theme.js";

beforeEach(() => {
  localStorage.clear();
});

describe("theme", () => {
  it("defaults to dark", () => {
    expect(getTheme()).toBe("dark");
  });

  it("persists theme value", () => {
    setTheme("light");
    expect(getTheme()).toBe("light");
  });

  it("returns stored theme", () => {
    localStorage.setItem("theme", "system");
    expect(getTheme()).toBe("system");
  });
});

describe("fontSize", () => {
  it("defaults to medium", () => {
    expect(getFontSize()).toBe("medium");
  });

  it("persists font size", () => {
    setFontSize("large");
    expect(getFontSize()).toBe("large");
  });
});

describe("packSize", () => {
  it("defaults to 14", () => {
    expect(getPackSize()).toBe(14);
  });

  it("returns 15 when stored as '15'", () => {
    setPackSize(15);
    expect(getPackSize()).toBe(15);
  });

  it("returns 14 for any non-15 value", () => {
    localStorage.setItem("packSize", "99");
    expect(getPackSize()).toBe(14);
  });
});

describe("accent", () => {
  it("defaults to first accent color", () => {
    expect(getAccent()).toEqual(ACCENT_COLORS[0]);
  });

  it("persists accent by value", () => {
    const color = ACCENT_COLORS[3];
    setAccent(color);
    expect(getAccent()).toEqual(color);
  });

  it("falls back to first if stored value is unknown", () => {
    localStorage.setItem("accent", "#notacolor");
    expect(getAccent()).toEqual(ACCENT_COLORS[0]);
  });

  it("ACCENT_COLORS has 8 entries", () => {
    expect(ACCENT_COLORS).toHaveLength(8);
  });

  it("each accent has value, hover, and label", () => {
    for (const c of ACCENT_COLORS) {
      expect(c.value).toMatch(/^#/);
      expect(c.hover).toMatch(/^#/);
      expect(typeof c.label).toBe("string");
    }
  });
});

describe("game", () => {
  it("defaults to mtg", () => {
    expect(getGame()).toBe("mtg");
  });

  it("persists game selection", () => {
    setGame("riftbound");
    expect(getGame()).toBe("riftbound");
  });
});

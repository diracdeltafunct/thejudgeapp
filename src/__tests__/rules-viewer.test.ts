import { describe, it, expect } from "vitest";
import { isCrLike, docLabel } from "../pages/rules-viewer.js";

type DocType = "cr" | "mtr" | "ipg" | "jar" | "riftbound_cr" | "riftbound_tr" | "riftbound_ep";

describe("isCrLike", () => {
  it("returns true for cr", () => {
    expect(isCrLike("cr")).toBe(true);
  });

  it("returns true for riftbound_cr", () => {
    expect(isCrLike("riftbound_cr")).toBe(true);
  });

  it("returns false for mtr", () => {
    expect(isCrLike("mtr")).toBe(false);
  });

  it("returns false for ipg", () => {
    expect(isCrLike("ipg")).toBe(false);
  });

  it("returns false for jar", () => {
    expect(isCrLike("jar")).toBe(false);
  });

  it("returns false for riftbound_tr", () => {
    expect(isCrLike("riftbound_tr")).toBe(false);
  });

  it("returns false for riftbound_ep", () => {
    expect(isCrLike("riftbound_ep")).toBe(false);
  });
});

describe("docLabel", () => {
  it("labels cr correctly", () => {
    expect(docLabel("cr")).toBe("Comprehensive Rules");
  });

  it("labels mtr correctly", () => {
    expect(docLabel("mtr")).toBe("Tournament Rules");
  });

  it("labels ipg correctly", () => {
    expect(docLabel("ipg" as DocType)).toContain("Infraction");
  });

  it("labels jar correctly", () => {
    expect(docLabel("jar" as DocType)).toContain("Regular");
  });

  it("returns a non-empty string for all doc types", () => {
    const types: DocType[] = ["cr", "mtr", "ipg", "jar", "riftbound_cr", "riftbound_tr", "riftbound_ep"];
    for (const t of types) {
      expect(docLabel(t).length).toBeGreaterThan(0);
    }
  });
});

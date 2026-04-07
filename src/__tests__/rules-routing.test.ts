import { describe, expect, it } from "vitest";
import {
  isRulesHash,
  normalizeRulesDocForGame,
  normalizeRulesHashForGame,
} from "../rules-routing.js";

describe("normalizeRulesDocForGame", () => {
  it("maps magic CR to riftbound CR in riftbound mode", () => {
    expect(normalizeRulesDocForGame("cr", "riftbound")).toBe("riftbound_cr");
  });

  it("maps riftbound CR to magic CR in mtg mode", () => {
    expect(normalizeRulesDocForGame("riftbound_cr", "mtg")).toBe("cr");
  });

  it("maps penalty docs to riftbound E&P in riftbound mode", () => {
    expect(normalizeRulesDocForGame("ipg", "riftbound")).toBe("riftbound_ep");
    expect(normalizeRulesDocForGame("jar", "riftbound")).toBe("riftbound_ep");
  });
});

describe("normalizeRulesHashForGame", () => {
  it("normalizes a saved rules hash for riftbound mode", () => {
    expect(normalizeRulesHashForGame("#/rules/cr", "riftbound")).toBe(
      "#/rules/riftbound_cr",
    );
  });

  it("preserves deep links while normalizing", () => {
    expect(normalizeRulesHashForGame("#/rules/riftbound_cr/315.2", "riftbound")).toBe(
      "#/rules/riftbound_cr/315.2",
    );
  });

  it("drops deep links when changing doc families across games", () => {
    expect(normalizeRulesHashForGame("#/rules/cr/601.2", "riftbound")).toBe(
      "#/rules/riftbound_cr",
    );
  });

  it("leaves non-rules hashes alone", () => {
    expect(normalizeRulesHashForGame("#/tournament/active", "riftbound")).toBe(
      "#/tournament/active",
    );
  });

  it("recognizes riftbound rules routes", () => {
    expect(isRulesHash("#/rules/riftbound_cr")).toBe(true);
  });
});

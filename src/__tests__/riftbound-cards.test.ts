import { describe, expect, it } from "vitest";
import { riftboundCardNameSuggestions } from "../pages/riftbound-cards.js";

describe("riftboundCardNameSuggestions", () => {
  const cards = [
    { name: "Ahri, Nine-Tailed Fox" },
    { name: "Ahri's Charm" },
    { name: "Jinx, Loose Cannon" },
  ];

  it("returns matching card names in result order", () => {
    expect(riftboundCardNameSuggestions(cards, "ahri")).toEqual([
      "Ahri, Nine-Tailed Fox",
      "Ahri's Charm",
    ]);
  });

  it("is case-insensitive and ignores one-character queries", () => {
    expect(riftboundCardNameSuggestions(cards, "JINX")).toEqual([
      "Jinx, Loose Cannon",
    ]);
    expect(riftboundCardNameSuggestions(cards, "a")).toEqual([]);
  });
});

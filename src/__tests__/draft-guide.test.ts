import { describe, it, expect } from "vitest";
import { parseScript } from "../pages/draft-guide.js";

const SAMPLE_SCRIPT = `
<START FIRST PACK>
Open your first pack.
<Time 25>
Pick a card.
<Time 25>
Pass left.
<END FIRST PACK>

<START SECOND PACK>
Open your second pack.
<Time 25>
Pick a card.
<END SECOND PACK>

<START THIRD PACK>
Open your third pack.
<END THIRD PACK>
`;

describe("parseScript", () => {
  it("parses all three packs", () => {
    const packs = parseScript(SAMPLE_SCRIPT);
    expect(packs).toHaveLength(3);
  });

  it("assigns correct pack names", () => {
    const packs = parseScript(SAMPLE_SCRIPT);
    expect(packs[0].name).toBe("Pack 1");
    expect(packs[1].name).toBe("Pack 2");
    expect(packs[2].name).toBe("Pack 3");
  });

  it("extracts timer values", () => {
    const packs = parseScript(SAMPLE_SCRIPT);
    const timers = packs[0].steps.filter((s) => s.timer !== undefined).map((s) => s.timer);
    expect(timers).toContain(25);
  });

  it("step text contains the instruction before the timer", () => {
    const packs = parseScript(SAMPLE_SCRIPT);
    const firstStep = packs[0].steps[0];
    expect(firstStep.text).toContain("Open your first pack");
  });

  it("handles pack with no timer", () => {
    const packs = parseScript(SAMPLE_SCRIPT);
    const thirdPack = packs[2];
    expect(thirdPack.steps.some((s) => s.text.includes("Open your third pack"))).toBe(true);
  });

  it("returns empty array for empty input", () => {
    expect(parseScript("")).toHaveLength(0);
  });

  it("returns empty array if no pack markers found", () => {
    expect(parseScript("Just some text with no markers")).toHaveLength(0);
  });

  it("strips lines that are just 'Draft'", () => {
    const script = "<START FIRST PACK>\nDraft\nPick a card.\n<Time 10>\n<END FIRST PACK>";
    const packs = parseScript(script);
    const allText = packs[0].steps.map((s) => s.text).join(" ");
    expect(allText).not.toContain("Draft");
  });
});

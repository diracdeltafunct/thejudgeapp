import scriptRaw from "../data/draftscript.txt?raw";
import { getPackSize, type PackSize } from "../theme.js";

interface DraftStep {
  text: string;
  timer?: number;
}

interface DraftPack {
  name: string;
  steps: DraftStep[];
}

export function parseScript(raw: string): DraftPack[] {
  const packRegex =
    /<START (FIRST|SECOND|THIRD) PACK>([\s\S]*?)<END (FIRST|SECOND|THIRD) PACK>/g;
  const packNames: Record<string, string> = {
    FIRST: "Pack 1",
    SECOND: "Pack 2",
    THIRD: "Pack 3",
  };
  const packs: DraftPack[] = [];
  let match: RegExpExecArray | null;

  while ((match = packRegex.exec(raw)) !== null) {
    const packName = packNames[match[1]];
    const packContent = match[2];
    const steps: DraftStep[] = [];
    const timeRegex = /<Time (\d+)>/g;
    let lastIndex = 0;
    let timeMatch: RegExpExecArray | null;

    while ((timeMatch = timeRegex.exec(packContent)) !== null) {
      const textBefore = packContent.slice(lastIndex, timeMatch.index);
      const cleanText = textBefore
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l && l !== "Draft")
        .join("\n");
      steps.push({ text: cleanText || "", timer: parseInt(timeMatch[1]) });
      lastIndex = timeMatch.index + timeMatch[0].length;
    }

    const remaining = packContent.slice(lastIndex);
    const cleanRemaining = remaining
      .split("\n")
      .map((l) => l.trim())
      .filter((l) => l && l !== "Draft")
      .join("\n");
    if (cleanRemaining) {
      steps.push({ text: cleanRemaining });
    }

    packs.push({ name: packName, steps });
  }

  return packs;
}

let audioCtx: AudioContext | null = null;

function getAudioCtx(): AudioContext {
  if (!audioCtx) audioCtx = new AudioContext();
  return audioCtx;
}

function vibrate(pattern: number | number[]): void {
  if (_haptic) navigator.vibrate?.(pattern);
}

function playBeep(): void {
  const ctx = getAudioCtx();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.type = "sine";
  osc.frequency.setValueAtTime(880, ctx.currentTime);
  gain.gain.setValueAtTime(0.3, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.15);
  osc.start(ctx.currentTime);
  osc.stop(ctx.currentTime + 0.15);
  vibrate(100);
}

function playDing(): void {
  const ctx = getAudioCtx();
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.type = "sine";
  osc.frequency.setValueAtTime(1320, ctx.currentTime);
  gain.gain.setValueAtTime(0.4, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.8);
  osc.start(ctx.currentTime);
  osc.stop(ctx.currentTime + 0.8);
  vibrate([200, 100, 200]);
}

function buildPacks(size: PackSize): DraftPack[] {
  const base = parseScript(scriptRaw);
  if (size === 14) return base;
  return base.map((pack) => {
    const step0 = pack.steps[0];
    if (!step0) return pack;
    const newText = step0.text.replace(/\b14\b/g, "15").replace(/\b13\b/g, "14");
    return { ...pack, steps: [{ text: newText, timer: step0.timer }, ...pack.steps] };
  });
}

// Module-level state persists across navigation
let _packSize: PackSize = getPackSize();
let _packs = buildPacks(_packSize);
let _packIndex = 0;
let _stepIndex = 0;
let _timerState: "idle" | "running" | "paused" | "flash" = "idle";
let _timeRemaining = 0;
let _muted = false;
let _haptic = true;

export function initDraftGuide(container: HTMLElement): void {
  const currentSize = getPackSize();
  if (currentSize !== _packSize) {
    _packSize = currentSize;
    _packs = buildPacks(_packSize);
    _packIndex = 0;
    _stepIndex = 0;
  }
  const packs = _packs;

  let packIndex = _packIndex;
  let stepIndex = _stepIndex;
  // If we navigated away while the timer was running, treat it as paused
  let timerState: "idle" | "running" | "paused" | "flash" =
    _timerState === "running" ? "paused" : _timerState;
  let timerInterval: ReturnType<typeof setInterval> | null = null;
  let timeRemaining = _timeRemaining;
  let muted = _muted;
  let haptic = _haptic;

  function persist(): void {
    _packIndex = packIndex;
    _stepIndex = stepIndex;
    _timerState = timerState;
    _timeRemaining = timeRemaining;
    _muted = muted;
    _haptic = haptic;
  }

  function getCurrentStep(): DraftStep | null {
    if (packIndex >= packs.length) return null;
    return packs[packIndex].steps[stepIndex] ?? null;
  }

  function stopTimer(): void {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }

  function advance(): void {
    stopTimer();
    if (packIndex < packs.length) {
      stepIndex++;
      if (stepIndex >= packs[packIndex].steps.length) {
        packIndex++;
        stepIndex = 0;
      }
    }
    timerState = "idle";
    persist();
    render();
  }

  function goBack(): void {
    stopTimer();
    if (stepIndex > 0) {
      stepIndex--;
    } else if (packIndex > 0) {
      packIndex--;
      stepIndex = packs[packIndex].steps.length - 1;
    }
    timerState = "idle";
    persist();
    render();
  }

  function isAtStart(): boolean {
    return packIndex === 0 && stepIndex === 0;
  }

  function runInterval(): void {
    timerInterval = setInterval(() => {
      timeRemaining--;
      if (timeRemaining <= 0) {
        clearInterval(timerInterval!);
        timerInterval = null;
        timerState = "flash";
        if (!muted) playDing();
      } else if (timeRemaining === 11) {
        if (!muted) { playBeep(); setTimeout(playBeep, 200); }
      }
      persist();
      render();
    }, 1000);
  }

  function togglePause(): void {
    if (timerState === "running") {
      stopTimer();
      timerState = "paused";
      persist();
      render();
    } else if (timerState === "paused") {
      timerState = "running";
      persist();
      render();
      runInterval();
    }
  }

  function resetTimer(): void {
    const step = getCurrentStep();
    if (!step?.timer) return;
    stopTimer();
    timeRemaining = step.timer;
    timerState = "running";
    persist();
    render();
    runInterval();
  }

  function startTimer(): void {
    const step = getCurrentStep();
    if (!step?.timer) return;
    timeRemaining = step.timer;
    timerState = "running";
    persist();
    render();
    runInterval();
  }

  function render(): void {
    if (packIndex >= packs.length) {
      container.innerHTML = `
        <div class="draft-guide">
          <div class="draft-complete">
            <div class="draft-complete-icon">&#10003;</div>
            <h1>Draft Complete!</h1>
            <p>Good luck building your decks.</p>
            <button class="draft-btn draft-start" id="draft-reset-all">Reset Draft</button>
          </div>
        </div>
      `;
      container.querySelector("#draft-reset-all")!.addEventListener("click", () => {
        packIndex = 0;
        stepIndex = 0;
        timerState = "idle";
        timeRemaining = 0;
        persist();
        render();
      });
      return;
    }

    const step = getCurrentStep()!;

    const packIndicator = packs
      .map((p, i) => {
        const cls =
          i === packIndex ? "pack-tab active" : i < packIndex ? "pack-tab done" : "pack-tab";
        return `<div class="${cls}">${p.name}</div>`;
      })
      .join("");
    const muteIcon = muted ? "🔇" : "🔊";
    const hapticIcon = haptic ? "📳" : "📴";

    let bodyHtml: string;

    if (timerState === "flash") {
      bodyHtml = `
        <div class="draft-flash-wrap">
          <div class="draft-flash">DRAFT</div>
          <button class="draft-btn draft-start" id="draft-continue">Continue</button>
        </div>
      `;
    } else {
      const paragraphs = step.text
        .split("\n")
        .map((l) => `<p class="draft-line">${l}</p>`)
        .join("");

      if (step.timer !== undefined) {
        const isActive = timerState === "running" || timerState === "paused";
        const displayTime = isActive ? timeRemaining : step.timer;
        const urgent = timerState === "running" && timeRemaining <= 10;
        const timerCls = [
          "draft-timer",
          timerState === "running" ? "running" : "",
          urgent ? "urgent" : "",
        ]
          .filter(Boolean)
          .join(" ");

        const pauseIcon = timerState === "running" ? "&#9646;&#9646;" : "&#9654;";
        const btn =
          timerState === "idle"
            ? `<button class="draft-btn draft-start" id="draft-start">Start</button>`
            : isActive
              ? `<div class="draft-timer-controls">
                   <button class="draft-reset-btn" id="draft-pause" aria-label="${timerState === "running" ? "Pause" : "Resume"} timer">${pauseIcon}</button>
                   <button class="draft-reset-btn" id="draft-reset" aria-label="Reset timer">&#8635;</button>
                 </div>`
              : "";

        bodyHtml = `
          <div class="draft-text">${paragraphs}</div>
          <div class="${timerCls}">${displayTime}</div>
          ${btn}
        `;
      } else {
        bodyHtml = `<div class="draft-text">${paragraphs}</div>`;
      }
    }

    const atStart = isAtStart();
    container.innerHTML = `
      <div class="draft-guide">
        <div class="draft-pack-indicator">${packIndicator}</div>
        <div class="draft-indicator-sub">
          <button class="draft-mute-btn" id="draft-mute" aria-label="${muted ? "Unmute" : "Mute"}">${muteIcon}</button>
          <button class="draft-mute-btn" id="draft-haptic" aria-label="${haptic ? "Disable vibration" : "Enable vibration"}">${hapticIcon}</button>
        </div>
        <div class="draft-body">${bodyHtml}</div>
        <div class="draft-nav-bar">
          <button class="draft-nav-btn" id="draft-back" ${atStart ? "disabled" : ""}>&#8592; Back</button>
          <button class="draft-nav-btn draft-nav-next" id="draft-next">Next &#8594;</button>
        </div>
      </div>
    `;

    container.querySelector("#draft-start")?.addEventListener("click", startTimer);
    container.querySelector("#draft-continue")?.addEventListener("click", advance);
    container.querySelector("#draft-pause")?.addEventListener("click", togglePause);
    container.querySelector("#draft-reset")?.addEventListener("click", resetTimer);
    container.querySelector("#draft-back")?.addEventListener("click", goBack);
    container.querySelector("#draft-mute")?.addEventListener("click", () => {
      muted = !muted;
      persist();
      render();
    });
    container.querySelector("#draft-haptic")?.addEventListener("click", () => {
      haptic = !haptic;
      persist();
      render();
    });
    container.querySelector("#draft-next")?.addEventListener("click", advance);
  }

  render();
}

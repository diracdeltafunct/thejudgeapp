import { invoke } from "@tauri-apps/api/core";

const TIMER_PREFIX = "timer_";
const DEFAULT_TIME_KEY = "default_round_time_mins";
const ALARM_SOUND_KEY = "alarm_sound";

export type AlarmSound = "beep" | "bell" | "buzzer" | "chime" | "none";

export const ALARM_SOUND_OPTIONS: { value: AlarmSound; label: string }[] = [
  { value: "beep",   label: "Beep"   },
  { value: "bell",   label: "Bell"   },
  { value: "buzzer", label: "Buzzer" },
  { value: "chime",  label: "Chime"  },
  { value: "none",   label: "None"   },
];

export function getAlarmSound(): AlarmSound {
  const v = localStorage.getItem(ALARM_SOUND_KEY);
  return (v as AlarmSound | null) ?? "beep";
}

export function setAlarmSound(sound: AlarmSound): void {
  localStorage.setItem(ALARM_SOUND_KEY, sound);
}

interface TimerState {
  durationSecs: number;
  startedAt: number | null; // epoch ms, null = paused
  elapsedSecs: number;      // accumulated seconds while paused
  alarmFired: boolean;      // prevents re-firing on reload
}

// ── Persistence ───────────────────────────────────────────────────────────────

export function getDefaultRoundTime(): number {
  const v = localStorage.getItem(DEFAULT_TIME_KEY);
  return v ? Math.max(1, parseInt(v, 10)) : 60;
}

export function setDefaultRoundTime(mins: number): void {
  localStorage.setItem(DEFAULT_TIME_KEY, String(Math.max(1, Math.round(mins))));
}

function loadState(id: string): TimerState {
  try {
    const raw = localStorage.getItem(TIMER_PREFIX + id);
    if (raw) return JSON.parse(raw);
  } catch { /* ignore */ }
  return {
    durationSecs: getDefaultRoundTime() * 60,
    startedAt: null,
    elapsedSecs: 0,
    alarmFired: false,
  };
}

function saveState(id: string, state: TimerState): void {
  localStorage.setItem(TIMER_PREFIX + id, JSON.stringify(state));
}

export function deleteTimerState(id: string): void {
  localStorage.removeItem(TIMER_PREFIX + id);
}

// ── Time math ─────────────────────────────────────────────────────────────────

function getElapsed(state: TimerState): number {
  if (state.startedAt === null) return state.elapsedSecs;
  return state.elapsedSecs + (Date.now() - state.startedAt) / 1000;
}

function getRemaining(state: TimerState): number {
  return state.durationSecs - getElapsed(state);
}

function formatRemaining(remainingSecs: number): { text: string; overtime: boolean } {
  const overtime = remainingSecs < 0;
  const abs = Math.abs(Math.floor(remainingSecs));
  const mins = Math.floor(abs / 60);
  const secs = abs % 60;
  const text = `${overtime ? "+" : ""}${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  return { text, overtime };
}

// ── Alarm (Web Audio API) ─────────────────────────────────────────────────────

function audioCtx(): AudioContext | null {
  try {
    const Ctor = window.AudioContext ?? (window as any).webkitAudioContext;
    return Ctor ? new Ctor() : null;
  } catch { return null; }
}

function tone(
  ctx: AudioContext,
  type: OscillatorType,
  freq: number,
  startVol: number,
  offset: number,
  duration: number,
): void {
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.type = type;
  osc.frequency.value = freq;
  gain.gain.setValueAtTime(startVol, ctx.currentTime + offset);
  gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + offset + duration);
  osc.start(ctx.currentTime + offset);
  osc.stop(ctx.currentTime + offset + duration + 0.05);
}

const alarmPlayers: Record<AlarmSound, (ctx: AudioContext) => void> = {
  beep(ctx) {
    tone(ctx, "sine",   880, 0.6, 0,    0.35);
    tone(ctx, "sine",   880, 0.6, 0.45, 0.35);
    tone(ctx, "sine",   880, 0.6, 0.9,  0.35);
  },
  bell(ctx) {
    // Fundamental + overtone for a bell-like timbre, long decay
    tone(ctx, "sine",   880, 0.5, 0, 2.5);
    tone(ctx, "sine",  2200, 0.2, 0, 1.5);
  },
  buzzer(ctx) {
    tone(ctx, "square", 220, 0.4, 0,    0.18);
    tone(ctx, "square", 220, 0.4, 0.22, 0.18);
    tone(ctx, "square", 220, 0.4, 0.44, 0.35);
  },
  chime(ctx) {
    tone(ctx, "sine", 1047, 0.5, 0,    0.6);
    tone(ctx, "sine",  784, 0.5, 0.35, 0.6);
    tone(ctx, "sine",  523, 0.5, 0.7,  0.9);
  },
  none() { /* silent */ },
};

export function playAlarm(): void {
  const sound = getAlarmSound();
  if (sound === "none") return;
  const ctx = audioCtx();
  if (!ctx) return;
  try { alarmPlayers[sound](ctx); } catch { /* Web Audio not available */ }
}

// ── Push notification ─────────────────────────────────────────────────────────

async function sendRoundEndNotification(tournamentName: string): Promise<void> {
  if (!("Notification" in window)) return;
  if (Notification.permission === "denied") return;
  try {
    if (Notification.permission !== "granted") {
      const result = await Notification.requestPermission();
      if (result !== "granted") return;
    }
    new Notification("Round Over", {
      body: `The round for "${tournamentName}" has ended.`,
    });
  } catch { /* notifications not supported in this environment */ }
}

// ── Active interval registry ──────────────────────────────────────────────────

const activeIntervals = new Map<string, number>();

export function clearAllTimerIntervals(): void {
  activeIntervals.forEach((intervalId) => clearInterval(intervalId));
  activeIntervals.clear();
}

// ── Purple Fox parsing ────────────────────────────────────────────────────────

function parsePurpleFoxTime(mmss: string): number | null {
  // Expects "MM:SS" returned from the hidden webview's JS extraction.
  const match = mmss.trim().match(/^(\d{1,3}):(\d{2})$/);
  if (!match) return null;
  const mins = parseInt(match[1], 10);
  const secs = parseInt(match[2], 10);
  if (isNaN(mins) || isNaN(secs)) return null;
  return mins * 60 + secs;
}

// ── Card initialisation ───────────────────────────────────────────────────────

function applyDisplay(
  displayEl: HTMLElement,
  startBtn: HTMLButtonElement,
  remaining: number,
  running: boolean,
): void {
  const { text, overtime } = formatRemaining(remaining);
  displayEl.textContent = text;
  displayEl.classList.toggle("timer-overtime", overtime);
  startBtn.textContent = running ? "⏸" : "▶";
  startBtn.setAttribute("aria-label", running ? "Pause timer" : "Start timer");
}

export function initTimerCard(
  tournamentId: string,
  tournamentName: string,
  card: HTMLElement,
): void {
  const displayEl = card.querySelector<HTMLElement>(".timer-display");
  const startBtn = card.querySelector<HTMLButtonElement>(".timer-start-btn");
  const resetBtn = card.querySelector<HTMLButtonElement>(".timer-reset-btn");
  if (!displayEl || !startBtn || !resetBtn) return;

  // ── Tick ────────────────────────────────────────────────────────────────────
  function tick(): void {
    const state = loadState(tournamentId);
    const remaining = getRemaining(state);
    applyDisplay(displayEl!, startBtn!, remaining, state.startedAt !== null);

    if (remaining <= 0 && !state.alarmFired) {
      state.alarmFired = true;
      saveState(tournamentId, state);
      playAlarm();
      sendRoundEndNotification(tournamentName);
    }
  }

  function startTicking(): void {
    if (activeIntervals.has(tournamentId)) return;
    const id = window.setInterval(tick, 500);
    activeIntervals.set(tournamentId, id);
  }

  function stopTicking(): void {
    const id = activeIntervals.get(tournamentId);
    if (id !== undefined) {
      clearInterval(id);
      activeIntervals.delete(tournamentId);
    }
  }

  // ── Initial render ───────────────────────────────────────────────────────────
  const initial = loadState(tournamentId);
  applyDisplay(displayEl, startBtn, getRemaining(initial), initial.startedAt !== null);
  if (initial.startedAt !== null) startTicking();

  // ── Start / Pause ────────────────────────────────────────────────────────────
  startBtn.addEventListener("click", () => {
    const s = loadState(tournamentId);
    if (s.startedAt !== null) {
      // Pause
      s.elapsedSecs += (Date.now() - s.startedAt) / 1000;
      s.startedAt = null;
      saveState(tournamentId, s);
      stopTicking();
      applyDisplay(displayEl, startBtn, getRemaining(s), false);
    } else {
      // Request notification permission contextually on first start
      if ("Notification" in window && Notification.permission === "default") {
        Notification.requestPermission();
      }
      s.startedAt = Date.now();
      saveState(tournamentId, s);
      startTicking();
      tick();
    }
  });

  // ── Reset ────────────────────────────────────────────────────────────────────
  resetBtn.addEventListener("click", () => {
    stopTicking();
    const fresh: TimerState = {
      durationSecs: getDefaultRoundTime() * 60,
      startedAt: null,
      elapsedSecs: 0,
      alarmFired: false,
    };
    saveState(tournamentId, fresh);
    applyDisplay(displayEl, startBtn, fresh.durationSecs, false);
  });

  // ── Edit overlay ─────────────────────────────────────────────────────────────
  const editOverlay = card.querySelector<HTMLElement>(".timer-edit-overlay");
  if (editOverlay !== null) {
    const overlay = editOverlay;
    const digitEls = Array.from(overlay.querySelectorAll<HTMLElement>(".digit-val"));
    const maxPerDigit = [9, 9, 5, 9];
    let digits = [0, 0, 0, 0];

    function updateDigitDisplay(): void {
      digitEls.forEach((el, i) => { el.textContent = String(digits[i]); });
    }

    let wasRunning = false;

    function openEdit(): void {
      const state = loadState(tournamentId);
      wasRunning = state.startedAt !== null;
      const rem = Math.max(0, Math.ceil(getRemaining(state)));
      const mins = Math.floor(rem / 60);
      const secs = rem % 60;
      digits = [Math.floor(mins / 10), mins % 10, Math.floor(secs / 10), secs % 10];
      updateDigitDisplay();
      overlay.hidden = false;
    }

    function closeEdit(): void {
      overlay.hidden = true;
    }

    // Double-click to open
    displayEl.addEventListener("dblclick", (e) => { e.preventDefault(); openEdit(); });

    // Long-press to open
    let longPressTimer: number | null = null;
    displayEl.addEventListener("pointerdown", () => {
      longPressTimer = window.setTimeout(openEdit, 600);
    });
    const cancelLongPress = () => {
      if (longPressTimer !== null) { clearTimeout(longPressTimer); longPressTimer = null; }
    };
    displayEl.addEventListener("pointerup", cancelLongPress);
    displayEl.addEventListener("pointerleave", cancelLongPress);
    displayEl.addEventListener("pointermove", cancelLongPress);

    // Digit +/- buttons
    overlay.querySelectorAll<HTMLButtonElement>(".digit-inc").forEach(btn => {
      btn.addEventListener("click", () => {
        const i = parseInt(btn.dataset.idx!, 10);
        digits[i] = digits[i] >= maxPerDigit[i] ? 0 : digits[i] + 1;
        updateDigitDisplay();
      });
    });
    overlay.querySelectorAll<HTMLButtonElement>(".digit-dec").forEach(btn => {
      btn.addEventListener("click", () => {
        const i = parseInt(btn.dataset.idx!, 10);
        digits[i] = digits[i] <= 0 ? maxPerDigit[i] : digits[i] - 1;
        updateDigitDisplay();
      });
    });

    // OK — set the entered time as the new duration, preserving running state
    overlay.querySelector<HTMLButtonElement>(".timer-edit-ok")?.addEventListener("click", () => {
      const totalSecs = digits[0] * 600 + digits[1] * 60 + digits[2] * 10 + digits[3];
      stopTicking();
      const edited: TimerState = {
        durationSecs: totalSecs,
        startedAt: wasRunning ? Date.now() : null,
        elapsedSecs: 0,
        alarmFired: false,
      };
      saveState(tournamentId, edited);
      applyDisplay(displayEl!, startBtn!, totalSecs, wasRunning);
      if (wasRunning) startTicking();
      closeEdit();
    });

    overlay.querySelector<HTMLButtonElement>(".timer-edit-cancel")?.addEventListener("click", closeEdit);
  }

  // ── Purple Fox sync ───────────────────────────────────────────────────────────
  const syncBtn = card.querySelector<HTMLButtonElement>(".timer-sync-btn");
  if (syncBtn) {
    syncBtn.addEventListener("click", async () => {
      const url = syncBtn.dataset.url;
      if (!url) return;

      const prev = syncBtn.textContent!;
      syncBtn.textContent = "…";
      syncBtn.disabled = true;

      try {
        const mmss = await invoke<string>("sync_purple_fox_timer", { url });
        const secs = parsePurpleFoxTime(mmss);
        if (secs === null || secs <= 0) {
          syncBtn.textContent = "✕";
          setTimeout(() => { syncBtn.textContent = prev; }, 2000);
          return;
        }

        stopTicking();
        if ("Notification" in window && Notification.permission === "default") {
          Notification.requestPermission();
        }
        const synced: TimerState = {
          durationSecs: secs,
          startedAt: Date.now(),
          elapsedSecs: 0,
          alarmFired: false,
        };
        saveState(tournamentId, synced);
        startTicking();
        tick();
        syncBtn.textContent = "✓";
        setTimeout(() => { syncBtn.textContent = prev; }, 1500);
      } catch {
        syncBtn.textContent = "✕";
        setTimeout(() => { syncBtn.textContent = prev; }, 2000);
      } finally {
        syncBtn.disabled = false;
      }
    });
  }
}

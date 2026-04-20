import { invoke } from "@tauri-apps/api/core";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
  createChannel,
  registerActionTypes,
  onAction,
} from "@tauri-apps/plugin-notification";

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

export function getAlarmSound(): string {
  return localStorage.getItem(ALARM_SOUND_KEY) ?? "beep";
}

export function setAlarmSound(sound: string): void {
  localStorage.setItem(ALARM_SOUND_KEY, sound);
}

function isSystemAlarmUri(sound: string): boolean {
  return sound.startsWith("content://") || sound.startsWith("file://");
}

export function stopSystemAlarm(): void {
  try { (window as any).__AlarmSounds__?.stopAlarmSound(); } catch { /* no bridge */ }
}

export function stopAlarmPreview(): void {
  stopWebAudio();
  stopSystemAlarm();
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

let _activeAudioCtx: AudioContext | null = null;

function audioCtx(): AudioContext | null {
  try {
    if (_activeAudioCtx && _activeAudioCtx.state !== "closed") return _activeAudioCtx;
    const Ctor = window.AudioContext ?? (window as any).webkitAudioContext;
    _activeAudioCtx = Ctor ? new Ctor() : null;
    return _activeAudioCtx;
  } catch { return null; }
}

function stopWebAudio(): void {
  _activeAudioCtx?.close().catch(() => {});
  _activeAudioCtx = null;
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
  if (isSystemAlarmUri(sound)) {
    try { (window as any).__AlarmSounds__?.playAlarmSound(sound); } catch { /* no bridge */ }
    return;
  }
  // Android WebView doesn't request audio focus for Web Audio automatically.
  // Without it, Android silently mutes the oscillators. Request focus first.
  try { (window as any).__AlarmSounds__?.requestFocusForWebAudio(); } catch { /* no bridge on non-Android */ }
  const ctx = audioCtx();
  if (!ctx) return;
  const player = alarmPlayers[sound as AlarmSound];
  if (!player) return;
  const doPlay = () => { try { player(ctx); } catch { /* Web Audio not available */ } };
  if (ctx.state === "suspended") {
    ctx.resume().then(doPlay).catch(() => {});
  } else {
    doPlay();
  }
}

// ── Push notification ─────────────────────────────────────────────────────────

const ALARM_ACTION_TYPE = "round-alarm";
const ALARM_CHANNEL_ID = "round-alarm-channel";

// Create the notification channel (Android 8+ requires this; no-op on other platforms).
// Store the promise so sendRoundEndNotification can await it before sending.
const channelReady: Promise<void> = createChannel({
  id: ALARM_CHANNEL_ID,
  name: "Round Alarms",
  description: "Notifies when a tournament round ends",
  importance: 4,
  vibration: true,
}).catch(() => {});

// Register the "Stop Alarm" action type and listen for it — runs once at module load.
registerActionTypes([{
  id: ALARM_ACTION_TYPE,
  actions: [{ id: "stop", title: "Stop Alarm", foreground: true }],
}]).catch(() => {});

onAction((action) => {
  const extra = (action as any).notification?.extra as Record<string, string> | undefined;
  const id = extra?.tournamentId;
  if (id) stopAlarmLoop(id);
  stopSystemAlarm();
}).catch(() => {});

async function sendRoundEndNotification(tournamentId: string, tournamentName: string): Promise<void> {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (!granted) return;
    await channelReady;
    sendNotification({
      title: "Round Over",
      body: `The round for "${tournamentName}" has ended.`,
      channelId: ALARM_CHANNEL_ID,
      actionTypeId: ALARM_ACTION_TYPE,
      extra: { tournamentId },
    });
  } catch { /* notifications not supported in this environment */ }
}

// ── Active interval registry ──────────────────────────────────────────────────

const activeIntervals = new Map<string, number>();

export function clearAllTimerIntervals(): void {
  activeIntervals.forEach((intervalId) => clearInterval(intervalId));
  activeIntervals.clear();
}

// ── Alarm trigger ─────────────────────────────────────────────────────────────

function startAlarmLoop(tournamentId: string, tournamentName: string): void {
  playAlarm();
  sendRoundEndNotification(tournamentId, tournamentName);
}

export function stopAlarmLoop(tournamentId: string): void {
  void tournamentId;
  stopSystemAlarm();
}

export function clearAllAlarmLoops(): void {
  stopSystemAlarm();
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

// ── Web Timer API ─────────────────────────────────────────────────────────────

const WEB_TIMER_API_KEY = "web_timer_api_url";
const WEB_TIMER_CODE_PREFIX = "web_timer_code_";

function getWebTimerApi(): string {
  return (localStorage.getItem(WEB_TIMER_API_KEY) ?? "http://localhost:3001").replace(/\/$/, "");
}

function getLinkedCode(tournamentId: string): string | null {
  return localStorage.getItem(WEB_TIMER_CODE_PREFIX + tournamentId);
}

function setLinkedCode(tournamentId: string, code: string): void {
  localStorage.setItem(WEB_TIMER_CODE_PREFIX + tournamentId, code);
}

async function pushPlay(tournamentId: string, state: TimerState): Promise<void> {
  const code = getLinkedCode(tournamentId);
  if (!code) return;
  const remaining = getRemaining(state);
  fetch(`${getWebTimerApi()}/timer/start`, {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      id: code,
      time_started: Math.round(Date.now() / 1000),
      time_remaining_when_started: Math.max(0, Math.round(remaining)),
    }),
  }).catch(() => {});
}

async function pushPause(tournamentId: string, remainingSecs: number): Promise<void> {
  const code = getLinkedCode(tournamentId);
  if (!code) return;
  fetch(`${getWebTimerApi()}/timer/stop`, {
    method: "PUT",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      id: code,
      time_remaining_when_started: Math.max(0, Math.round(remainingSecs)),
    }),
  }).catch(() => {});
}

interface WebTimerCallbacks {
  stopTicking: () => void;
  startTicking: () => void;
  tick: () => void;
  displayEl: HTMLElement;
  startBtn: HTMLButtonElement;
}

function openWebTimerModal(tournamentId: string, cb: WebTimerCallbacks): void {
  const overlay = document.createElement("div");
  overlay.className = "web-timer-overlay";
  overlay.innerHTML = `
    <div class="web-timer-dialog">
      <div class="web-timer-header">
        <span class="web-timer-title">Web Timer</span>
        <button class="web-timer-close" aria-label="Close">✕</button>
      </div>
      <div class="web-timer-section">
        <div class="web-timer-section-label">Create</div>
        <p class="web-timer-hint">Publish your current timer and share the code</p>
        <button class="web-timer-create-btn">Create Web Timer</button>
        <div class="web-timer-result" hidden>
          <span class="web-timer-code"></span>
          <span class="web-timer-copy" title="Copy code">⧉</span>
        </div>
      </div>
      <div class="web-timer-divider"></div>
      <div class="web-timer-section">
        <div class="web-timer-section-label">Join</div>
        <p class="web-timer-hint">Enter a 6-character code to sync this timer</p>
        <div class="web-timer-join-row">
          <input class="web-timer-code-input" type="text" maxlength="6" placeholder="abc123" autocomplete="off" spellcheck="false" />
          <button class="web-timer-sync-btn">Sync</button>
        </div>
        <div class="web-timer-status"></div>
      </div>
      <div class="web-timer-api-row">
        <label class="web-timer-api-label">Server</label>
        <input class="web-timer-api-input" type="text" value="${getWebTimerApi()}" />
      </div>
    </div>
  `;

  const close = () => document.body.removeChild(overlay);
  overlay.addEventListener("click", (e) => { if (e.target === overlay) close(); });
  overlay.querySelector(".web-timer-close")!.addEventListener("click", close);

  // Keep API URL in sync
  const apiInput = overlay.querySelector<HTMLInputElement>(".web-timer-api-input")!;
  apiInput.addEventListener("change", () => {
    localStorage.setItem(WEB_TIMER_API_KEY, apiInput.value.trim());
  });

  // ── Create ──────────────────────────────────────────────────────────────────
  const createBtn = overlay.querySelector<HTMLButtonElement>(".web-timer-create-btn")!;
  const resultEl = overlay.querySelector<HTMLElement>(".web-timer-result")!;
  const codeEl = overlay.querySelector<HTMLElement>(".web-timer-code")!;
  const copyEl = overlay.querySelector<HTMLElement>(".web-timer-copy")!;

  createBtn.addEventListener("click", async () => {
    const state = loadState(tournamentId);
    const remaining = getRemaining(state);
    const running = state.startedAt !== null;
    const body: Record<string, unknown> = {
      time_length: Math.round(state.durationSecs),
      running,
      time_remaining_when_started: Math.max(0, Math.round(remaining)),
    };
    if (running && state.startedAt !== null) {
      body["time_started"] = Math.round(state.startedAt / 1000);
    }

    createBtn.disabled = true;
    const prev = createBtn.textContent;
    createBtn.textContent = "…";
    try {
      const res = await fetch(`${getWebTimerApi()}/timer`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      codeEl.textContent = data.id;
      resultEl.hidden = false;
      setLinkedCode(tournamentId, data.id);
    } catch {
      createBtn.textContent = "Failed — check server URL";
      setTimeout(() => { createBtn.textContent = prev; createBtn.disabled = false; }, 2500);
      return;
    }
    createBtn.textContent = prev;
    createBtn.disabled = false;
  });

  copyEl.addEventListener("click", () => {
    const code = codeEl.textContent ?? "";
    navigator.clipboard.writeText(code).catch(() => {});
    copyEl.textContent = "✓";
    setTimeout(() => { copyEl.textContent = "⧉"; }, 1500);
  });

  // ── Sync ────────────────────────────────────────────────────────────────────
  const codeInput = overlay.querySelector<HTMLInputElement>(".web-timer-code-input")!;
  const syncBtn = overlay.querySelector<HTMLButtonElement>(".web-timer-sync-btn")!;
  const statusEl = overlay.querySelector<HTMLElement>(".web-timer-status")!;

  codeInput.addEventListener("input", () => {
    codeInput.value = codeInput.value.toLowerCase().replace(/[^a-z0-9]/g, "");
  });

  function setStatus(msg: string, isError = false): void {
    statusEl.textContent = msg;
    statusEl.className = "web-timer-status" + (isError ? " web-timer-status-error" : " web-timer-status-ok");
  }

  syncBtn.addEventListener("click", async () => {
    const code = codeInput.value.trim();
    if (code.length !== 6) { setStatus("Enter a 6-character code", true); return; }

    syncBtn.disabled = true;
    syncBtn.textContent = "…";
    statusEl.textContent = "";

    try {
      const res = await fetch(`${getWebTimerApi()}/timer/${code}`);
      if (res.status === 404) { setStatus("Timer not found", true); return; }
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data: {
        running: boolean;
        time_started: number;
        time_length: number;
        time_remaining_when_started: number;
      } = await res.json();

      cb.stopTicking();

      let remainingSecs: number;
      if (data.running) {
        const nowSecs = Date.now() / 1000;
        const elapsedSinceStart = nowSecs - data.time_started;
        const base = data.time_remaining_when_started >= 0
          ? data.time_remaining_when_started
          : data.time_length;
        remainingSecs = base - elapsedSinceStart;
      } else {
        remainingSecs = data.time_remaining_when_started >= 0
          ? data.time_remaining_when_started
          : data.time_length;
      }

      const synced: TimerState = {
        durationSecs: Math.max(0, remainingSecs),
        startedAt: data.running ? Date.now() : null,
        elapsedSecs: 0,
        alarmFired: false,
      };
      saveState(tournamentId, synced);

      if (data.running) {
        cb.startTicking();
        cb.tick();
      } else {
        applyDisplay(cb.displayEl, cb.startBtn, synced.durationSecs, false);
      }

      setLinkedCode(tournamentId, code);
      setStatus("Synced!");
      setTimeout(close, 1200);
    } catch {
      setStatus("Failed — check server URL", true);
    } finally {
      syncBtn.disabled = false;
      syncBtn.textContent = "Sync";
    }
  });

  const existingCode = getLinkedCode(tournamentId);
  if (existingCode) {
    codeInput.value = existingCode;
    codeEl.textContent = existingCode;
    resultEl.hidden = false;
  }

  document.body.appendChild(overlay);
  codeInput.focus();
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
      startAlarmLoop(tournamentId, tournamentName);
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
  card.addEventListener("click", () => stopAlarmLoop(tournamentId));

  startBtn.addEventListener("click", () => {
    stopAlarmLoop(tournamentId);
    const s = loadState(tournamentId);
    if (s.startedAt !== null) {
      // Pause
      s.elapsedSecs += (Date.now() - s.startedAt) / 1000;
      s.startedAt = null;
      saveState(tournamentId, s);
      stopTicking();
      applyDisplay(displayEl, startBtn, getRemaining(s), false);
      pushPause(tournamentId, getRemaining(s));
    } else {
      s.startedAt = Date.now();
      saveState(tournamentId, s);
      startTicking();
      tick();
      pushPlay(tournamentId, s);
    }
  });

  // ── Reset ────────────────────────────────────────────────────────────────────
  resetBtn.addEventListener("click", () => {
    stopTicking();
    stopAlarmLoop(tournamentId);
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
  // Each digit button adds/subtracts a fixed number of seconds as an offset.
  // The display always shows (currentRemaining + offsetSecs), so the clock keeps
  // ticking through and the user's adjustments are preserved.
  const editOverlay = card.querySelector<HTMLElement>(".timer-edit-overlay");
  if (editOverlay !== null) {
    const overlay = editOverlay;
    const digitEls = Array.from(overlay.querySelectorAll<HTMLElement>(".digit-val"));
    const incBtns = Array.from(overlay.querySelectorAll<HTMLButtonElement>(".digit-inc"));
    const decBtns = Array.from(overlay.querySelectorAll<HTMLButtonElement>(".digit-dec"));
    // How many seconds each digit position represents
    const digitSecs = [600, 60, 10, 1];
    let offsetSecs = 0;
    let editTickId: number | null = null;
    let wasRunning = false;

    function syncDisplay(): void {
      const state = loadState(tournamentId);
      const rem = Math.max(0, Math.ceil(getRemaining(state)));
      const total = Math.max(0, rem + offsetSecs);
      const mins = Math.floor(total / 60);
      const secs = total % 60;
      const d = [Math.floor(mins / 10), mins % 10, Math.floor(secs / 10), secs % 10];
      digitEls.forEach((el, i) => { el.textContent = String(d[i]); });
    }

    function openEdit(): void {
      const state = loadState(tournamentId);
      wasRunning = state.startedAt !== null;
      offsetSecs = 0;
      syncDisplay();
      incBtns[3].disabled = wasRunning;
      decBtns[3].disabled = wasRunning;
      if (wasRunning) editTickId = window.setInterval(syncDisplay, 500);
      overlay.hidden = false;
    }

    function closeEdit(): void {
      if (editTickId !== null) { clearInterval(editTickId); editTickId = null; }
      incBtns[3].disabled = false;
      decBtns[3].disabled = false;
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

    // Digit +/- buttons adjust the offset and refresh the display
    incBtns.forEach((btn, i) => {
      btn.addEventListener("click", () => { offsetSecs += digitSecs[i]; syncDisplay(); });
    });
    decBtns.forEach((btn, i) => {
      btn.addEventListener("click", () => { offsetSecs -= digitSecs[i]; syncDisplay(); });
    });

    // OK — apply offset to the current remaining time
    overlay.querySelector<HTMLButtonElement>(".timer-edit-ok")?.addEventListener("click", () => {
      const state = loadState(tournamentId);
      const rem = Math.max(0, Math.ceil(getRemaining(state)));
      const totalSecs = Math.max(0, rem + offsetSecs);
      stopTicking();
      stopAlarmLoop(tournamentId);
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

  // ── Web Timer ─────────────────────────────────────────────────────────────────
  const webTimerBtn = card.querySelector<HTMLButtonElement>(".timer-web-btn");
  if (webTimerBtn) {
    webTimerBtn.addEventListener("click", () => {
      openWebTimerModal(tournamentId, { stopTicking, startTicking, tick, displayEl, startBtn });
    });
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

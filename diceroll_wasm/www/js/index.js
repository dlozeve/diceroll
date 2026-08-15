import init, { Session } from "../pkg/diceroll_wasm.js";
import { createKeypad } from "./keypad.js";
import { createTerminal } from "./terminal.js";
import { generateSeed, readSessionState, sessionUrl } from "./url-state.js";
import { trackViewportInsets } from "./viewport.js";

const STATS_SAMPLES = 10000;

const input = document.getElementById("input");
const rollBtn = document.getElementById("roll-btn");
const terminal = createTerminal(document.getElementById("terminal"));

let session = null;
let sessionSeed = null;
const submittedHistory = [];
let historyIndex = 0;

trackViewportInsets();

async function syncUrl() {
  const state = sessionSeed ? { seed: sessionSeed, history: submittedHistory } : null;
  history.replaceState(null, "", await sessionUrl(window.location.href, state));
}

async function clearSession() {
  session = null;
  sessionSeed = null;
  submittedHistory.length = 0;
  historyIndex = 0;
  terminal.clear();
  await syncUrl();
}

function evaluate(line) {
  const trimmed = line.trim();
  if (!trimmed) return;

  terminal.appendEcho(line);

  const statsMatch = trimmed.match(/^stats\s+(.+)$/i);
  try {
    if (statsMatch) {
      terminal.appendText(session.stats(statsMatch[1], STATS_SAMPLES));
    } else {
      terminal.appendRoll(session.rollSpans(trimmed));
    }
  } catch (e) {
    terminal.appendText(e.message ?? String(e), "error");
  }

  terminal.scrollToBottom();
}

async function submit(line) {
  const trimmed = line.trim();
  if (!trimmed) return;

  if (trimmed === "clear") {
    await clearSession();
    return;
  }

  if (!session) {
    sessionSeed = generateSeed();
    session = new Session(sessionSeed);
  }

  submittedHistory.push(line);
  historyIndex = submittedHistory.length;
  await syncUrl();
  evaluate(line);
}

// A shared link carries a seed and the expressions that were rolled against it;
// replaying them reproduces the transcript exactly.
async function restoreSessionFromUrl() {
  const state = await readSessionState(window.location.href);
  if (!state) {
    await clearSession();
    return;
  }

  try {
    session = new Session(state.seed);
    sessionSeed = state.seed;
  } catch {
    await clearSession();
    return;
  }

  submittedHistory.splice(0, submittedHistory.length, ...state.history);
  historyIndex = submittedHistory.length;

  await syncUrl();
  terminal.clear();
  submittedHistory.forEach((line) => evaluate(line));
}

function recallPrevious() {
  if (historyIndex > 0) {
    historyIndex--;
    input.value = submittedHistory[historyIndex];
    requestAnimationFrame(() =>
      input.setSelectionRange(input.value.length, input.value.length),
    );
  }
}

function recallNext() {
  if (historyIndex < submittedHistory.length - 1) {
    historyIndex++;
    input.value = submittedHistory[historyIndex];
  } else {
    historyIndex = submittedHistory.length;
    input.value = "";
  }
}

function rollFromInput() {
  void submit(input.value);
  input.value = "";
  input.focus();
}

input.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    rollFromInput();
  } else if (e.key === "ArrowUp") {
    recallPrevious();
    e.preventDefault();
  } else if (e.key === "ArrowDown") {
    recallNext();
    e.preventDefault();
  }
});

rollBtn.addEventListener("click", rollFromInput);

const infoBtn = document.getElementById("info-btn");
const hint = document.getElementById("hint");
const infoBtnLabel = document.getElementById("info-btn-label");
infoBtn.addEventListener("click", () => {
  const open = hint.classList.toggle("open");
  infoBtn.setAttribute("aria-expanded", String(open));
  infoBtnLabel.textContent = open ? "Close" : "Help";
});

const keypad = createKeypad({
  root: document.getElementById("keypad"),
  numbers: document.getElementById("keypad-numbers"),
  modifiers: document.getElementById("keypad-modifiers"),
  utility: document.getElementById("keypad-utility"),
  input,
  actions: {
    roll: rollFromInput,
    clear: () => void submit("clear"),
    recallPrevious,
    recallNext,
  },
});

// Initialize WASM — input and buttons start disabled (see HTML)
try {
  await init();
  await restoreSessionFromUrl();
  input.disabled = false;
  rollBtn.disabled = false;
  keypad.setEnabled(true);
  input.focus();
} catch (e) {
  terminal.appendText("Failed to load the dice engine. Please reload the page.", "error");
}

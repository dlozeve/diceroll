import init, { Session } from "./pkg/diceroll_wasm.js";

const STATS_SAMPLES = 10000;
const URL_SEED_KEY = "seed";
const URL_HISTORY_KEY = "h";

const terminal = document.getElementById("terminal");
const input = document.getElementById("input");
let session = null;
let sessionSeed = null;
const submittedHistory = [];
let historyIndex = 0;

function appendLine({ classes = [], echo = false } = {}) {
  const div = document.createElement("div");
  div.classList.add("line", ...classes);
  if (echo) div.classList.add("echo");
  terminal.appendChild(div);
  return div;
}

function scrollTerminalToBottom() {
  terminal.scrollTop = terminal.scrollHeight;
}

function appendEcho(line) {
  const div = appendLine({ echo: true });
  const prompt = document.createElement("span");
  prompt.className = "prompt";
  prompt.textContent = ">>> ";
  div.appendChild(prompt);
  div.appendChild(document.createTextNode(line));
}

function appendText(text, ...classes) {
  const div = appendLine({ classes });
  div.textContent = text;
}

function appendRoll(result) {
  const div = appendLine();
  renderTerms(div, result.terms);
  const total = document.createElement("span");
  total.className = "total";
  total.textContent = ` = ${result.total}`;
  div.appendChild(total);
}

function renderTerms(parent, terms) {
  terms.forEach((term, idx) => {
    parent.appendChild(document.createTextNode(termOperator(term, idx)));
    if (term.kind === "dice") {
      renderDice(parent, term);
    } else if (term.kind === "const") {
      parent.appendChild(document.createTextNode(String(term.value)));
    } else if (term.kind === "group") {
      parent.appendChild(document.createTextNode("("));
      renderTerms(parent, term.terms);
      parent.appendChild(document.createTextNode(")"));
      if (term.multiplier !== 1) {
        parent.appendChild(document.createTextNode(` * ${term.multiplier}`));
      }
    }
  });
}

function termOperator(term, idx) {
  if (term.sign < 0) return idx === 0 ? "-" : " - ";
  return idx === 0 ? "" : " + ";
}

function renderDice(parent, term) {
  let header = `${term.count}d${term.sides}`;
  if (term.modifier != null) {
    header += Array.isArray(term.modifier) ? term.modifier.join("") : term.modifier;
  }
  parent.appendChild(document.createTextNode(header + "["));
  term.rolls.forEach((roll, i) => {
    if (i > 0) parent.appendChild(document.createTextNode(","));
    const kept = term.kept[i];
    if (!kept) parent.appendChild(document.createTextNode("{"));
    parent.appendChild(rollNode(roll, term.sides));
    if (!kept) parent.appendChild(document.createTextNode("}"));
  });
  parent.appendChild(document.createTextNode("]"));
}

function rollNode(roll, sides) {
  // sides is a number for numeric dice and "F" for Fate dice; only highlight numerics.
  if (typeof sides === "number") {
    if (roll === 1 || roll === sides) {
      const span = document.createElement("span");
      span.className = roll === 1 ? "nat-1" : "nat-max";
      span.textContent = String(roll);
      span.setAttribute("aria-label", roll === 1 ? `${roll} (natural 1)` : `${roll} (critical)`);
      return span;
    }
  }
  return document.createTextNode(String(roll));
}

function generateSeed() {
  const bytes = new Uint32Array(2);
  crypto.getRandomValues(bytes);
  return Array.from(bytes, (n) => n.toString(16).padStart(8, "0")).join("");
}

function bytesToBase64Url(bytes) {
  let binary = "";
  for (const byte of bytes) {
    binary += String.fromCharCode(byte);
  }
  return btoa(binary).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
}

function base64UrlToBytes(text) {
  let base64 = text.replaceAll("-", "+").replaceAll("_", "/");
  while (base64.length % 4) base64 += "=";
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

async function encodeHistory(history) {
  const json = JSON.stringify(history);
  const stream = new Blob([json]).stream().pipeThrough(new CompressionStream("gzip"));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  return bytesToBase64Url(bytes);
}

async function decodeHistory(text) {
  const bytes = base64UrlToBytes(text);
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
  const json = await new Response(stream).text();
  const value = JSON.parse(json);
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string")) {
    throw new Error("invalid history");
  }
  return value;
}

async function readSessionFromUrl() {
  const params = new URL(window.location.href).searchParams;
  const seed = params.get(URL_SEED_KEY);
  const historyText = params.get(URL_HISTORY_KEY);

  if (!seed && !historyText) {
    return null;
  }

  if (!seed) {
    return null;
  }

  let parsedHistory = [];
  if (historyText) {
    try {
      parsedHistory = await decodeHistory(historyText);
    } catch {
      return null;
    }
  }

  return { seed, history: parsedHistory };
}

async function syncUrl() {
  const url = new URL(window.location.href);
  if (sessionSeed) {
    url.searchParams.set(URL_SEED_KEY, sessionSeed);
    url.searchParams.set(URL_HISTORY_KEY, await encodeHistory(submittedHistory));
  } else {
    url.searchParams.delete(URL_SEED_KEY);
    url.searchParams.delete(URL_HISTORY_KEY);
  }
  history.replaceState(null, "", url);
}

function startSession(seed) {
  sessionSeed = seed;
  session = new Session(seed);
}

async function clearSession() {
  session = null;
  sessionSeed = null;
  submittedHistory.length = 0;
  historyIndex = 0;
  terminal.replaceChildren();
  await syncUrl();
}

function evaluate(line) {
  const trimmed = line.trim();
  if (!trimmed) return;

  appendEcho(line);

  const statsMatch = trimmed.match(/^stats\s+(.+)$/i);
  try {
    if (statsMatch) {
      appendText(session.stats(statsMatch[1], STATS_SAMPLES));
    } else {
      appendRoll(session.rollJson(trimmed));
    }
  } catch (e) {
    appendText(e.message ?? String(e), "error");
  }

  scrollTerminalToBottom();
}

async function submit(line) {
  const trimmed = line.trim();
  if (!trimmed) return;

  if (trimmed === "clear") {
    await clearSession();
    return;
  }

  if (!session) {
    startSession(generateSeed());
  }

  submittedHistory.push(line);
  historyIndex = submittedHistory.length;
  await syncUrl();
  evaluate(line);
}

async function restoreSessionFromUrl() {
  const state = await readSessionFromUrl();
  if (!state) {
    await clearSession();
    return;
  }

  submittedHistory.splice(0, submittedHistory.length, ...state.history);
  historyIndex = submittedHistory.length;

  try {
    startSession(state.seed);
  } catch {
    await clearSession();
    return;
  }

  await syncUrl();
  terminal.replaceChildren();
  submittedHistory.forEach((line) => evaluate(line));
}

const infoBtn = document.getElementById("info-btn");
const hint = document.getElementById("hint");
const infoBtnLabel = document.getElementById("info-btn-label");
infoBtn.addEventListener("click", () => {
  const open = hint.classList.toggle("open");
  infoBtn.setAttribute("aria-expanded", String(open));
  infoBtnLabel.textContent = open ? "Close" : "Help";
});

input.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    const value = input.value;
    void submit(value);
    input.value = "";
  } else if (e.key === "ArrowUp") {
    if (historyIndex > 0) {
      historyIndex--;
      input.value = submittedHistory[historyIndex];
      requestAnimationFrame(() =>
        input.setSelectionRange(input.value.length, input.value.length),
      );
    }
    e.preventDefault();
  } else if (e.key === "ArrowDown") {
    if (historyIndex < submittedHistory.length - 1) {
      historyIndex++;
      input.value = submittedHistory[historyIndex];
    } else {
      historyIndex = submittedHistory.length;
      input.value = "";
    }
    e.preventDefault();
  }
});

// Mobile: Roll button
document.getElementById("roll-btn").addEventListener("click", () => {
  void submit(input.value);
  input.value = "";
  input.focus();
});


// Mobile: quick dice bar
const QUICK_DICE = [
  { label: "d4",       expr: "d4" },
  { label: "d6",       expr: "d6" },
  { label: "d8",       expr: "d8" },
  { label: "d10",      expr: "d10" },
  { label: "d12",      expr: "d12" },
  { label: "d20",      expr: "d20" },
  { label: "d100",     expr: "d100" },
  { label: "2d6",      expr: "2d6" },
  { label: "4d6kh3",   expr: "4d6kh3" },
  { label: "2d20kh1",  expr: "2d20kh1" },
  { label: "2d20kl1",  expr: "2d20kl1" },
];

const diceBar = document.getElementById("dice-bar");
QUICK_DICE.forEach(({ label, expr }) => {
  const btn = document.createElement("button");
  btn.className = "dice-btn";
  btn.textContent = label;
  btn.setAttribute("aria-label", `Roll ${expr}`);
  btn.disabled = true;
  btn.addEventListener("click", () => {
    submit(expr);
  });
  diceBar.appendChild(btn);
});

// Initialize WASM — input and buttons start disabled (see HTML)
try {
  await init();
  await restoreSessionFromUrl();
  input.disabled = false;
  document.getElementById("roll-btn").disabled = false;
  diceBar.querySelectorAll(".dice-btn").forEach((btn) => (btn.disabled = false));
  input.focus();
} catch (e) {
  appendText("Failed to load the dice engine. Please reload the page.", "error");
}

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

function updateViewportInsets() {
  const viewport = window.visualViewport;
  const keyboardInset = viewport
    ? Math.max(0, window.innerHeight - viewport.height - viewport.offsetTop)
    : 0;
  document.documentElement.style.setProperty(
    "--keyboard-inset",
    `${keyboardInset}px`,
  );
}

if (window.visualViewport) {
  window.visualViewport.addEventListener("resize", updateViewportInsets);
  window.visualViewport.addEventListener("scroll", updateViewportInsets);
}
window.addEventListener("resize", updateViewportInsets);
updateViewportInsets();

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

const rollBtn = document.getElementById("roll-btn");
rollBtn.addEventListener("click", rollFromInput);


// Mobile: custom dice keypad. Touch devices get a keypad holding exactly the
// tokens the dice notation uses, instead of the device keyboard; desktop keeps
// the plain text field. Visibility is driven by CSS (see #keypad in the HTML),
// the matching media query below only decides whether to suppress the device
// keyboard with inputmode="none".
const STATS_PREFIX = "stats ";

const keypad = document.getElementById("keypad");
const numbersPane = document.getElementById("keypad-numbers");
const modifiersPane = document.getElementById("keypad-modifiers");
const utilityRow = document.getElementById("keypad-utility");

function insertText(text) {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? input.value.length;
  input.setRangeText(text, start, end, "end");
}

function backspace() {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? input.value.length;
  // Delete the selection when there is one, otherwise the character before the caret
  if (start === end && start === 0) return;
  input.setRangeText("", start === end ? start - 1 : start, end, "end");
}

function toggleStatsPrefix() {
  input.value = input.value.startsWith(STATS_PREFIX)
    ? input.value.slice(STATS_PREFIX.length)
    : STATS_PREFIX + input.value;
  input.setSelectionRange(input.value.length, input.value.length);
}

function clearTranscript() {
  input.value = "";
  void submit("clear");
}

const insertKey = (text, label = text) => ({ label, insert: text });
const backspaceKey = { label: "⌫", ariaLabel: "Backspace", action: backspace };
const rollKey = {
  label: "Roll",
  ariaLabel: "Roll",
  className: "key-roll",
  action: rollFromInput,
};

const NUMBER_KEYS = [
  insertKey("7"), insertKey("8"), insertKey("9"), insertKey("d"), backspaceKey,
  insertKey("4"), insertKey("5"), insertKey("6"), insertKey("+"), insertKey("("),
  insertKey("1"), insertKey("2"), insertKey("3"), insertKey("-", "−"), insertKey(")"),
  insertKey("0"), insertKey("kh"), insertKey("kl"), insertKey("d%"), rollKey,
];

const MODIFIER_KEYS = [
  insertKey("kh"), insertKey("kl"), insertKey("dh"), insertKey("dl"), backspaceKey,
  insertKey("min"), insertKey("max"), insertKey("r"), insertKey("ro"), insertKey("!"),
  insertKey("c>"), insertKey("c>="), insertKey("c<"), insertKey("c<="), insertKey("*"),
  insertKey("dF"), insertKey("d%"),
  { label: "stats", ariaLabel: "Toggle the stats prefix", action: toggleStatsPrefix },
  { label: "clear", ariaLabel: "Clear the transcript", action: clearTranscript },
  rollKey,
];

function createKey({ label, insert, action, ariaLabel, className }) {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "key";
  if (className) btn.classList.add(...className.split(" "));
  else if (label.length > 2) btn.classList.add("key-word");
  btn.textContent = label;
  btn.setAttribute("aria-label", ariaLabel ?? `Insert ${insert}`);
  btn.disabled = true;
  btn.addEventListener("click", insert === undefined ? action : () => insertText(insert));
  return btn;
}

function renderPane(pane, keys) {
  pane.replaceChildren(...keys.map(createKey));
}

renderPane(numbersPane, NUMBER_KEYS);
renderPane(modifiersPane, MODIFIER_KEYS);

let modifiersVisible = false;

function togglePane() {
  modifiersVisible = !modifiersVisible;
  numbersPane.hidden = modifiersVisible;
  modifiersPane.hidden = !modifiersVisible;
  paneKey.textContent = modifiersVisible ? "123" : "mods";
  paneKey.setAttribute("aria-pressed", String(modifiersVisible));
  paneKey.setAttribute(
    "aria-label",
    modifiersVisible ? "Show the number keys" : "Show the modifier keys",
  );
  // Asking for a pane means asking for the keypad back
  setNativeKeyboard(false);
}

// The device keyboard stays suppressed unless the user asks for it, for the
// expressions the keypad cannot compose.
const touchLayout = window.matchMedia("(hover: none) and (pointer: coarse)");
let nativeKeyboard = false;

function applyInputMode() {
  if (touchLayout.matches && !nativeKeyboard) {
    input.inputMode = "none";
  } else {
    input.removeAttribute("inputmode");
  }
}

// While the device keyboard is up the keypad collapses to its utility row, so
// the two keyboards never stack (see body.native-keyboard in the stylesheet).
function setNativeKeyboard(on) {
  if (nativeKeyboard === on) return;
  nativeKeyboard = on;
  applyInputMode();
  document.body.classList.toggle("native-keyboard", on);
  keyboardKey.setAttribute("aria-pressed", String(on));
  // Re-focus so the device re-reads inputmode and shows or hides its keyboard
  input.blur();
  input.focus();
}

function toggleNativeKeyboard() {
  setNativeKeyboard(!nativeKeyboard);
}

touchLayout.addEventListener("change", () => {
  setNativeKeyboard(false);
  applyInputMode();
});
applyInputMode();

const paneKey = createKey({
  label: "mods",
  ariaLabel: "Show the modifier keys",
  className: "key-util key-pane",
  action: togglePane,
});
const keyboardKey = createKey({
  label: "⌨",
  ariaLabel: "Use the device keyboard",
  className: "key-util key-keyboard",
  action: toggleNativeKeyboard,
});
paneKey.setAttribute("aria-pressed", "false");
keyboardKey.setAttribute("aria-pressed", "false");

utilityRow.replaceChildren(
  paneKey,
  createKey({
    label: "↑",
    ariaLabel: "Previous expression",
    className: "key-util",
    action: recallPrevious,
  }),
  createKey({
    label: "↓",
    ariaLabel: "Next expression",
    className: "key-util",
    action: recallNext,
  }),
  keyboardKey,
);

// Tapping a key must not move focus away from the input, so the caret stays put
keypad.addEventListener("mousedown", (e) => e.preventDefault());

// Initialize WASM — input and buttons start disabled (see HTML)
try {
  await init();
  await restoreSessionFromUrl();
  input.disabled = false;
  rollBtn.disabled = false;
  keypad.querySelectorAll(".key").forEach((key) => (key.disabled = false));
  input.focus();
} catch (e) {
  appendText("Failed to load the dice engine. Please reload the page.", "error");
}

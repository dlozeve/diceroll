import init, { rollJson, stats } from "./pkg/diceroll_wasm.js";

const STATS_SAMPLES = 10000;

const terminal = document.getElementById("terminal");
const input = document.getElementById("input");
const history = [];
let historyIndex = 0;

function appendLine({ classes = [], echo = false } = {}) {
  const div = document.createElement("div");
  div.classList.add("line", ...classes);
  if (echo) div.classList.add("echo");
  terminal.appendChild(div);
  terminal.scrollTop = terminal.scrollHeight;
  return div;
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

function submit(line) {
  if (line.trim()) {
    history.push(line);
    historyIndex = history.length;
  }
  evaluate(line);
}

function evaluate(line) {
  const trimmed = line.trim();
  if (!trimmed) return;

  if (trimmed === "clear") {
    terminal.replaceChildren();
    return;
  }

  appendEcho(line);

  const statsMatch = trimmed.match(/^stats\s+(.+)$/i);
  try {
    if (statsMatch) {
      appendText(stats(statsMatch[1], STATS_SAMPLES));
    } else {
      appendRoll(rollJson(trimmed));
    }
  } catch (e) {
    appendText(e.message ?? String(e), "error");
  }
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
    submit(value);
    input.value = "";
  } else if (e.key === "ArrowUp") {
    if (historyIndex > 0) {
      historyIndex--;
      input.value = history[historyIndex];
      requestAnimationFrame(() =>
        input.setSelectionRange(input.value.length, input.value.length),
      );
    }
    e.preventDefault();
  } else if (e.key === "ArrowDown") {
    if (historyIndex < history.length - 1) {
      historyIndex++;
      input.value = history[historyIndex];
    } else {
      historyIndex = history.length;
      input.value = "";
    }
    e.preventDefault();
  }
});

// Mobile: Roll button
document.getElementById("roll-btn").addEventListener("click", () => {
  submit(input.value);
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
  input.disabled = false;
  document.getElementById("roll-btn").disabled = false;
  diceBar.querySelectorAll(".dice-btn").forEach((btn) => (btn.disabled = false));
  input.focus();
} catch (e) {
  appendText("Failed to load the dice engine. Please reload the page.", "error");
}

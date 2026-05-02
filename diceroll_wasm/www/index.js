import init, { rollJson, stats } from "./pkg/diceroll_wasm.js";

await init();

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
  div.appendChild(document.createTextNode(` = ${result.total}`));
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
      return span;
    }
  }
  return document.createTextNode(String(roll));
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

input.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    const value = input.value;
    if (value.trim()) {
      history.push(value);
      historyIndex = history.length;
    }
    evaluate(value);
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

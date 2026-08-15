// The roll transcript: an append-only log of echoed input and rendered results.

// The engine hands back each line already split into styled runs, so the dice
// notation is defined once, in Rust. A span's style doubles as its CSS class.
const SPAN_ARIA_LABEL = {
  "nat-1": (text) => `${text} (natural 1)`,
  "nat-max": (text) => `${text} (critical)`,
};

function spanNode(text, style) {
  if (style === "plain") return document.createTextNode(text);

  const span = document.createElement("span");
  span.className = style;
  span.textContent = text;
  const ariaLabel = SPAN_ARIA_LABEL[style];
  if (ariaLabel) span.setAttribute("aria-label", ariaLabel(text));
  return span;
}

export function createTerminal(root) {
  function appendLine(...classes) {
    const div = document.createElement("div");
    div.classList.add("line", ...classes);
    root.appendChild(div);
    return div;
  }

  return {
    /** Echoes a submitted expression behind a prompt. */
    appendEcho(line) {
      const div = appendLine("echo");
      const prompt = document.createElement("span");
      prompt.className = "prompt";
      prompt.textContent = ">>> ";
      div.appendChild(prompt);
      div.appendChild(document.createTextNode(line));
    },

    /** Appends plain text — statistics blocks and error messages. */
    appendText(text, ...classes) {
      appendLine(...classes).textContent = text;
    },

    /** Appends a roll from the engine's `{ text, style }` spans. */
    appendRoll(spans) {
      const div = appendLine();
      for (const { text, style } of spans) {
        div.appendChild(spanNode(text, style));
      }
    },

    clear() {
      root.replaceChildren();
    },

    scrollToBottom() {
      root.scrollTop = root.scrollHeight;
    },
  };
}

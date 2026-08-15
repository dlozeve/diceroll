// Touch devices get a keypad holding exactly the tokens the dice notation uses,
// instead of the device keyboard; desktop keeps the plain text field. Which one
// shows is driven by CSS (see #keypad in the stylesheet); the media query here
// only decides whether to suppress the device keyboard with inputmode="none".

const STATS_PREFIX = "stats ";

const insertKey = (text, label = text) => ({ label, insert: text });

export function createKeypad({ root, numbers, modifiers, utility, input, actions }) {
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
    actions.clear();
  }

  const backspaceKey = { label: "⌫", ariaLabel: "Backspace", action: backspace };
  const rollKey = {
    label: "Roll",
    ariaLabel: "Roll",
    className: "key-roll",
    action: actions.roll,
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

  numbers.replaceChildren(...NUMBER_KEYS.map(createKey));
  modifiers.replaceChildren(...MODIFIER_KEYS.map(createKey));

  let modifiersVisible = false;

  function togglePane() {
    modifiersVisible = !modifiersVisible;
    numbers.hidden = modifiersVisible;
    modifiers.hidden = !modifiersVisible;
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
    action: () => setNativeKeyboard(!nativeKeyboard),
  });
  paneKey.setAttribute("aria-pressed", "false");
  keyboardKey.setAttribute("aria-pressed", "false");

  utility.replaceChildren(
    paneKey,
    createKey({
      label: "↑",
      ariaLabel: "Previous expression",
      className: "key-util",
      action: actions.recallPrevious,
    }),
    createKey({
      label: "↓",
      ariaLabel: "Next expression",
      className: "key-util",
      action: actions.recallNext,
    }),
    keyboardKey,
  );

  // Tapping a key must not move focus away from the input, so the caret stays put
  root.addEventListener("mousedown", (e) => e.preventDefault());

  return {
    setEnabled(enabled) {
      root.querySelectorAll(".key").forEach((key) => (key.disabled = !enabled));
    },
  };
}

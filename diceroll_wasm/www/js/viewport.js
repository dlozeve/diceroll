// The on-screen keyboard shrinks the visual viewport without moving the layout
// viewport, so the input row would sit underneath it. Publishing the overlap as
// --keyboard-inset lets the stylesheet pad the body out of the way.

export function trackViewportInsets() {
  function update() {
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
    window.visualViewport.addEventListener("resize", update);
    window.visualViewport.addEventListener("scroll", update);
  }
  window.addEventListener("resize", update);
  update();
}

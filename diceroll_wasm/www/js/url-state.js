// A session lives entirely in the URL: the seed replays the same RNG stream,
// and the compressed history replays the same expressions against it. Sharing
// a link therefore shares an exact transcript.
//
// Everything here takes and returns plain values, so it is testable without a
// DOM — see tests/url-state.test.js.

const URL_SEED_KEY = "seed";
const URL_HISTORY_KEY = "h";

/** A fresh 64-bit seed, hex-encoded — the form the engine parses. */
export function generateSeed() {
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

/** Gzips a list of expressions into one URL-safe parameter. */
export async function encodeHistory(history) {
  const json = JSON.stringify(history);
  const stream = new Blob([json]).stream().pipeThrough(new CompressionStream("gzip"));
  const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
  return bytesToBase64Url(bytes);
}

/** Inverse of {@link encodeHistory}. Throws on anything but a list of strings. */
export async function decodeHistory(text) {
  const bytes = base64UrlToBytes(text);
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
  const json = await new Response(stream).text();
  const value = JSON.parse(json);
  if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string")) {
    throw new Error("invalid history");
  }
  return value;
}

/**
 * The session an href carries, or null when it carries none. A history without
 * a seed is unreplayable, so it counts as no session at all.
 */
export async function readSessionState(href) {
  const params = new URL(href).searchParams;
  const seed = params.get(URL_SEED_KEY);
  if (!seed) return null;

  const historyText = params.get(URL_HISTORY_KEY);
  if (!historyText) return { seed, history: [] };

  try {
    return { seed, history: await decodeHistory(historyText) };
  } catch {
    return null;
  }
}

/** `href` carrying `state`, or stripped of the session parameters for null. */
export async function sessionUrl(href, state) {
  const url = new URL(href);
  if (state) {
    url.searchParams.set(URL_SEED_KEY, state.seed);
    url.searchParams.set(URL_HISTORY_KEY, await encodeHistory(state.history));
  } else {
    url.searchParams.delete(URL_SEED_KEY);
    url.searchParams.delete(URL_HISTORY_KEY);
  }
  return url.toString();
}

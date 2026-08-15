import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  decodeHistory,
  encodeHistory,
  generateSeed,
  readSessionState,
  sessionUrl,
} from "../diceroll_wasm/www/js/url-state.js";

const BASE = "https://diceroll.run/";

describe("generateSeed", () => {
  it("produces a 16-digit hex seed", () => {
    for (let i = 0; i < 100; i += 1) {
      assert.match(generateSeed(), /^[0-9a-f]{16}$/);
    }
  });

  it("does not repeat", () => {
    const seeds = new Set(Array.from({ length: 100 }, generateSeed));
    assert.equal(seeds.size, 100);
  });
});

describe("history encoding", () => {
  it("round-trips a list of expressions", async () => {
    const history = ["d20", "4d6kh3", "stats 2d6", "(2d6+3)*2", "8d6c>3"];
    assert.deepEqual(await decodeHistory(await encodeHistory(history)), history);
  });

  it("round-trips an empty history", async () => {
    assert.deepEqual(await decodeHistory(await encodeHistory([])), []);
  });

  it("round-trips expressions needing escaping", async () => {
    const history = ["d20 + (2d6+3)*2", "8d6c>=3", "  spaced  ", "é½ 🎲"];
    assert.deepEqual(await decodeHistory(await encodeHistory(history)), history);
  });

  it("encodes to a URL-safe alphabet", async () => {
    // A long history exercises base64 padding and the +/ substitutions
    const history = Array.from({ length: 200 }, (_, i) => `${i}d${i + 2}kh1`);
    assert.match(await encodeHistory(history), /^[A-Za-z0-9_-]+$/);
  });

  it("compresses a repetitive history well below its JSON size", async () => {
    const history = Array.from({ length: 200 }, () => "4d6kh3");
    const encoded = await encodeHistory(history);
    assert.ok(
      encoded.length < JSON.stringify(history).length / 4,
      `expected compression, got ${encoded.length} chars`,
    );
  });

  it("rejects payloads that are not a list of strings", async () => {
    for (const value of [{ seed: "x" }, [1, 2, 3], "d20", null, ["ok", 5]]) {
      const stream = new Blob([JSON.stringify(value)])
        .stream()
        .pipeThrough(new CompressionStream("gzip"));
      const bytes = new Uint8Array(await new Response(stream).arrayBuffer());
      const encoded = btoa(String.fromCharCode(...bytes))
        .replaceAll("+", "-")
        .replaceAll("/", "_")
        .replaceAll("=", "");
      await assert.rejects(() => decodeHistory(encoded), `accepted ${JSON.stringify(value)}`);
    }
  });

  it("rejects data that is not gzip", async () => {
    await assert.rejects(() => decodeHistory("bm90LWd6aXA"));
  });
});

describe("readSessionState", () => {
  it("returns null for a bare URL", async () => {
    assert.equal(await readSessionState(BASE), null);
  });

  it("returns an empty history for a seed on its own", async () => {
    assert.deepEqual(await readSessionState(`${BASE}?seed=1234abcd`), {
      seed: "1234abcd",
      history: [],
    });
  });

  it("ignores a history with no seed, which cannot be replayed", async () => {
    const h = await encodeHistory(["d20"]);
    assert.equal(await readSessionState(`${BASE}?h=${h}`), null);
  });

  it("reads back what sessionUrl wrote", async () => {
    const state = { seed: "cafebabe12345678", history: ["d20", "2d8+5"] };
    assert.deepEqual(await readSessionState(await sessionUrl(BASE, state)), state);
  });

  it("returns null on a corrupt history rather than throwing", async () => {
    assert.equal(await readSessionState(`${BASE}?seed=abcd&h=not-valid-gzip`), null);
  });
});

describe("sessionUrl", () => {
  it("strips both parameters for a null state", async () => {
    const url = await sessionUrl(`${BASE}?seed=abcd&h=xyz`, null);
    assert.equal(url, BASE);
  });

  it("overwrites a previous session in place", async () => {
    const first = await sessionUrl(BASE, { seed: "1111", history: ["d4"] });
    const second = await sessionUrl(first, { seed: "2222", history: ["d6"] });
    const params = new URL(second).searchParams;
    assert.deepEqual(params.getAll("seed"), ["2222"]);
    assert.deepEqual(await readSessionState(second), { seed: "2222", history: ["d6"] });
  });

  it("preserves unrelated query parameters and the fragment", async () => {
    const url = await sessionUrl(`${BASE}?utm=x#frag`, { seed: "abcd", history: [] });
    const parsed = new URL(url);
    assert.equal(parsed.searchParams.get("utm"), "x");
    assert.equal(parsed.hash, "#frag");
  });
});

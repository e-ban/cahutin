import { expect, test } from "vitest";
import { onFontsLoaded } from "./fontsLoaded";

/** Imite `document.fonts` : `ready` et l'événement `loadingdone`. */
function fakeFonts() {
  let resolve!: () => void;
  const target = new EventTarget() as EventTarget & { ready: Promise<unknown> };
  target.ready = new Promise<void>((r) => (resolve = r));
  return { fonts: target, ready: () => resolve(), loaded: () => target.dispatchEvent(new Event("loadingdone")) };
}

test("onFontsLoaded calls back when the fonts are ready and after each later load", async () => {
  const f = fakeFonts();
  let calls = 0;
  onFontsLoaded(f.fonts, () => calls++);
  expect(calls).toBe(0);
  f.ready();
  await Promise.resolve();
  await Promise.resolve();
  expect(calls).toBe(1);
  f.loaded();
  expect(calls).toBe(2);
});

test("onFontsLoaded stops after cleanup, even before the fonts are ready", async () => {
  const f = fakeFonts();
  let calls = 0;
  const stop = onFontsLoaded(f.fonts, () => calls++);
  stop();
  f.ready();
  f.loaded();
  await Promise.resolve();
  await Promise.resolve();
  expect(calls).toBe(0);
});

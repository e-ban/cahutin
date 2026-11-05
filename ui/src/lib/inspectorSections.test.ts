import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { InspectorSections } from "./inspectorSections.svelte";

beforeEach(() => {
  const data = new Map<string, string>();
  vi.stubGlobal("localStorage", { getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => void data.set(k, v) });
});
afterEach(() => vi.unstubAllGlobals());

test("a section is open or closed by default until it is toggled, then remembered", () => {
  const s = new InspectorSections();
  expect(s.isOpen("frame", true)).toBe(true);
  expect(s.isOpen("crop", false)).toBe(false);
  s.set("frame", false);
  s.set("crop", true);
  const again = new InspectorSections();
  expect(again.isOpen("frame", true)).toBe(false);
  expect(again.isOpen("crop", false)).toBe(true);
});

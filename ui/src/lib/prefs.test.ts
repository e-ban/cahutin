import { afterEach, expect, test, vi } from "vitest";
import { readPref, writePref } from "./prefs";

afterEach(() => vi.unstubAllGlobals());

function memoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (k) => data.get(k) ?? null,
    key: (i) => [...data.keys()][i] ?? null,
    removeItem: (k) => void data.delete(k),
    setItem: (k, v) => void data.set(k, String(v)),
  };
}

test("readPref falls back when storage throws", () => {
  vi.stubGlobal("localStorage", {
    getItem: () => {
      throw new Error("bloqué");
    },
    setItem: () => {
      throw new Error("bloqué");
    },
  });
  expect(readPref("inspectorOpen", false)).toBe(false);
  expect(() => writePref("inspectorOpen", true)).not.toThrow();
});

test("writePref then readPref round-trips under the carnet. prefix", () => {
  const storage = memoryStorage();
  vi.stubGlobal("localStorage", storage);
  writePref("thumbSize", 140);
  expect(storage.getItem("carnet.thumbSize")).toBe("140");
  expect(readPref("thumbSize", 96)).toBe(140);
  storage.setItem("carnet.broken", "{pas du json");
  expect(readPref("broken", "défaut")).toBe("défaut");
});

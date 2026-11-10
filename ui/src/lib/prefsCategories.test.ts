import { describe, expect, it } from "vitest";
import { PREFS_CATEGORIES, searchPrefs } from "./prefsCategories";

const ids = (query: string) => searchPrefs(PREFS_CATEGORIES, query).map((c) => c.id);

describe("searchPrefs", () => {
  it("shows every category when nothing is typed", () => {
    expect(ids("  ")).toEqual(PREFS_CATEGORIES.map((c) => c.id));
  });

  it("finds a setting by its name, without accents or capitals", () => {
    expect(ids("contraste")).toEqual(["accessibility"]);
    expect(ids("theme")).toEqual(["display"]);
    expect(ids("OLLAMA")).toEqual(["assistant"]);
  });

  it("finds a category by its own name", () => {
    expect(ids("daltonisme")).toEqual(["colorblind"]);
  });

  it("finds nothing for an unknown word", () => {
    expect(ids("wifi")).toEqual([]);
  });

  it("gives every category a unique id", () => {
    expect(new Set(PREFS_CATEGORIES.map((c) => c.id)).size).toBe(PREFS_CATEGORIES.length);
  });
});

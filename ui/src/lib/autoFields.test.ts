import { expect, test, vi } from "vitest";
import { autoText, completePage } from "./autoFields";
import type { Class, Page } from "./types";

const run = (text: string) => ({ text, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null });
const page = (...texts: string[]): Page => ({
  id: "p",
  size: { width_mm: 210, height_mm: 297 },
  grid: null,
  elements: texts.map((t, i) => ({
    id: `t${i}`,
    frame: { x_mm: 0, y_mm: 0, w_mm: 10, h_mm: 10, rotation_deg: 0 },
    locked: false,
    zone: false,
    kind: { type: "Text" as const, content: { paragraphs: [{ align: null, runs: [run(t)] }] }, style: { font: "Andika", size_pt: 14, color: "#222222", align: "Left" as const }, prompt: null },
  })),
});
const words = (p: Page) => p.elements.map((e) => (e.kind.type === "Text" ? e.kind.content.paragraphs[0].runs.map((r) => r.text).join("") : ""));
/** Comme le moteur, pour deux champs : {annee} et {date} avec la date donnée, {classe} avec la classe. */
const fill = vi.fn(async (p: Page, date: string | null, cls: Class | null) => ({
  ...p,
  elements: p.elements.map((e) =>
    e.kind.type === "Text"
      ? { ...e, kind: { ...e.kind, content: { paragraphs: e.kind.content.paragraphs.map((q) => ({ ...q, runs: q.runs.map((r) => ({ ...r, text: r.text.replace("{annee}", "2026").replace("{date}", date ?? "{date}").replace("{classe}", cls?.name ?? "{classe}") })) })) } } }
      : e,
  ),
}));
const cls = { name: "Les Loups" } as Class;

test("a page from a template: one date asked for all its date fields, none without them", async () => {
  const ask = vi.fn(async () => "9 nov");
  const a = await completePage(page("{date}", "Le {date}", "{prenom}"), { templateMode: false, cls, ask, fill });
  expect(ask).toHaveBeenCalledTimes(1);
  expect(ask).toHaveBeenCalledWith(false);
  expect(words(a.page)).toEqual(["9 nov", "Le 9 nov", "{prenom}"]);
  ask.mockClear();
  const b = await completePage(page("{classe}"), { templateMode: false, cls: null, ask, fill });
  expect(ask).not.toHaveBeenCalled();
  expect(words(b.page)).toEqual(["{classe}"]);
  expect(b.missingClass).toBe(true);
  const c = await completePage(page("{classe}"), { templateMode: false, cls, ask, fill });
  expect([words(c.page)[0], c.missingClass]).toEqual(["Les Loups", false]);
});

test("in template mode nothing is filled", async () => {
  const ask = vi.fn(async () => "9 nov");
  fill.mockClear();
  const a = await completePage(page("{date}"), { templateMode: true, cls, ask, fill });
  expect([ask.mock.calls.length, fill.mock.calls.length, words(a.page)[0]]).toEqual([0, 0, "{date}"]);
});

test("a field placed in a document: its text, nothing if the date is cancelled, braces without a class", async () => {
  expect(await autoText("classe", { cls, ask: async () => null, fill })).toBe("Les Loups");
  expect(await autoText("annee", { cls, ask: async () => "2026-11-09", fill })).toBe("2026");
  expect(await autoText("date", { cls, ask: async () => "9 nov", fill })).toBe("9 nov");
  expect(await autoText("date", { cls, ask: async () => null, fill })).toBeNull();
  expect(await autoText("classe", { cls: null, ask: async () => null, fill })).toBe("{classe}");
});

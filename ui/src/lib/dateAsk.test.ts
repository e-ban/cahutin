import { expect, test } from "vitest";
import { DateAsk, todayIso } from "./dateAsk.svelte";

test("asking for the date opens on today; closing keeps today unless it can be cancelled", async () => {
  const d = new DateAsk();
  const p = d.ask(false);
  expect([d.open, d.value, d.cancellable]).toEqual([true, todayIso(), false]);
  d.close();
  expect(await p).toBe(todayIso());
  const q = d.ask(true);
  d.value = "2026-11-09";
  d.cancel();
  expect(await q).toBeNull();
  const r = d.ask(true);
  d.value = "2026-11-09";
  d.confirm();
  expect([await r, d.open]).toEqual(["2026-11-09", false]);
});

test("today in ISO, from the computer's clock", () => {
  expect(todayIso()).toMatch(/^\d{4}-\d{2}-\d{2}$/);
});

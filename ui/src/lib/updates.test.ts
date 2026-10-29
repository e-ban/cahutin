import { expect, test } from "vitest";
import { type UpdateState, bannerVersion, shouldAskConsent, shouldCheck } from "./updates.svelte";

const fresh: UpdateState = { checkedOn: null, forVersion: null, available: null, dismissed: null };

test("the question waits for the welcome screen and the tour to be over", () => {
  expect(shouldAskConsent(null, { tourDone: true, tourRunning: false, welcomeOpen: false })).toBe(true);
  expect(shouldAskConsent(null, { tourDone: false, tourRunning: false, welcomeOpen: false })).toBe(false);
  expect(shouldAskConsent(null, { tourDone: true, tourRunning: true, welcomeOpen: false })).toBe(false);
  expect(shouldAskConsent(null, { tourDone: true, tourRunning: false, welcomeOpen: true })).toBe(false);
});

test("the question is asked only once", () => {
  const calm = { tourDone: true, tourRunning: false, welcomeOpen: false };
  expect(shouldAskConsent(true, calm)).toBe(false);
  expect(shouldAskConsent(false, calm)).toBe(false);
});

test("nothing is checked without consent", () => {
  expect(shouldCheck(null, fresh, "0.5.0", "2026-10-29")).toBe(false);
  expect(shouldCheck(false, fresh, "0.5.0", "2026-10-29")).toBe(false);
  expect(shouldCheck(true, fresh, "0.5.0", "2026-10-29")).toBe(true);
});

test("at most one check a day, and again after installing another version", () => {
  const today = { ...fresh, checkedOn: "2026-10-29", forVersion: "0.5.0" };
  expect(shouldCheck(true, today, "0.5.0", "2026-10-29")).toBe(false);
  expect(shouldCheck(true, today, "0.5.0", "2026-10-30")).toBe(true);
  expect(shouldCheck(true, today, "0.6.0", "2026-10-29")).toBe(true);
});

test("the banner shows the version found, until it is closed", () => {
  const found = { ...fresh, checkedOn: "2026-10-29", forVersion: "0.5.0", available: "0.6.0" };
  expect(bannerVersion(true, found, "0.5.0")).toBe("0.6.0");
  expect(bannerVersion(true, { ...found, dismissed: "0.6.0" }, "0.5.0")).toBeNull();
  expect(bannerVersion(true, { ...found, dismissed: "0.6.0", available: "0.7.0" }, "0.5.0")).toBe("0.7.0");
  expect(bannerVersion(true, { ...found, available: null }, "0.5.0")).toBeNull();
});

test("no banner once that version is installed, nor after withdrawing consent", () => {
  const found = { ...fresh, checkedOn: "2026-10-29", forVersion: "0.5.0", available: "0.6.0" };
  expect(bannerVersion(true, found, "0.6.0")).toBeNull();
  expect(bannerVersion(false, found, "0.5.0")).toBeNull();
});

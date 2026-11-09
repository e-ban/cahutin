import { expect, test } from "vitest";
import { BACKUP_INTERVAL_MS, shouldBackup } from "./backup";

test("a backup is written only for a changed, unsaved document, not during a save", () => {
  const a = { title: "a" } as never;
  const b = { title: "b" } as never;
  expect(shouldBackup(true, false, a, null)).toBe(true);
  expect(shouldBackup(true, false, a, a)).toBe(false);
  expect(shouldBackup(true, false, b, a)).toBe(true);
  expect(shouldBackup(false, false, b, a)).toBe(false);
  expect(shouldBackup(true, true, b, a)).toBe(false);
  expect(BACKUP_INTERVAL_MS).toBe(120_000);
});

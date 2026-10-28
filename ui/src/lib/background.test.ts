import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { marks, type Mark } from "./background";
import type { Pattern } from "./types";

const reference: [Pattern, Mark[]][] = JSON.parse(readFileSync(fileURLToPath(new URL("../../../fixtures/background_marks.json", import.meta.url)), "utf8"));

test("marks match the engine reference file", () => {
  expect(reference.map(([p]) => p)).toEqual(["Seyes", "SmallSquares", "Lines", "Dots"]);
  for (const [pattern, expected] of reference) expect(marks({ width_mm: 210, height_mm: 297 }, pattern)).toEqual(expected);
});

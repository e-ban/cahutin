import { expect, test } from "vitest";
import { parseShapeTile, shapeTile } from "./dnd";

test("a shape tile survives the drag, and anything else is refused", () => {
  const ends = { start: "Arrow", end: "Arrow", size: "Medium" } as const;
  expect(parseShapeTile(shapeTile({ shape: "Arrow", radius: 0, ends }))).toEqual({ shape: "Arrow", radius: 0, ends });
  expect(parseShapeTile(shapeTile({ shape: "Rect", radius: 6 }))).toEqual({ shape: "Rect", radius: 6 });
  for (const bad of ["", "nope", "{}", '{"shape":"Banana","radius":0}', '{"shape":"Rect","radius":"6"}']) expect(parseShapeTile(bad)).toBeNull();
});

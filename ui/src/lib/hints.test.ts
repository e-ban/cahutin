import { expect, test } from "vitest";
import { HINT_MAX_SHOWS, firstHint, hintFor, nextHint } from "./hints.svelte";

test("nextHint shows a hint until it has been shown 3 times", () => {
  expect(HINT_MAX_SHOWS).toBe(3);
  expect(nextHint({}, "dblclick-crop")).toBe("dblclick-crop");
  expect(nextHint({ "dblclick-crop": 2 }, "dblclick-crop")).toBe("dblclick-crop");
  expect(nextHint({ "dblclick-crop": 3 }, "dblclick-crop")).toBeNull();
  expect(nextHint({ "dblclick-crop": 3 }, "rightclick")).toBe("rightclick");
});

test("disabled hints never show", () => {
  expect(hintFor(false, {}, "rightclick")).toBeNull();
  expect(hintFor(true, {}, "rightclick")).toBe("rightclick");
  expect(hintFor(true, { rightclick: 3 }, "rightclick")).toBeNull();
});

test("firstHint takes the next hint once the first has been shown 3 times", () => {
  expect(firstHint(true, {}, ["dblclick-crop", "blur-faces"])).toBe("dblclick-crop");
  expect(firstHint(true, { "dblclick-crop": 3 }, ["dblclick-crop", "blur-faces"])).toBe("blur-faces");
  expect(firstHint(true, { "dblclick-crop": 3, "blur-faces": 3 }, ["dblclick-crop", "blur-faces"])).toBeNull();
  expect(firstHint(false, {}, ["table-cell"])).toBeNull();
});

import { expect, test } from "vitest";
import { moveableCursorCss, resizeCursor, screenDir } from "./cursors";

test("screenDir turns a handle's direction with the element, by eighths of a turn", () => {
  expect(screenDir("n", 0)).toBe("n");
  expect(screenDir("n", 90)).toBe("e");
  expect(screenDir("nw", 45)).toBe("n");
  expect(screenDir("se", -90)).toBe("ne");
  expect(screenDir("e", 200)).toBe("w");
});

test("resizeCursor: system cursors, and on Linux a drawn diagonal arrow before the system one", () => {
  expect(resizeCursor("n", 0, false)).toBe("ns-resize");
  expect(resizeCursor("w", 0, true)).toBe("ew-resize");
  expect(resizeCursor("nw", 0, false)).toBe("nwse-resize");
  expect(resizeCursor("ne", 0, false)).toBe("nesw-resize");
  const linux = resizeCursor("se", 0, true);
  expect(linux).toMatch(/^url\("data:image\/svg\+xml,.+"\) 12 12, nwse-resize$/);
  // Tourné d'un huitième de tour, un coin devient un bord.
  expect(resizeCursor("nw", 45, true)).toBe("ns-resize");
});

test("moveableCursorCss sets every Moveable handle of a scope to its cursor", () => {
  const css = moveableCursorCss(".scope", 90, false);
  expect(css).toContain('.scope .moveable-direction[data-direction="n"] { cursor: ew-resize !important; }');
  expect(css).toContain('.scope .moveable-direction[data-direction="nw"] { cursor: nesw-resize !important; }');
});

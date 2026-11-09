import { expect, test } from "vitest";
import { THUMB_LONG_EDGE_PX, useThumb } from "./photoSource";

test("the thumbnail serves a photo whose displayed long edge fits in it", () => {
  // 100 mm à zoom 0,5 sur un écran 1× : 189 px, sous 256.
  expect(useThumb(100, 60, 0.5, 1)).toBe(true);
  // Le même sur un écran 2× : 378 px physiques, l'image entière.
  expect(useThumb(100, 60, 0.5, 2)).toBe(false);
  // Bande des pages (zoom 0,07) : toujours la vignette, même pour une pleine page A4.
  expect(useThumb(210, 297, 0.07, 2)).toBe(true);
  expect(THUMB_LONG_EDGE_PX).toBe(256);
});

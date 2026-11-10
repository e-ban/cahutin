import { describe, expect, it } from "vitest";
import { fitPopover } from "./popover";

const view = { width: 1000, height: 800 };

describe("fitPopover", () => {
  it("leaves a menu that fits below its button as it is", () => {
    expect(fitPopover({ top: 100, bottom: 130 }, { left: 50, width: 200, height: 300 }, view)).toEqual({ above: false, maxHeight: null, shiftX: 0 });
  });

  it("opens above when there is more room there", () => {
    const fit = fitPopover({ top: 600, bottom: 630 }, { left: 50, width: 200, height: 400 }, view);
    expect(fit.above).toBe(true);
    expect(fit.maxHeight).toBeNull();
  });

  it("scrolls when neither side is tall enough", () => {
    const fit = fitPopover({ top: 300, bottom: 330 }, { left: 50, width: 200, height: 900 }, view);
    expect(fit.above).toBe(false);
    expect(fit.maxHeight).toBe(800 - 8 - 336);
  });

  it("stays below and scrolls when below is the larger side", () => {
    const fit = fitPopover({ top: 200, bottom: 230 }, { left: 50, width: 200, height: 700 }, view);
    expect(fit).toEqual({ above: false, maxHeight: 556, shiftX: 0 });
  });

  it("moves left to stay inside the right edge", () => {
    expect(fitPopover({ top: 100, bottom: 130 }, { left: 900, width: 200, height: 100 }, view).shiftX).toBe(-108);
  });

  it("never goes past the left edge", () => {
    expect(fitPopover({ top: 100, bottom: 130 }, { left: -40, width: 200, height: 100 }, view).shiftX).toBe(48);
  });

  it("keeps a usable height even in a tiny window", () => {
    const fit = fitPopover({ top: 40, bottom: 70 }, { left: 0, width: 200, height: 500 }, { width: 1000, height: 150 });
    expect(fit.maxHeight).toBe(120);
  });
});

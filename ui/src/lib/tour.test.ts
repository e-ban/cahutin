import { expect, test } from "vitest";
import { placeBubble, shouldStartTour, tour, tourMayStart, TOUR_STEPS } from "./tour.svelte";

const bubble = { width: 300, height: 150 };
const viewport = { width: 1000, height: 700 };

test("eight steps, the first without target, one for the class", () => {
  expect(TOUR_STEPS).toHaveLength(8);
  expect(TOUR_STEPS[0].target).toBeNull();
  expect(TOUR_STEPS.map((s) => s.target)).toEqual([null, ".rail", "main.stage .page", "main.stage .page", ".right, button.tab", ".pagebar", '.rail [data-tour="class"]', ".top"]);
  expect(TOUR_STEPS.every((s) => s.title && s.text && !/[–—]/.test(s.title + s.text))).toBe(true);
});

test("missing target centres the bubble", () => {
  expect(placeBubble(null, bubble, viewport)).toEqual({ left: 350, top: 275 });
});

test("bubble goes right of the target when there is room", () => {
  expect(placeBubble({ left: 100, top: 100, width: 200, height: 100 }, bubble, viewport)).toEqual({ left: 312, top: 100 });
});

test("bubble goes below a target near the right edge", () => {
  expect(placeBubble({ left: 700, top: 100, width: 250, height: 80 }, bubble, viewport)).toEqual({ left: 688, top: 192 });
});

test("bubble stays inside the viewport", () => {
  const targets = [
    { left: 600, top: 600, width: 390, height: 90 },
    { left: 0, top: 0, width: 1000, height: 50 },
    { left: 0, top: 0, width: 1000, height: 700 },
    { left: 0, top: 60, width: 70, height: 600 },
  ];
  for (const t of targets) {
    const p = placeBubble(t, bubble, viewport);
    expect(p.left).toBeGreaterThanOrEqual(12);
    expect(p.top).toBeGreaterThanOrEqual(12);
    expect(p.left + bubble.width).toBeLessThanOrEqual(viewport.width - 12);
    expect(p.top + bubble.height).toBeLessThanOrEqual(viewport.height - 12);
  }
});

test("tour starts only once", () => {
  expect(shouldStartTour(false, true, false)).toBe(true);
  expect(shouldStartTour(true, true, false)).toBe(false);
  expect(shouldStartTour(false, false, false)).toBe(false);
  expect(shouldStartTour(false, true, true)).toBe(false);
});

test("end marks the tour done and next ends it at the last step", () => {
  tour.start();
  expect(tour.step).toBe(0);
  tour.prev();
  expect(tour.step).toBe(0);
  for (let i = 0; i < TOUR_STEPS.length - 1; i++) tour.next();
  expect(tour.step).toBe(TOUR_STEPS.length - 1);
  tour.next();
  expect(tour.step).toBeNull();
  expect(tour.done).toBe(true);
});

test("a pending tour waits for open dialogs and notices", () => {
  expect(tourMayStart(true, false, false)).toBe(true);
  expect(tourMayStart(true, true, false)).toBe(false);
  expect(tourMayStart(true, false, true)).toBe(false);
  expect(tourMayStart(false, false, false)).toBe(false);
});

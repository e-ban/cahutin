import { afterEach, expect, test, vi } from "vitest";
import { drawMoreNearEnd, scrollParent } from "./drawMore";

// Un DOM réduit au nécessaire : des éléments avec un parent, un style de défilement et une position.
type Fake = { parentElement: Fake | null; overflowY: string; top: number; bottom: number; listeners: Map<string, () => void> };
const el = (parentElement: Fake | null, overflowY = "visible", top = 0, bottom = 0): Fake => ({ parentElement, overflowY, top, bottom, listeners: new Map() });
const dom = (f: Fake) =>
  Object.assign(f, {
    getBoundingClientRect: () => ({ top: f.top, bottom: f.bottom }),
    addEventListener: (type: string, fn: () => void) => f.listeners.set(type, fn),
    removeEventListener: (type: string) => f.listeners.delete(type),
  }) as unknown as HTMLElement;

let frames: (() => void)[] = [];
vi.stubGlobal("getComputedStyle", (e: Fake) => ({ overflowY: e.overflowY }));
vi.stubGlobal("requestAnimationFrame", (fn: () => void) => frames.push(fn));
vi.stubGlobal("cancelAnimationFrame", () => {});
afterEach(() => (frames = []));

test("la grille défile dans le plus proche parent qui défile : la page, ou le corps du tiroir", () => {
  const page = el(null, "auto");
  const section = el(page);
  const mark = el(section);
  [page, section, mark].forEach(dom);
  expect(scrollParent(mark as unknown as Element)).toBe(page);
  expect(scrollParent(dom(el(null)) as unknown as Element)).toBeNull();
});

test("davantage de vignettes quand le repère approche du bas, ou après un saut tout en bas", () => {
  const scroller = el(null, "auto", 0, 800);
  const mark = el(scroller, "visible", 5000);
  [scroller, mark].forEach(dom);
  const more = vi.fn();
  const stop = drawMoreNearEnd(mark as unknown as HTMLElement, more);
  // Loin : rien, ni à l'image suivante ni au défilement.
  frames.forEach((f) => f());
  scroller.listeners.get("scroll")!();
  expect(more).not.toHaveBeenCalled();
  // À moins de 1500 px du bas de l'écran : davantage.
  mark.top = 2200;
  scroller.listeners.get("scroll")!();
  expect(more).toHaveBeenCalledTimes(1);
  // Déjà passé au-dessus de l'écran (un saut tout en bas) : davantage aussi.
  mark.top = -3000;
  scroller.listeners.get("scroll")!();
  expect(more).toHaveBeenCalledTimes(2);
  stop();
  expect(scroller.listeners.has("scroll")).toBe(false);
});

test("sans parent qui défile, rien n'est attendu", () => {
  const mark = dom(el(null));
  const stop = drawMoreNearEnd(mark, vi.fn());
  expect(frames).toEqual([]);
  stop();
});

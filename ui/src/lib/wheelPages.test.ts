import { expect, test } from "vitest";
import { WHEEL_PAGE_PAUSE_MS, wheelPages, wheelPagesStart } from "./wheelPages";

test("page entière à l'écran : un cran de molette tourne la page, dans les deux sens", () => {
  expect(wheelPages(wheelPagesStart(), 100, 0, false, false, 1000).dir).toBe(1);
  expect(wheelPages(wheelPagesStart(), -3, 1, false, false, 1000).dir).toBe(-1);
});

test("tant que la vue peut défiler, la molette la fait défiler", () => {
  const r = wheelPages(wheelPagesStart(), 100, 0, false, true, 1000);
  expect(r).toMatchObject({ scroll: true, dir: 0 });
  // Le haut de la page n'empêche pas de descendre.
  expect(wheelPages(wheelPagesStart(), 100, 0, true, false, 1000).dir).toBe(1);
});

test("arrivé au bord en défilant, l'élan ne tourne pas la page : il faut une pause", () => {
  const scrolled = wheelPages(wheelPagesStart(), 100, 0, false, true, 1000).state;
  const pushed = wheelPages(scrolled, 100, 0, false, false, 1100);
  expect(pushed.dir).toBe(0);
  // L'élan qui continue repousse l'attente.
  const still = wheelPages(pushed.state, 100, 0, false, false, 1100 + WHEEL_PAGE_PAUSE_MS - 10);
  expect(still.dir).toBe(0);
  expect(wheelPages(still.state, 100, 0, false, false, 1100 + 2 * WHEEL_PAGE_PAUSE_MS).dir).toBe(1);
});

test("après une page tournée, l'élan ne tourne pas la suivante ; de petits pas s'ajoutent", () => {
  const turned = wheelPages(wheelPagesStart(), 100, 0, false, false, 1000);
  expect(wheelPages(turned.state, 100, 0, false, false, 1100).dir).toBe(0);
  let s = wheelPagesStart();
  const small = wheelPages(s, 15, 0, false, false, 5000);
  s = small.state;
  expect(small.dir).toBe(0);
  s = wheelPages(s, 15, 0, false, false, 5010).state;
  expect(wheelPages(s, 15, 0, false, false, 5020).dir).toBe(1);
});

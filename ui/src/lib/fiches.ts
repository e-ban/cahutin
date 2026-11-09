// Fiches à générer : labyrinthes et dessins à compléter par symétrie sur quadrillage. Comme les lignes de
// graphisme, des formes ordinaires (tracés libres, traits, images) réunies en un groupe, sous ce que la page
// contient déjà.
// Copié pour l'assistant dans crates/cahutin-mcp/src/exercises.rs : changer les deux ensemble.
import { newId } from "./factory";
import type { Level } from "./graphisme";
import type { Element, Frame, Page, Stroke } from "./types";

const MARGIN_MM = 15;

function path(frame: Frame, d: string, stroke: Stroke, group: string): Element {
  return { id: newId(), frame, locked: false, zone: false, group, kind: { type: "Shape", shape: "Path", fill: null, stroke, corner_radius_mm: 0, path: d } };
}

function picture(code: string, frame: Frame, group: string): Element {
  return { id: newId(), frame, locked: false, zone: false, group, kind: { type: "Image", code } };
}

/** Haut de la place libre : sous le plus bas des éléments de la page. */
const freeTop = (page: Page) => Math.max(MARGIN_MM + 10, ...page.elements.map((e) => e.frame.y_mm + e.frame.h_mm + 8));

/** Générateur pseudo-aléatoire à graine (un même labyrinthe pour une même graine, pour les essais). */
function random(seed: number) {
  let s = seed >>> 0 || 1;
  return () => {
    s ^= s << 13;
    s ^= s >>> 17;
    s ^= s << 5;
    return (s >>> 0) / 4294967296;
  };
}

/** Cases d'un labyrinthe : de petite à grande section. */
export const MAZE_CELLS: Record<Level, number> = { PS: 4, MS: 6, GS: 9 };

/** Murs d'un labyrinthe parfait (un seul chemin entre deux cases) de `n` × `n` cases, par exploration en
 *  profondeur ; entrée à gauche de la première case, sortie à droite de la dernière. Tracé dans le carré
 *  0..1000, murs alignés fusionnés. */
export function mazePath(n: number, rnd: () => number): string {
  // right[y][x] : mur à droite de la case ; down[y][x] : mur sous la case.
  const right = Array.from({ length: n }, () => Array(n).fill(true));
  const down = Array.from({ length: n }, () => Array(n).fill(true));
  const seen = Array.from({ length: n }, () => Array(n).fill(false));
  const stack: [number, number][] = [[0, 0]];
  seen[0][0] = true;
  while (stack.length) {
    const [x, y] = stack[stack.length - 1];
    const next = ([[1, 0], [-1, 0], [0, 1], [0, -1]] as const)
      .map(([dx, dy]) => [x + dx, y + dy] as [number, number])
      .filter(([a, b]) => a >= 0 && b >= 0 && a < n && b < n && !seen[b][a]);
    if (!next.length) {
      stack.pop();
      continue;
    }
    const [a, b] = next[Math.floor(rnd() * next.length)];
    if (a > x) right[y][x] = false;
    else if (a < x) right[y][a] = false;
    else if (b > y) down[y][x] = false;
    else down[b][x] = false;
    seen[b][a] = true;
    stack.push([a, b]);
  }
  const u = (i: number) => Math.round((i * 1000) / n);
  const segs: string[] = [];
  const run = (cells: boolean[], at: (i: number) => [number, number, number, number]) => {
    for (let i = 0; i < cells.length; ) {
      if (!cells[i]) {
        i++;
        continue;
      }
      let j = i;
      while (j + 1 < cells.length && cells[j + 1]) j++;
      const [x1, y1] = at(i);
      const [, , x2, y2] = at(j);
      segs.push(`M ${x1} ${y1} L ${x2} ${y2}`);
      i = j + 1;
    }
  };
  // Bords : haut et bas pleins, gauche ouvert à l'entrée, droite ouverte à la sortie.
  run(Array(n).fill(true), (i) => [u(i), 0, u(i + 1), 0]);
  run(Array(n).fill(true), (i) => [u(i), 1000, u(i + 1), 1000]);
  run(Array.from({ length: n }, (_, i) => i !== 0), (i) => [0, u(i), 0, u(i + 1)]);
  run(Array.from({ length: n }, (_, i) => i !== n - 1), (i) => [1000, u(i), 1000, u(i + 1)]);
  // Murs intérieurs : verticaux colonne par colonne, horizontaux ligne par ligne.
  for (let x = 0; x < n - 1; x++) run(right.map((r) => r[x]), (y) => [u(x + 1), u(y), u(x + 1), u(y + 1)]);
  for (let y = 0; y < n - 1; y++) run(down[y], (x) => [u(x), u(y + 1), u(x + 1), u(y + 1)]);
  return segs.join(" ");
}

/** Un labyrinthe sous ce que la page contient : ses murs, un poussin à l'entrée, une poule à la sortie. */
export function maze(page: Page, level: Level, seed = Date.now()): Element[] {
  const n = MAZE_CELLS[level];
  const pic = 22;
  const top = freeTop(page);
  const size = Math.min(page.size.width_mm - 2 * MARGIN_MM - 2 * pic, page.size.height_mm - MARGIN_MM - top, level === "PS" ? 120 : 150);
  if (size < 60) return [];
  const x = (page.size.width_mm - size) / 2;
  const cell = size / n;
  const group = newId();
  return [
    path({ x_mm: x, y_mm: top, w_mm: size, h_mm: size, rotation_deg: 0 }, mazePath(n, random(seed)), { width_mm: level === "GS" ? 1 : 1.5, color: "#333333", style: "Solid" }, group),
    picture("1F424", { x_mm: x - pic - 1, y_mm: top + cell / 2 - pic / 2, w_mm: pic, h_mm: pic, rotation_deg: 0 }, group),
    picture("1F414", { x_mm: x + size + 1, y_mm: top + size - cell / 2 - pic / 2, w_mm: pic, h_mm: pic, rotation_deg: 0 }, group),
  ];
}

/** Moitiés gauches de dessins à compléter par symétrie : points du quadrillage, l'axe en x = 0, x vers la
 *  gauche (négatif), y vers le bas, sur 8 carreaux de haut. Chaque liste est un trait continu. */
export const SYMMETRY_FIGURES: Record<string, [number, number][][]> = {
  sapin: [[[0, 0], [-2, 2], [-1, 2], [-3, 4], [-2, 4], [-4, 6], [-1, 6], [-1, 8], [0, 8]]],
  maison: [
    [[0, 0], [-4, 3], [-3, 3], [-3, 8], [0, 8]],
    [[0, 6], [-1, 6], [-1, 8]],
    [[-2, 4], [-1, 4], [-1, 5], [-2, 5], [-2, 4]],
  ],
  fusee: [[[0, 0], [-1, 2], [-1, 6], [-2, 7], [-2, 8], [-1, 7], [0, 7]], [[0, 3], [-0.5, 3], [-0.5, 4], [0, 4]]],
  bateau: [[[0, 6], [-4, 6], [-3, 8], [0, 8]], [[0, 1], [-3, 5], [0, 5]]],
  coeur: [[[0, 2], [-1, 1], [-2, 1], [-3, 2], [-3, 3], [0, 7]]],
};

/** Carreaux du quadrillage : plus grands pour les petits. */
export const SYMMETRY_CELL_MM: Record<Level, number> = { PS: 20, MS: 16, GS: 12 };

/** Un quadrillage de 8 × 10 carreaux, l'axe rouge au milieu, et la moitié gauche d'un dessin à compléter. */
export function symmetry(page: Page, figure: string, level: Level): Element[] {
  const parts = SYMMETRY_FIGURES[figure];
  const c = SYMMETRY_CELL_MM[level];
  const [cols, rows] = [8, 10];
  const top = freeTop(page);
  if (!parts || top + rows * c > page.size.height_mm - MARGIN_MM) return [];
  const [w, h] = [cols * c, rows * c];
  const x = (page.size.width_mm - w) / 2;
  const group = newId();
  const lines: string[] = [];
  for (let i = 0; i <= cols; i++) lines.push(`M ${Math.round((i * 1000) / cols)} 0 L ${Math.round((i * 1000) / cols)} 1000`);
  for (let j = 0; j <= rows; j++) lines.push(`M 0 ${Math.round((j * 1000) / rows)} L 1000 ${Math.round((j * 1000) / rows)}`);
  // Le dessin : la moitié gauche du quadrillage, un carreau de marge en haut.
  const px = (gx: number) => Math.round(((cols / 2 + gx) * 1000) / cols);
  const py = (gy: number) => Math.round(((gy + 1) * 1000) / rows);
  const drawing = parts.map((pts) => pts.map(([gx, gy], i) => `${i ? "L" : "M"} ${px(gx)} ${py(gy)}`).join(" ")).join(" ");
  const frame = { x_mm: x, y_mm: top, w_mm: w, h_mm: h, rotation_deg: 0 };
  return [
    path(frame, lines.join(" "), { width_mm: 0.3, color: "#a0a0a0", style: "Solid" }, group),
    path(frame, drawing, { width_mm: 1.2, color: "#1f4e9c", style: "Solid" }, group),
    {
      id: newId(),
      frame: { x_mm: x + w / 2 - 1, y_mm: top, w_mm: 2, h_mm: h, rotation_deg: 0 },
      locked: false,
      zone: false,
      group,
      kind: { type: "Shape", shape: "Path", fill: null, stroke: { width_mm: 0.8, color: "#d32f2f", style: "Dashed" }, corner_radius_mm: 0, path: "M 500 0 L 500 1000" },
    },
  ];
}

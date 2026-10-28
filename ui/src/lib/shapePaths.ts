// Tracés des formes dessinées : copie exacte de crates/carnet-core/src/shapes.rs (un test Rust le vérifie).
// Le même tracé est dessiné à l'écran et écrit dans l'ODG.
import type { ShapeKind } from "./types";

export const SHAPE_VIEWBOX = 1000;

export const SHAPE_PATHS: Partial<Record<ShapeKind, string>> = {
  Star: "M 500 0 L 618 382 L 1000 382 L 691 618 L 809 1000 L 500 764 L 191 1000 L 309 618 L 0 382 L 382 382 Z",
  Heart: "M 500 1000 C 150 760 0 560 0 310 C 0 120 150 0 300 0 C 400 0 470 50 500 130 C 530 50 600 0 700 0 C 850 0 1000 120 1000 310 C 1000 560 850 760 500 1000 Z",
  Bubble: "M 120 0 L 880 0 C 946 0 1000 54 1000 120 L 1000 660 C 1000 726 946 780 880 780 L 380 780 L 180 1000 L 240 780 L 120 780 C 54 780 0 726 0 660 L 0 120 C 0 54 54 0 120 0 Z",
  Cloud: "M 500 127 C 598 -89 876 -8 779 209 C 917 144 1065 351 927 416 C 1054 434 1002 669 876 651 C 1015 785 787 938 648 805 C 648 1065 352 1065 352 805 C 213 938 -15 785 124 651 C -2 669 -54 434 73 416 C -65 351 83 144 221 209 C 124 -8 402 -89 500 127 Z",
};

/** Trait ou flèche : un segment au milieu du cadre, sans remplissage. */
export const isStrokeOnly = (kind: ShapeKind) => kind === "Line" || kind === "Arrow";

/** Largeur de la pointe d'une flèche, comme à l'export : quatre fois le trait, 4 mm au moins. */
export const arrowHeadMm = (strokeMm: number) => Math.max(4, strokeMm * 4);

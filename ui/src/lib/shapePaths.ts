// Tracés des formes dessinées : copie exacte de crates/carnet-core/src/shapes.rs (un test Rust le vérifie).
// Le même tracé est dessiné à l'écran et écrit dans l'ODG.
import type { HeadSize, ShapeKind } from "./types";

export const SHAPE_VIEWBOX = 1000;

export const SHAPE_PATHS: Partial<Record<ShapeKind, string>> = {
  Star: "M 500 0 L 618 382 L 1000 382 L 691 618 L 809 1000 L 500 764 L 191 1000 L 309 618 L 0 382 L 382 382 Z",
  Heart: "M 500 1000 C 150 760 0 560 0 310 C 0 120 150 0 300 0 C 400 0 470 50 500 130 C 530 50 600 0 700 0 C 850 0 1000 120 1000 310 C 1000 560 850 760 500 1000 Z",
  Bubble: "M 129 0 L 871 0 C 942 0 1000 90 1000 200 L 1000 578 C 1000 688 942 778 871 778 L 386 778 C 343 889 243 967 129 1000 C 186 922 229 856 229 778 L 129 778 C 58 778 0 688 0 578 L 0 200 C 0 90 58 0 129 0 Z",
  Cloud: "M 357 101 C 393 37 445 0 500 0 C 555 0 607 37 643 101 C 692 66 747 66 796 100 C 844 134 881 200 897 280 C 952 315 991 394 999 488 C 1006 581 981 673 933 729 C 925 799 897 860 856 890 C 816 921 768 919 729 884 C 708 959 660 1005 609 1000 C 557 994 514 937 500 858 C 486 937 443 994 391 1000 C 340 1005 292 959 271 884 C 232 919 184 921 144 890 C 103 860 75 799 67 729 C 19 673 -6 581 1 488 C 9 394 48 315 103 280 C 119 200 156 134 204 100 C 253 66 308 66 357 101 Z",
};

/** Trait ou flèche : un segment au milieu du cadre, sans remplissage. */
export const isStrokeOnly = (kind: ShapeKind) => kind === "Line" || kind === "Arrow";

/** Facteur de chaque taille de terminaison : les mêmes que `shapes::HEAD_FACTORS` (un test Rust le vérifie). */
export const HEAD_FACTORS: Record<HeadSize, number> = {
  Small: 0.7,
  Medium: 1,
  Large: 1.5,
};

/** Diamètre d'un rond, côté d'un carré : cette part de la largeur d'une pointe (`shapes::DOT_FACTOR`). */
export const DOT_FACTOR = 0.6;

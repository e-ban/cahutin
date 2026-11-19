// Tracés des formes dessinées : copie exacte de crates/carnet-core/src/shapes.rs (un test Rust le vérifie).
// Le même tracé est dessiné à l'écran et écrit dans l'ODG.
import type { HeadSize, ShapeKind } from "./types";

export const SHAPE_VIEWBOX = 1000;

export const SHAPE_PATHS: Partial<Record<ShapeKind, string>> = {
  Star: "M 500 0 L 618 382 L 1000 382 L 691 618 L 809 1000 L 500 764 L 191 1000 L 309 618 L 0 382 L 382 382 Z",
  Heart: "M 500 1000 C 150 760 0 560 0 310 C 0 120 150 0 300 0 C 400 0 470 50 500 130 C 530 50 600 0 700 0 C 850 0 1000 120 1000 310 C 1000 560 850 760 500 1000 Z",
  Bubble: "M 129 0 L 871 0 C 942 0 1000 90 1000 200 L 1000 578 C 1000 688 942 778 871 778 L 386 778 C 343 889 243 967 129 1000 C 186 922 229 856 229 778 L 129 778 C 58 778 0 688 0 578 L 0 200 C 0 90 58 0 129 0 Z",
  Cloud: "M 349 122 C 387 47 448 2 512 0 C 576 -2 638 40 678 113 C 770 87 859 174 877 308 C 937 326 983 393 996 480 C 1009 566 987 656 938 709 C 927 766 899 813 861 836 C 824 859 781 855 746 825 C 735 910 690 977 632 995 C 574 1013 514 979 480 908 C 450 957 404 978 360 963 C 315 947 281 898 269 834 C 219 893 149 901 93 853 C 37 805 8 712 19 620 C 0 571 -5 512 6 458 C 17 404 43 359 78 334 C 84 245 120 166 175 123 C 229 81 294 80 349 122 Z",
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

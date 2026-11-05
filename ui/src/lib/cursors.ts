// Curseurs des poignées de redimensionnement (Moveable et rognage) : ceux du système, dans le sens de la poignée
// à l'écran (l'élément peut être tourné). Sous Linux, les thèmes n'ont pas tous les flèches diagonales
// (Breeze n'a pas les noms que GTK cherche) : une flèche double dessinée, à la taille d'un curseur, passe avant
// le curseur du système. ↕ et ↔ sont des curseurs de base de X11, présents partout.

export type HandleDir = "n" | "ne" | "e" | "se" | "s" | "sw" | "w" | "nw";
const DIRS: HandleDir[] = ["n", "ne", "e", "se", "s", "sw", "w", "nw"];

/** Direction à l'écran d'une poignée, l'élément tourné de `rotationDeg` (au huitième de tour près). */
export function screenDir(dir: HandleDir, rotationDeg: number): HandleDir {
  const steps = Math.round(rotationDeg / 45);
  return DIRS[(((DIRS.indexOf(dir) + steps) % 8) + 8) % 8];
}

/** Flèche double noire bordée de blanc, 24 px, tournée de `deg` (0 : verticale). */
function arrow(deg: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path transform="rotate(${deg} 12 12)" d="M12 2 L17.5 8 L14 8 L14 16 L17.5 16 L12 22 L6.5 16 L10 16 L10 8 L6.5 8 Z" fill="#000" stroke="#fff" stroke-width="1.5" stroke-linejoin="round"/></svg>`;
  return `url("data:image/svg+xml,${encodeURIComponent(svg)}") 12 12`;
}

/** Le curseur CSS d'une poignée. `linux` : la flèche dessinée passe avant les diagonales du système. */
export function resizeCursor(dir: HandleDir, rotationDeg: number, linux: boolean): string {
  const d = screenDir(dir, rotationDeg);
  if (d === "n" || d === "s") return "ns-resize";
  if (d === "e" || d === "w") return "ew-resize";
  const nwse = d === "nw" || d === "se";
  const system = nwse ? "nwse-resize" : "nesw-resize";
  return linux ? `${arrow(nwse ? -45 : 45)}, ${system}` : system;
}

/** Sous Linux (WebKitGTK), pas sous Windows ni macOS. */
export const IS_LINUX = typeof navigator !== "undefined" && /Linux/.test(navigator.userAgent) && !/Android/.test(navigator.userAgent);

/** Règles CSS qui donnent à chaque poignée (et bord) de Moveable dans `scope` son curseur, à la place des
 *  grosses flèches que Moveable dessine. */
export function moveableCursorCss(scope: string, rotationDeg: number, linux: boolean): string {
  return DIRS.map((d) => `${scope} .moveable-direction[data-direction="${d}"] { cursor: ${resizeCursor(d, rotationDeg, linux)} !important; }`).join("\n");
}

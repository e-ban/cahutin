// Banque d'images OpenMoji : index (codes, catégories, mots-clés français et anglais), recherche,
// création et remplacement d'éléments image.
import { protocolUrl } from "./api";
import { centeredFrame, newId } from "./factory";
import { normalize } from "./fontSearch";
import { expandSelection } from "./groups";
import type { Element, Page } from "./types";

export interface PictoEntry {
  code: string;
  group: string;
  fr: string[];
  en: string[];
}

/** Catégories, dans l'ordre du tiroir (clés de l'index). */
export const PICTO_GROUPS = ["visages", "personnes", "animaux", "nourriture", "lieux", "activites", "objets", "symboles", "openmoji"] as const;

export const PICTO_SIZE_MM = 30;

/**
 * Images dont un mot-clé contient la recherche (sans majuscules ni accents) : d'abord celles dont un
 * mot-clé français commence par la recherche, puis les autres mots-clés français, puis l'anglais.
 */
export function searchPictos(index: PictoEntry[], query: string): PictoEntry[] {
  const q = normalize(query);
  if (!q) return index;
  const rank = (p: PictoEntry): number => {
    const fr = p.fr.map(normalize);
    if (fr.some((w) => w.startsWith(q) || w.split(/[\s’'-]+/).some((part) => part.startsWith(q)))) return 0;
    if (fr.some((w) => w.includes(q))) return 1;
    if (p.en.map(normalize).some((w) => w.includes(q))) return 2;
    return 3;
  };
  return index
    .map((p, i) => ({ p, r: rank(p), i }))
    .filter((x) => x.r < 3)
    .sort((a, b) => a.r - b.r || a.i - b.i)
    .map((x) => x.p);
}

export const pictoUrl = (code: string) => protocolUrl(`/picto/${code}.svg`);

/** Nouvelle image carrée, centrée sur la page ou sur le point donné. */
export function pictoElement(page: Page, code: string, at?: { x_mm: number; y_mm: number }, size_mm = PICTO_SIZE_MM): Element {
  const frame = at
    ? { x_mm: Math.round((at.x_mm - size_mm / 2) * 10) / 10, y_mm: Math.round((at.y_mm - size_mm / 2) * 10) / 10, w_mm: size_mm, h_mm: size_mm, rotation_deg: 0 }
    : centeredFrame(page, size_mm, size_mm);
  return { id: newId(), frame, locked: false, zone: false, kind: { type: "Image", code } };
}

/** Remplace toutes les images de la sélection et de ses groupes (une frise change de motif). */
export function replacePictos(page: Page, ids: string[], code: string, expand = true): Page {
  const all = new Set(expand ? expandSelection(page, ids) : ids);
  return { ...page, elements: page.elements.map((e) => (all.has(e.id) && e.kind.type === "Image" && !e.locked ? { ...e, kind: { type: "Image", code } } : e)) };
}

export const pageHasPictos = (page: Page) => page.elements.some((e) => e.kind.type === "Image");

class Pictos {
  index = $state.raw<PictoEntry[]>([]);
  #loading: Promise<void> | null = null;

  /** Index servi avec l'interface (dossier assets) ; chargé une fois. */
  load(): Promise<void> {
    this.#loading ??= fetch("/openmoji/index.json")
      .then((r) => r.json())
      .then((data: PictoEntry[]) => void (this.index = data))
      .catch(() => void (this.#loading = null));
    return this.#loading;
  }
}

export const pictos = new Pictos();

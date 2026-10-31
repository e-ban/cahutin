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

/** Catégories de la classe, choisies dans OpenMoji pour les affichages et les rituels, dans l'ordre voulu. */
export const CLASS_PICTO_GROUPS: Record<string, string[]> = {
  rituels: ["1F4C5", "1F4C6", "23F0", "23F2", "2600", "2601", "1F327", "2744", "1F308", "1F44B", "1F3EB", "1F9D1-200D-1F91D-200D-1F9D1", "1F64B", "1F92B", "1F4DA", "1F3B5", "1F37D", "1F6CC", "1F6CF", "1F3C3", "1F6DD", "1F382", "1F9E5", "1F6AA"],
  materiel: ["270F", "1F58D", "1F58A", "1F58C", "1F3A8", "2702", "1F4CF", "1F4D0", "1F4CE", "1F4D3", "1F4D6", "1F9E9", "1F3B2", "1F9EE", "1F522", "1F524", "1F392", "1F9F8", "26BD", "1F3C0", "1FA80", "1FA81", "1F3B8", "1F941", "1F3B9"],
  emotions: ["1F600", "1F60A", "1F642", "1F970", "1F60C", "1F622", "1F62D", "1F97A", "1F620", "1F621", "1F628", "1F632", "1F62E", "1F971", "1F62A", "1F614", "1F914", "2764", "1F44D", "1F44F", "1F91D"],
  soins: ["1F9FC", "1F6B0", "1FAA5", "1F6BD", "1F9FB", "1F927", "1FA79", "1F964", "1F95B", "1F34E", "1F442", "1F441", "1F443", "1F444", "270B", "1F9E6", "1F45F", "1F9E4"],
};

/** Les images d'une catégorie : celles de la classe dans leur ordre, les autres d'après l'index. */
export function pictosOf(index: PictoEntry[], group: string): PictoEntry[] {
  const chosen = CLASS_PICTO_GROUPS[group];
  if (!chosen) return index.filter((p) => p.group === group);
  const byCode = new Map(index.map((p) => [p.code, p]));
  return chosen.flatMap((c) => byCode.get(c) ?? []);
}

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

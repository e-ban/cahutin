// Flou des visages, côté interface : remplacer la photo d'origine par sa version floutée dans tout le
// document et dans la photothèque, et les petites règles de la fenêtre de flou.
import type { LibraryPhoto } from "./photoLibrary";
import type { AssetMeta, Document, FaceBox } from "./types";

/** Plus petite zone tracée à la main, en fraction de l'image. */
const MIN_ZONE = 0.01;

/** La photo floutée prend la place de l'originale partout (photos et images de fond de cadre) ; l'originale
 *  quitte les images du document : elle ne sera pas enregistrée. */
export function replaceAsset(doc: Document, oldId: string, meta: AssetMeta): Document {
  const assets = { ...doc.assets, [meta.id]: meta };
  delete assets[oldId];
  return {
    ...doc,
    assets,
    pages: doc.pages.map((p) => ({
      ...p,
      elements: p.elements.map((e) => {
        const k = e.kind;
        if (k.type === "Photo" && k.asset === oldId) return { ...e, kind: { ...k, asset: meta.id } };
        if (k.type === "Text" && k.frame?.image === oldId) return { ...e, kind: { ...k, frame: { ...k.frame, image: meta.id } } };
        return e;
      }),
    })),
  };
}

/** Dans la photothèque aussi ; sans chemin de fichier, l'originale ne revient pas par les photos en attente. */
export function replaceInLibrary(photos: LibraryPhoto[], oldId: string, meta: AssetMeta): LibraryPhoto[] {
  return photos.map((p) => (p.meta.id === oldId ? { ...p, meta, path: "" } : p));
}

/** Zone tracée à la main, de (x0, y0) à (x1, y1) en fraction de l'image ; trop petite : aucune. */
export function zoneFromDrag(x0: number, y0: number, x1: number, y1: number): FaceBox | null {
  const c = (v: number) => Math.min(1, Math.max(0, v));
  const [ax, bx] = [c(Math.min(x0, x1)), c(Math.max(x0, x1))];
  const [ay, by] = [c(Math.min(y0, y1)), c(Math.max(y0, y1))];
  if (bx - ax < MIN_ZONE || by - ay < MIN_ZONE) return null;
  const r = (v: number) => Math.round(v * 1e6) / 1e6;
  return { x: r(ax), y: r(ay), w: r(bx - ax), h: r(by - ay) };
}

export function toggleZone(chosen: Set<number>, i: number): Set<number> {
  const next = new Set(chosen);
  if (!next.delete(i)) next.add(i);
  return next;
}

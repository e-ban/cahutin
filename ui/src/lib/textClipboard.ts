// Copier-coller de texte entre zones de Cahutin, avec sa mise en forme. Le presse-papiers du système ne
// garde que le texte ; le passage copié dans Cahutin est retenu ici, et revient au collage si le
// presse-papiers contient toujours ce même texte. Ce qui vient d'ailleurs reste collé en texte brut.
import type { JSONContent } from "@tiptap/core";
import { sanitizePlainText } from "./richtext";
import type { TextStyle } from "./types";

/** Un passage copié (`Slice.toJSON()` de ProseMirror). */
export interface CopiedSlice {
  content: JSONContent[];
  openStart: number;
  openEnd: number;
}

let last: { text: string; slice: CopiedSlice } | null = null;

const normalize = (text: string) => sanitizePlainText(text);

export function rememberCopy(text: string, slice: CopiedSlice) {
  last = { text: normalize(text), slice };
}

/** Le passage copié dans Cahutin, si le presse-papiers contient encore son texte. */
export function copiedSlice(text: string): CopiedSlice | null {
  return last && last.text === normalize(text) ? last.slice : null;
}

/**
 * Le passage avec la police, la taille et la couleur de sa zone écrites sur chaque morceau de texte : collé
 * dans une autre zone, il les garde. Ce que le morceau avait déjà en propre l'emporte.
 */
export function withZoneStyle(slice: CopiedSlice, style: TextStyle): CopiedSlice {
  const zone = { color: style.color, fontFamily: style.font, fontSize: `${style.size_pt}pt` };
  const fill = (node: JSONContent): JSONContent => {
    if (node.type === "text") {
      const marks = node.marks ?? [];
      const own = marks.find((m) => m.type === "textStyle")?.attrs ?? {};
      const attrs = Object.fromEntries(Object.entries(zone).map(([k, v]) => [k, own[k] ?? v]));
      return { ...node, marks: [...marks.filter((m) => m.type !== "textStyle"), { type: "textStyle", attrs }] };
    }
    return node.content ? { ...node, content: node.content.map(fill) } : node;
  };
  return { ...slice, content: slice.content.map(fill) };
}

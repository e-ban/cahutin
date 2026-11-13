// Ce qui vient d'un autre logiciel : une image ou un texte collé (Ctrl+V, « Coller »), des fichiers image lâchés
// sur la page. Les images passent par la filière de l'import des photos (sans métadonnées, réduites, copiées dans
// le dossier temporaire). Rien n'est jamais chargé depuis internet : une image dont on n'a que l'adresse est refusée.
import { importPhotoBytes } from "./api";
import { editor } from "./docStore.svelte";
import { addPhotoAt, hasClipboard, paste, usePhoto } from "./editActions";
import { centeredFrame, defaultTextStyle, textElement } from "./factory";
import { fr } from "./i18n/fr";
import { sanitizePlainText, TEXT_PADDING_MM } from "./richtext";
import { COPY_MIME, lastMark, type CopyMark } from "./systemClipboard";
import type { AssetMeta, Element, Page } from "./types";

/** Le contenu du presse-papiers du système, tel que la vue web le donne. */
export interface Incoming {
  token: string;
  text: string;
  html: string;
  image: Blob | null;
}

export type PasteChoice = "internal" | "image" | "text" | "remoteImage" | "nothing";

const clean = (text: string) => sanitizePlainText(text).trim();

/** Le texte d'un fragment HTML, lu sur la chaîne : il n'est jamais mis dans la page, aucune image n'est chargée. */
export function htmlText(html: string): string {
  const entities: Record<string, string> = { nbsp: " ", amp: "&", lt: "<", gt: ">", quot: '"', apos: "'" };
  return html
    .replace(/<(script|style|head)\b[\s\S]*?<\/\1>/gi, "")
    .replace(/<br\b[^>]*>|<\/(p|div|li|tr|h[1-6])>/gi, "\n")
    .replace(/<[^>]*>/g, "")
    .replace(/&(#x?[0-9a-f]+|[a-z]+);/gi, (m, e: string) => {
      if (e[0] !== "#") return entities[e.toLowerCase()] ?? m;
      const code = e[1].toLowerCase() === "x" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
      return Number.isFinite(code) ? String.fromCodePoint(code) : m;
    })
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l)
    .join("\n");
}

/**
 * Qui colle-t-on ? La copie de Cahutin tant que le presse-papiers du système contient encore ce qu'elle y a mis
 * (son jeton, ou à défaut son texte, sans image) ; sinon ce qui a été copié ailleurs depuis : une image, puis un
 * texte. Si Cahutin n'a pas pu marquer sa copie, elle l'emporte, comme avant.
 */
export function choosePaste(c: { token: string; text: string; html: string; image: boolean }, mark: CopyMark | null, hasInternal: boolean): PasteChoice {
  if (hasInternal && (!mark || (c.token && c.token === mark.token) || (!c.image && clean(c.text) === clean(mark.text)))) return "internal";
  const text = clean(c.text) || htmlText(c.html);
  const hasImg = /<img\b/i.test(c.html);
  // Un texte copié dans un traitement de texte ou un tableur est souvent offert aussi en image : c'est le texte.
  if (c.image && (!text || hasImg || !c.html)) return "image";
  if (text) return "text";
  return hasImg ? "remoteImage" : "nothing";
}

/** Fichier accepté sur la page : photo ou image, comme l'import des photos (un HEIC y reçoit son message). */
export function isImageFile(f: { name: string; type: string }): boolean {
  return /^image\/(png|jpeg|webp|svg\+xml|heic|heif)$/.test(f.type) || /\.(png|jpe?g|webp|svg|heic|heif)$/i.test(f.name);
}

/** Nouveau bloc de texte au centre de la page, assez grand pour le texte (estimé), sans en sortir. */
export function pastedTextElement(page: Page, text: string): Element {
  const t = clean(text);
  const style = defaultTextStyle();
  const lineMm = style.size_pt * 0.3528 * 1.3;
  const w = Math.min(120, page.size.width_mm - 20);
  const perLine = Math.max(1, Math.floor(w / (style.size_pt * 0.3528 * 0.5)));
  const lines = t.split("\n").reduce((n, l) => n + Math.max(1, Math.ceil(l.length / perLine)), 0);
  const h = Math.min(page.size.height_mm - 20, Math.round(lines * lineMm + 2 * TEXT_PADDING_MM + 2));
  const el = textElement(centeredFrame(page, w, h));
  if (el.kind.type === "Text") el.kind.content = { paragraphs: [{ align: null, runs: [{ text: t, bold: false, italic: false, underline: false, color: null, font: null, size_pt: null }] }] };
  return el;
}

/** L'image passe par l'import des photos et rejoint le tiroir Photos ; une erreur dit pourquoi. */
async function importImage(blob: Blob): Promise<AssetMeta> {
  const meta = await importPhotoBytes(new Uint8Array(await blob.arrayBuffer()));
  editor.addPhotos([{ meta, path: "", folder: "" }]);
  return meta;
}

/** Colle le contenu du presse-papiers : chaque pose est une étape d'annulation. */
export async function pasteIncoming(c: Incoming) {
  switch (choosePaste({ ...c, image: !!c.image }, lastMark(), hasClipboard())) {
    case "internal":
      paste();
      return;
    case "image":
      try {
        usePhoto(await importImage(c.image!), { warn: false });
      } catch (e) {
        editor.notice = fr.imageNotPlaced(String(e));
      }
      return;
    case "text": {
      const el = pastedTextElement(editor.page, clean(c.text) || htmlText(c.html));
      editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
      editor.selection = [el.id];
      return;
    }
    case "remoteImage":
      editor.notice = fr.pasteRemoteImage;
      return;
    case "nothing":
      editor.notice = fr.pasteNothing;
  }
}

// Ctrl+V hors d'un texte : l'événement « paste » de la vue web donne le presse-papiers sans demander d'accès. S'il
// ne vient pas (vue web qui ne l'envoie pas hors d'une zone de saisie), lecture comme pour « Coller » du menu.
let armed = false;

export function armPaste() {
  armed = true;
  setTimeout(() => {
    if (!armed) return;
    armed = false;
    void pasteFromMenu();
  }, 0);
}

export function handlePasteEvent(ev: ClipboardEvent) {
  if (!armed) return;
  armed = false;
  ev.preventDefault();
  const cd = ev.clipboardData;
  const image = cd ? (Array.from(cd.items).find((i) => i.kind === "file" && i.type.startsWith("image/"))?.getAsFile() ?? null) : null;
  void pasteIncoming({ token: cd?.getData(COPY_MIME) ?? "", text: cd?.getData("text/plain") ?? "", html: cd?.getData("text/html") ?? "", image });
}

async function readSystemClipboard(): Promise<Incoming> {
  const c: Incoming = { token: "", text: "", html: "", image: null };
  for (const item of await navigator.clipboard.read()) {
    for (const type of item.types) {
      if (type.startsWith("image/") && !c.image) c.image = await item.getType(type);
      else if (type === "text/plain" && !c.text) c.text = await (await item.getType(type)).text();
      else if (type === "text/html" && !c.html) c.html = await (await item.getType(type)).text();
    }
  }
  return c;
}

/** « Coller » du clic droit : le presse-papiers du système s'il est lisible ; sinon la copie de Cahutin. */
export async function pasteFromMenu() {
  // Copie de Cahutin non marquée : elle l'emporte de toute façon, inutile de lire le presse-papiers.
  if (hasClipboard() && !lastMark()) {
    paste();
    return;
  }
  let c: Incoming | null = null;
  try {
    c = await readSystemClipboard();
  } catch {
    // Lecture refusée par la vue web.
  }
  if (c) await pasteIncoming(c);
  else if (hasClipboard()) paste();
  else editor.notice = fr.pasteUseKeyboard;
}

/** Fichiers lâchés sur la page depuis l'explorateur : chaque image posée là où elle tombe (un peu décalées). */
export async function dropFiles(files: File[], xMm: number, yMm: number) {
  if (files.length === 0) {
    editor.notice = fr.dropFromWeb;
    return;
  }
  const images = files.filter(isImageFile);
  const notes: string[] = [];
  if (images.length < files.length) notes.push(fr.dropNotImage(files.length - images.length));
  let placed = 0;
  for (const f of images) {
    try {
      addPhotoAt(await importImage(f), xMm + placed * 8, yMm + placed * 8);
      placed++;
    } catch (e) {
      notes.push(fr.imageNotPlaced(String(e)));
    }
  }
  if (notes.length) editor.notice = [...new Set(notes)].join("\n");
}

/** Un fichier lâché à côté de la page : la vue web l'ouvrirait à la place de Cahutin. Rien n'est ouvert. */
export function guardWindowDrop(ev: DragEvent) {
  if (!ev.dataTransfer?.types.includes("Files")) return;
  ev.preventDefault();
  if (ev.type === "drop") editor.notice = fr.dropOnPage;
}

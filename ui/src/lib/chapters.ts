// Chapitres d'un carnet : une page ouvre un chapitre (nom, couleur), qui va jusqu'à la page qui en ouvre un autre.
// Les pages d'avant le premier chapitre n'en ont pas. Un chapitre reste à sa place quand on déplace ou supprime
// sa première page : il passe à la page suivante.
import type { Chapter, Document, Page } from "./types";

/** Couleurs d'onglet proposées tour à tour aux nouveaux chapitres. */
export const CHAPTER_COLORS = ["#2e86de", "#e67e22", "#27ae60", "#8e44ad", "#c0392b", "#f1c40f"];

/** Un chapitre et ses pages : de `start` (la page qui l'ouvre) à `end` (exclue). */
export type ChapterSpan = Chapter & { start: number; end: number };

export function chapterSpans(pages: Page[]): ChapterSpan[] {
  const starts = pages.flatMap((p, i) => (p.chapter ? [i] : []));
  return starts.map((start, k) => ({ ...pages[start].chapter!, start, end: starts[k + 1] ?? pages.length }));
}

/** La place juste après le chapitre de la page `i` (avant le premier chapitre : le début de celui-ci). */
export function chapterEnd(pages: Page[], i: number): number {
  return chapterOf(pages, i)?.end ?? chapterSpans(pages)[0]?.start ?? pages.length;
}

/** Le chapitre de la page `i`, ou `null` avant le premier chapitre. */
export function chapterOf(pages: Page[], i: number): ChapterSpan | null {
  return chapterSpans(pages).find((c) => c.start <= i && i < c.end) ?? null;
}

function withChapter(doc: Document, i: number, chapter: Chapter | null): Document {
  return { ...doc, pages: doc.pages.map((p, k) => (k === i ? { ...p, chapter } : p)) };
}

/** La couleur d'un nouveau chapitre ouvert à la page `i` : la moins employée des couleurs proposées, hors celles des
 *  chapitres juste avant et juste après, pour que deux onglets voisins ne se confondent pas. */
export function nextChapterColor(pages: Page[], i: number): string {
  const spans = chapterSpans(pages).filter((c) => c.start !== i);
  const before = [...spans].reverse().find((c) => c.start < i);
  const after = spans.find((c) => c.start > i);
  const uses = (color: string) => spans.filter((c) => c.color === color).length;
  const free = CHAPTER_COLORS.filter((c) => c !== before?.color && c !== after?.color);
  return [...free].sort((a, b) => uses(a) - uses(b))[0] ?? CHAPTER_COLORS[0];
}

/** La page `i` ouvre un chapitre nommé `title`, d'une couleur qui le distingue de ses voisins. */
export function startChapter(doc: Document, i: number, title: string): Document {
  return withChapter(doc, i, { title, color: nextChapterColor(doc.pages, i) });
}

export function updateChapter(doc: Document, i: number, patch: Partial<Chapter>): Document {
  const chapter = doc.pages[i]?.chapter;
  return chapter ? withChapter(doc, i, { ...chapter, ...patch }) : doc;
}

/** Le chapitre de la page `i` n'existe plus : ses pages restent, dans le chapitre d'avant. */
export function removeChapter(doc: Document, i: number): Document {
  return doc.pages[i]?.chapter ? withChapter(doc, i, null) : doc;
}

/** Avant que la page `i` quitte sa place : si elle ouvre un chapitre, la page suivante l'ouvre à sa place (seule dans
 *  son chapitre, elle l'emporte). */
export function passChapterOn(pages: Page[], i: number): Page[] {
  const next = pages[i + 1];
  if (!pages[i]?.chapter || !next || next.chapter) return pages;
  return pages.map((p, k) => (k === i ? { ...p, chapter: null } : k === i + 1 ? { ...p, chapter: pages[i].chapter } : p));
}

/** Les pages sans la page `i` ; son chapitre passe à la suivante. */
export function withoutPage(pages: Page[], i: number): Page[] {
  return passChapterOn(pages, i).filter((_, k) => k !== i);
}

/** Les pages qui remplacent la page `replaced` (modèle choisi, pages faites pour chaque enfant) : la première garde
 *  son chapitre, les autres n'en ouvrent pas. */
export function carryChapter(replaced: Page, pages: Page[]): Page[] {
  return pages.map((p, k) => ({ ...p, chapter: k === 0 ? (replaced.chapter ?? null) : null }));
}

/** Le chapitre qui commence à `start` échange sa place avec le chapitre d'avant (`-1`) ou d'après (`1`), toutes ses
 *  pages avec lui ; `null` s'il n'y en a pas. Les pages d'avant le premier chapitre restent en tête. */
export function moveChapter(doc: Document, start: number, dir: -1 | 1): { doc: Document; start: number } | null {
  const spans = chapterSpans(doc.pages);
  const k = spans.findIndex((c) => c.start === start);
  const other = spans[k + dir];
  if (k < 0 || !other) return null;
  const [a, b] = dir < 0 ? [other, spans[k]] : [spans[k], other];
  const p = doc.pages;
  const pages = [...p.slice(0, a.start), ...p.slice(b.start, b.end), ...p.slice(a.start, a.end), ...p.slice(b.end)];
  return { doc: { ...doc, pages }, start: dir < 0 ? a.start : a.start + (b.end - b.start) };
}

/** Les miniatures cachées dans la bande : les pages des chapitres autres que celui de la page affichée `current`, réduits
 *  à leur onglet. Les pages d'avant le premier chapitre restent visibles. */
export function hiddenPages(pages: Page[], current: number): boolean[] {
  const open = chapterOf(pages, current);
  const hide = pages.map(() => false);
  for (const c of chapterSpans(pages)) if (c.start !== open?.start) for (let i = c.start; i < c.end; i++) hide[i] = true;
  return hide;
}

/** Les signets d'un PDF des pages `start` à `end` (exclue) : un par chapitre qui commence dans ces pages, avec le rang
 *  de sa première page dans le PDF (0 : la première). Le chapitre où commencent ces pages compte aussi. */
export function chapterOutline(pages: Page[], start = 0, end = pages.length): { title: string; page: number }[] {
  return chapterSpans(pages)
    .filter((c) => c.end > start && c.start < end)
    .map((c) => ({ title: c.title, page: Math.max(c.start, start) - start }));
}

/** Nom de fichier pour un chapitre : « Cahier de vie - Septembre », sans les signes qu'un nom de fichier refuse. */
export function chapterFileName(base: string, title: string): string {
  const clean = title.replace(/[\\/:*?"<>|]+/g, " ").replace(/\s+/g, " ").trim();
  return clean ? `${base} - ${clean}` : base;
}

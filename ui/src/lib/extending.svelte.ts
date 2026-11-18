// Prolonger un trait clic après clic : chaque clic ajoute un point au bout choisi. Toute la séance est un aperçu,
// qui devient un seul pas d'annulation à la fin (`finish`). `undoLast` retire le dernier point ajouté.
import { editor } from "./docStore.svelte";
import { throughPoints } from "./elementEdit";
import { dragPoint, extend, MAX_PATH_POINTS, pagePoints, type Pt } from "./linePath";
import type { Document } from "./types";

type Lines = { xs: number[]; ys: number[] };

class Extending {
  /** Un prolongement est en cours. */
  active = $state(false);
  /** Le dernier clic a été refusé : le chemin a déjà 200 points. */
  full = $state(false);
  /** Où va le prochain segment (le trait fin qui suit la souris), en mm de page. */
  preview = $state<Pt | null>(null);
  id = "";
  from: "start" | "end" = "end";
  #base: Document | null = null;
  /** Le dernier aperçu montré par cette séance, et le document ouvert à son début : `finish` ne touche au document
   *  que s'il est encore là (une autre action, un autre document, ne sont jamais défaits). */
  #shown: Document | null = null;
  #generation = 0;
  #unflush: (() => void) | null = null;
  #pts = $state.raw<Pt[]>([]);
  #added = 0;

  /** Points du chemin en cours, dans l'ordre du trait. */
  get points(): Pt[] {
    return this.#pts;
  }

  /** Le bout qui se prolonge. */
  get tip(): Pt | null {
    return this.#pts.length ? this.#pts[this.from === "end" ? this.#pts.length - 1 : 0] : null;
  }

  start(id: string, from: "start" | "end") {
    if (this.active) this.finish();
    const e = editor.page.elements.find((o) => o.id === id);
    if (!e || e.kind.type !== "Shape") return;
    this.id = id;
    this.from = from;
    this.#base = editor.doc;
    this.#pts = pagePoints(e.frame, e.kind, !!e.flip_h, !!e.flip_v);
    this.#added = 0;
    this.full = false;
    this.preview = null;
    this.#shown = null;
    this.#generation = editor.generation;
    // Enregistrer, ouvrir un autre document, fermer : la séance se termine d'abord (un pas d'annulation).
    this.#unflush = editor.onFlush(() => this.finish());
    this.active = true;
  }

  /** Le point où irait un clic en `p` : accroché comme un point qu'on tire. */
  snap(p: Pt, shift: boolean, lines: Lines, tolMm: number): Pt {
    const next = extend(this.#pts, this.from, p);
    if (next === this.#pts) return p;
    const i = this.from === "end" ? next.length - 1 : 0;
    return dragPoint({ pts: next, i, to: p, shift, lines, tolMm })[i];
  }

  /** Ajoute un point en `p` (accroché). Au-delà de 200 points, refuse et finit. */
  add(p: Pt, shift: boolean, lines: Lines, tolMm: number) {
    if (!this.active) return;
    if (this.#pts.length >= MAX_PATH_POINTS) {
      this.full = true;
      this.finish();
      return;
    }
    this.#pts = extend(this.#pts, this.from, this.snap(p, shift, lines, tolMm));
    this.#added++;
    this.#show();
  }

  /** Retire le dernier point ajouté pendant cette séance. */
  undoLast() {
    if (!this.active || this.#added === 0) return;
    this.#pts = this.from === "end" ? this.#pts.slice(0, -1) : this.#pts.slice(1);
    this.#added--;
    this.#show();
  }

  /** Fin du prolongement : un seul pas d'annulation s'il a ajouté des points, rien sinon. */
  finish() {
    if (!this.active) return;
    this.#unflush?.();
    this.#unflush = null;
    // Le document montre encore notre aperçu : il devient un pas d'annulation (ou rien, sans point ajouté).
    if (this.#shown && editor.doc === this.#shown && editor.generation === this.#generation) {
      if (this.#added === 0 && this.#base) editor.preview(this.#base);
      editor.endPreview();
    }
    this.active = false;
    this.preview = null;
    this.#base = null;
    this.#shown = null;
  }

  #show() {
    const base = this.#base!;
    const page = base.pages[editor.pageIndex];
    const e = page?.elements.find((o) => o.id === this.id);
    if (!e || e.kind.type !== "Shape") return;
    const moved = throughPoints(e, e.kind, this.#pts);
    this.#shown = { ...base, pages: base.pages.map((p, k) => (k === editor.pageIndex ? { ...p, elements: p.elements.map((o) => (o.id === e.id ? moved : o)) } : p)) };
    editor.preview(this.#shown);
  }
}

/** Une touche pendant un prolongement : Échap et Entrée le terminent, Ctrl+Z retire le dernier point, une touche de
 *  modification seule ne fait rien ; toute autre touche (raccourci, flèche, Suppr) le termine puis fait son effet. */
export function extendKey(ev: { key: string; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean }): "finish" | "undo" | "pass" | "ignore" {
  if (ev.key === "Escape" || ev.key === "Enter") return "finish";
  if ((ev.ctrlKey || ev.metaKey) && !ev.shiftKey && ev.key.toLowerCase() === "z") return "undo";
  if (["Shift", "Control", "Alt", "Meta", "AltGraph"].includes(ev.key)) return "ignore";
  return "pass";
}

export const extending = new Extending();

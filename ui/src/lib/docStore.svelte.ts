// État de l'éditeur : le document (immuable, avec historique), la page courante, la sélection, le zoom.
import { createHistory, type History } from "./history";
import { newDocument } from "./factory";
import type { AssetMeta, Document, Element, Page } from "./types";

export type Zoom = number | "fit";

class Editor {
  doc = $state.raw<Document>(newDocument());
  path = $state<string | null>(null);
  pageIndex = $state(0);
  selection = $state<string[]>([]);
  zoom = $state<Zoom>("fit");
  fitZoom = $state(1);
  templateMode = $state(false);
  /** Photos importées pendant la session (pas forcément utilisées dans le document). */
  photos = $state.raw<AssetMeta[]>([]);
  /** Texte en cours d'édition. */
  editingText = $state<string | null>(null);
  /** Photo en cours de recadrage (double-clic). */
  cropping = $state<string | null>(null);
  /** Message à afficher (erreurs d'import, d'enregistrement...). */
  notice = $state<string | null>(null);
  canUndo = $state(false);
  canRedo = $state(false);
  dirty = $state(false);
  #history: History<Document> = createHistory(this.doc);

  get page(): Page {
    return this.doc.pages[Math.min(this.pageIndex, this.doc.pages.length - 1)];
  }

  get effectiveZoom(): number {
    return this.zoom === "fit" ? this.fitZoom : this.zoom;
  }

  get selected(): Element | undefined {
    return this.selection.length === 1 ? this.page.elements.find((e) => e.id === this.selection[0]) : undefined;
  }

  #sync() {
    this.doc = this.#history.current;
    this.canUndo = this.#history.canUndo;
    this.canRedo = this.#history.canRedo;
    this.dirty = this.#history.isDirty;
    if (this.pageIndex >= this.doc.pages.length) this.pageIndex = this.doc.pages.length - 1;
    const ids = new Set(this.page.elements.map((e) => e.id));
    this.selection = this.selection.filter((id) => ids.has(id));
  }

  commit(next: Document) {
    this.#history.commit(next);
    this.#sync();
  }

  undo() {
    this.#history.undo();
    this.#sync();
  }

  redo() {
    this.#history.redo();
    this.#sync();
  }

  markSaved(path: string) {
    this.path = path;
    this.#history.markSaved();
    this.#sync();
  }

  /** Nouveau document ou document ouvert : l'historique repart de zéro. */
  reset(doc: Document, path: string | null) {
    this.#history = createHistory(doc);
    this.path = path;
    this.pageIndex = 0;
    this.selection = [];
    this.#sync();
  }

  updatePage(fn: (p: Page) => Page) {
    const pages = this.doc.pages.map((p, i) => (i === this.pageIndex ? fn(p) : p));
    this.commit({ ...this.doc, pages });
  }

  addPhotos(metas: AssetMeta[]) {
    const known = new Set(this.photos.map((p) => p.id));
    this.photos = [...this.photos, ...metas.filter((m) => !known.has(m.id))];
  }

  /** Met une photo dans un élément photo ; l'image rejoint les images du document. */
  placePhoto(elementId: string, meta: AssetMeta) {
    const pages = this.doc.pages.map((p, i) =>
      i !== this.pageIndex
        ? p
        : {
            ...p,
            elements: p.elements.map((e) =>
              e.id === elementId && e.kind.type === "Photo"
                ? { ...e, kind: { ...e.kind, asset: meta.id, crop: { scale: 1, center_x: 0.5, center_y: 0.5 } } }
                : e,
            ),
          },
    );
    this.commit({ ...this.doc, pages, assets: { ...this.doc.assets, [meta.id]: meta } });
  }

  updateElement(id: string, fn: (e: Element) => Element) {
    this.updatePage((p) => ({ ...p, elements: p.elements.map((e) => (e.id === id ? fn(e) : e)) }));
  }
}

export const editor = new Editor();

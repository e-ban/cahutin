// État de l'éditeur : le document (immuable, avec historique), la page courante, la sélection, le zoom.
import { createHistory, type History } from "./history";
import { newDocument } from "./factory";
import { readPref, writePref } from "./prefs";
import type { AssetMeta, Document, Element, Page } from "./types";

export type Zoom = number | "fit";
export type DrawerId = "templates" | "photos" | "text" | "shapes" | "fonts" | "grid";

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
  /** Pages d'impression rendues seulement pendant une impression ou un export PDF. */
  printing = $state(false);
  /** Choix d'un modèle au démarrage et pour un nouveau document. */
  showPicker = $state(false);
  /** Message à afficher (erreurs d'import, d'enregistrement...). */
  notice = $state<string | null>(null);
  /** Tiroir ouvert à côté de la colonne d'icônes (un seul à la fois). */
  drawer = $state<DrawerId | null>(null);
  #inspectorOpen = $state(readPref("inspectorOpen", false));
  canUndo = $state(false);
  canRedo = $state(false);
  dirty = $state(false);
  #history: History<Document> = createHistory(this.doc);

  /** Panneau de droite (réglages avancés) ; fermé par défaut, mémorisé sur le poste. */
  get inspectorOpen(): boolean {
    return this.#inspectorOpen;
  }

  set inspectorOpen(open: boolean) {
    this.#inspectorOpen = open;
    writePref("inspectorOpen", open);
  }

  /** Re-cliquer l'icône du tiroir ouvert le referme. */
  toggleDrawer(id: DrawerId) {
    this.drawer = this.drawer === id ? null : id;
  }

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

  markSaved(path: string, snapshot?: Document) {
    this.path = path;
    this.#history.markSaved(snapshot);
    this.#sync();
  }

  // Éditions en cours hors du document (texte en cours de saisie, recadrage) : validées avant
  // d'enregistrer, d'ouvrir un autre document ou de fermer la fenêtre.
  #flushers = new Set<() => void>();

  onFlush(fn: () => void): () => void {
    this.#flushers.add(fn);
    return () => this.#flushers.delete(fn);
  }

  flushPending() {
    for (const fn of [...this.#flushers]) fn();
  }

  /** Nouveau document ou document ouvert : l'historique repart de zéro. */
  reset(doc: Document, path: string | null) {
    this.#history = createHistory(doc);
    this.path = path;
    this.pageIndex = 0;
    this.selection = [];
    this.#sync();
  }

  /** Modifie la page désignée, même si la page courante a changé entre-temps (après une attente). */
  updatePageById(id: string, fn: (p: Page) => Page) {
    if (!this.doc.pages.some((p) => p.id === id)) return;
    this.commit({ ...this.doc, pages: this.doc.pages.map((p) => (p.id === id ? fn(p) : p)) });
  }

  // Aperçu en direct (curseur que l'on fait glisser) : affiché tout de suite, mais un seul pas
  // d'annulation, enregistré quand on relâche.
  #previewBase: Document | null = null;

  preview(next: Document) {
    if (!this.#previewBase) this.#previewBase = this.#history.current;
    this.doc = next;
  }

  endPreview() {
    const base = this.#previewBase;
    if (!base) return;
    this.#previewBase = null;
    const final = this.doc;
    this.doc = base;
    if (final !== base) this.commit(final);
  }

  updatePage(fn: (p: Page) => Page) {
    const pages = this.doc.pages.map((p, i) => (i === this.pageIndex ? fn(p) : p));
    this.commit({ ...this.doc, pages });
  }

  addPhotos(metas: AssetMeta[]) {
    const known = new Set(this.photos.map((p) => p.id));
    this.photos = [...this.photos, ...metas.filter((m) => !known.has(m.id))];
  }

  /** Images connues sans modification du document (aperçus de modèles). */
  #extraAssets: Record<string, AssetMeta> = {};
  registerAssets(metas: AssetMeta[]) {
    for (const m of metas) this.#extraAssets[m.id] = m;
  }
  assetMeta(id: string): AssetMeta | undefined {
    return this.doc.assets[id] ?? this.#extraAssets[id];
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

export type DiscardChoice = "save" | "discard" | "cancel";

/**
 * Avant de quitter le document courant : rien à demander s'il est enregistré ; sinon « Enregistrer les
 * modifications ? ». Renvoie vrai si l'on peut continuer.
 */
export async function confirmDiscardIfDirty(ask: () => Promise<DiscardChoice>, save?: () => Promise<boolean>): Promise<boolean> {
  editor.flushPending();
  if (!editor.dirty) return true;
  const choice = await ask();
  if (choice === "cancel") return false;
  if (choice === "save") return save ? await save() : false;
  return true;
}

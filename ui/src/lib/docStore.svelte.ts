// État de l'éditeur : le document (immuable, avec historique), la page courante, la sélection, le zoom.
import { createHistory, type History } from "./history";
import { newDocument } from "./factory";
import { pathOf } from "./groups";
import { pagesUsing, usedAssetIds, withoutAsset, type LibraryPhoto } from "./photoLibrary";
import { readRecents, type Recent } from "./recents";
import { readPref, writePref } from "./prefs";
import type { AssetMeta, Document, Element, Page } from "./types";

export type Zoom = number | "fit";
export type ImagesTab = "photos" | "mine" | "pictos";
export type DrawerId = "templates" | "images" | "text" | "fields" | "shapes" | "fonts" | "grid" | "decor" | "class";

class Editor {
  doc = $state.raw<Document>(newDocument());
  path = $state<string | null>(null);
  pageIndex = $state(0);
  selection = $state<string[]>([]);
  zoom = $state<Zoom>("fit");
  fitZoom = $state(1);
  templateMode = $state(false);
  /** Photothèque de la session (pas forcément utilisées dans le document) ; gardée après « Nouveau ». */
  photos = $state.raw<LibraryPhoto[]>([]);
  /** Chemins en attente du document ouvert, gardés à l'enregistrement tant qu'ils ne sont pas réimportés. */
  carriedPending: string[] = [];
  /** Import en cours : photos traitées sur le total demandé. */
  importProgress = $state<{ done: number; total: number } | null>(null);
  /** Import de fichier en cours (Fichier > Importer) : nom du fichier converti. */
  fileImport = $state<{ name: string } | null>(null);
  /** Texte en cours d'édition. */
  editingText = $state<string | null>(null);
  /** Le texte qui s'ouvre est un texte modèle tout juste posé (« Titre ») : il est sélectionné, la frappe le remplace. */
  selectTextOnEdit = false;
  /** « Remplacer » (barre flottante, case de tableau) : le prochain clic dans le tiroir Décors remplace les images
   *  choisies au lieu d'en poser une nouvelle. Prend fin quand la sélection change. */
  replacingPictos = $state(false);
  /** « Remplacer » sur une photo posée : le prochain clic dans le tiroir Photos la remplace au lieu d'en poser une autre. */
  replacingPhoto = $state(false);
  /** Fusion en glissant (mode modèle) : cases couvertes, et si la fusion est possible. */
  mergePreview = $state<{ ids: string[]; ok: boolean } | null>(null);
  /** Élément d'un groupe modifié seul après un double-clic (Échap ou un clic ailleurs revient au groupe). */
  focusedMember = $state<string | null>(null);
  /** Groupe où l'on est entré par double-clic (son chemin) : un clic y prend un de ses sous-groupes. */
  insideGroup = $state<string[]>([]);
  /** Déplacement, redimensionnement ou rotation à la souris en cours (Moveable). */
  gesture = $state(false);
  /** Photo en cours de recadrage (double-clic). */
  cropping = $state<string | null>(null);
  /** Pages d'impression rendues seulement pendant une impression ou un export PDF. */
  printing = $state(false);
  /** Impression des pages A3 en deux moitiés sur A4. */
  printTiles = $state(false);
  /** Impression recto-verso de cartes : les pages paires (versos) inversées de gauche à droite. */
  printDuplex = $state(false);
  /** Pages en vis-à-vis dans la vue d'édition. */
  spread = $state(false);
  /** Panneau du fond de page. */
  showBackground = $state(false);
  /** La page vue comme la voit une personne daltonienne (bouton des Préférences). */
  cvdView = $state(false);
  /** Écran À propos (menu Aide). */
  showAbout = $state(false);
  /** Fenêtre des préférences de l'application. */
  showPreferences = $state(false);
  /** Panneau « Astuces » (menu Aide). */
  showTips = $state(false);
  /** Écran d'accueil : au démarrage et pour un nouveau document. */
  showWelcome = $state(false);
  /** L'accueil est ouvert par-dessus un document (logo, Fichier > Page d'accueil) : il garde ce document. */
  welcomeOverDoc = $state(false);
  /** Page « Ma classe », par-dessus le document (et l'accueil). */
  showClassPage = $state(false);
  /** Photothèque de la classe (page entière). */
  showPhototheque = $state(false);
  /** Page de la collection (thèmes, noms, images retirées), par-dessus tout. */
  showCollection = $state(false);
  /** Fenêtre des nouveautés de la version. */
  showWhatsNew = $state(false);
  /** Photo dont on floute des visages (id de l'image). */
  blurring = $state<string | null>(null);
  /** Documents récents (sur le poste seulement). */
  recents = $state.raw<Recent[]>(readRecents());
  #notice = $state<string | null>(null);
  /** Bouton d'action de l'avis (« Remettre dans la classe ») ; perdu dès qu'un autre avis le remplace. */
  noticeAction = $state<{ label: string; run: () => void } | null>(null);

  /** Message à afficher (erreurs d'import, d'enregistrement...). */
  get notice(): string | null {
    return this.#notice;
  }

  set notice(message: string | null) {
    this.#notice = message;
    this.noticeAction = null;
  }

  /** Avis avec, au besoin, un bouton qui défait ce qui vient d'être fait. */
  notify(message: string, action?: { label: string; run: () => void }) {
    this.notice = message;
    this.noticeAction = action ?? null;
  }
  /** Travail en cours sans progression mesurable (export) : un message sans bouton. */
  busy = $state<string | null>(null);
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
  /** Rubrique ouverte du tiroir Images (retenue d'une fois à l'autre). */
  imagesTab = $state<ImagesTab>(readPref("imagesTab", "pictos"));

  /** Ouvre le tiroir Images sur une rubrique. */
  openImages(tab: ImagesTab) {
    this.setImagesTab(tab);
    this.drawer = "images";
  }

  setImagesTab(tab: ImagesTab) {
    this.imagesTab = tab;
    writePref("imagesTab", tab);
  }

  toggleDrawer(id: DrawerId) {
    this.drawer = this.drawer === id ? null : id;
  }

  get page(): Page {
    return this.doc.pages[Math.min(this.pageIndex, this.doc.pages.length - 1)];
  }

  get effectiveZoom(): number {
    return this.zoom === "fit" ? this.fitZoom : this.zoom;
  }

  /** Le groupe où l'on est entré, tant que la sélection y reste ; sinon aucun. */
  get inside(): string[] {
    const sel = this.page.elements.filter((e) => this.selection.includes(e.id));
    const inIt = (e: Element) => this.insideGroup.every((g, i) => pathOf(e)[i] === g);
    return sel.length && sel.every(inIt) ? this.insideGroup : [];
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
    // Sélection réaffectée seulement si un élément a disparu : un menu ouvert de la barre flottante reste ouvert.
    const ids = new Set(this.page.elements.map((e) => e.id));
    if (this.selection.some((id) => !ids.has(id))) this.selection = this.selection.filter((id) => ids.has(id));
  }

  /** Proposition de l'assistant affichée : le document ne se modifie pas à la main tant qu'elle est là. */
  proposing = $state(false);
  #proposal: Document | null = null;

  /** Affiche `doc` sans toucher à l'historique ; `null` : retour au document. */
  showProposal(doc: Document | null) {
    this.#proposal = doc;
    this.proposing = doc !== null;
    if (doc === null) return this.#sync();
    this.doc = doc;
    if (this.pageIndex >= doc.pages.length) this.pageIndex = doc.pages.length - 1;
    const ids = new Set(this.page.elements.map((e) => e.id));
    if (this.selection.some((id) => !ids.has(id))) this.selection = this.selection.filter((id) => ids.has(id));
  }

  /** L'assistant décide d'une modification à la main ou d'une annulation pendant sa proposition. */
  proposalGate: { edit: (next: Document) => void; undo: () => void } | null = null;

  commit(next: Document) {
    // Pendant une proposition, une modification à la main attend que l'on garde ou non la proposition (un aperçu
    // en cours est effacé).
    if (this.#proposal) {
      this.doc = this.#proposal;
      this.proposalGate?.edit(next);
      return;
    }
    this.#history.commit(next);
    this.#sync();
  }

  undo() {
    // Annuler pendant une proposition : la refuser.
    if (this.#proposal) return this.proposalGate?.undo();
    this.#history.undo();
    this.#sync();
    // Une photo retirée revient sur la page : elle revient aussi dans le tiroir.
    const used = usedAssetIds(this.doc);
    this.addPhotos(this.#removedPhotos.filter((p) => used.has(p.meta.id)));
  }

  redo() {
    if (this.#proposal) return;
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
  /** `unsaved` : document repris d'une copie de secours, à enregistrer (donc « modifié »). */
  reset(doc: Document, path: string | null, opts: { unsaved?: boolean } = {}) {
    this.#proposal = null;
    this.proposing = false;
    this.#history = createHistory(doc);
    if (opts.unsaved) this.#history.markUnsaved();
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

  /** Modifie la page affichée ; `assets` : images qu'elle utilise désormais, ajoutées à celles du document. */
  updatePage(fn: (p: Page) => Page, assets: AssetMeta[] = []) {
    const pages = this.doc.pages.map((p, i) => (i === this.pageIndex ? fn(p) : p));
    this.commit({ ...this.doc, pages, ...(assets.length ? { assets: { ...this.doc.assets, ...Object.fromEntries(assets.map((m) => [m.id, m])) } } : {}) });
  }

  /** Une photo déjà présente (même id, ou même fichier importé) n'est pas ajoutée une seconde fois. */
  addPhotos(photos: LibraryPhoto[]) {
    const ids = new Set(this.photos.map((p) => p.meta.id));
    const paths = new Set(this.photos.map((p) => p.path).filter(Boolean));
    const fresh: LibraryPhoto[] = [];
    for (const p of photos) {
      if (ids.has(p.meta.id) || (p.path && paths.has(p.path))) continue;
      ids.add(p.meta.id);
      if (p.path) paths.add(p.path);
      fresh.push(p);
    }
    if (fresh.length) this.photos = [...this.photos, ...fresh];
  }

  /** Retire une photo du tiroir, et des pages où elle est posée (ses cases restent, vides) : annulable. Le
   *  fichier sur l'ordinateur n'est pas touché. */
  #removedPhotos: LibraryPhoto[] = [];
  removePhoto(id: string) {
    const gone = this.photos.find((p) => p.meta.id === id);
    if (gone) this.#removedPhotos.push(gone);
    this.photos = this.photos.filter((p) => p.meta.id !== id);
    if (gone?.path) this.carriedPending = this.carriedPending.filter((path) => path !== gone.path);
    if (pagesUsing(this.doc, id).length) this.commit(withoutAsset(this.doc, id));
  }

  /** Document ouvert : la photothèque devient la sienne (ses photos, puis ses photos en attente). */
  replaceLibrary(photos: LibraryPhoto[]) {
    this.photos = [];
    this.addPhotos(photos);
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

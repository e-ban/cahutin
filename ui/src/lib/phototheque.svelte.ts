// La photothèque de la classe ouverte : son index (montré tout de suite), le parcours des dossiers suivis en
// arrière-plan, les dossiers suivis, les étiquettes, « Retirer », les originaux des anciennes copies, et le
// passage d'une photo dans le document.
import { open as pickFolder } from "@tauri-apps/plugin-dialog";
import { untrack } from "svelte";
import {
  photothequeApplyOriginals,
  photothequeFindOriginals,
  photothequeFollow,
  photothequeHide,
  photothequeOpen,
  photothequeScan,
  photothequeTag,
  photothequeUnfollow,
  photothequeUse,
} from "./api";
import { ask } from "./ask";
import { classes } from "./classStore.svelte";
import { editor } from "./docStore.svelte";
import { fr } from "./i18n/fr";
import { baseName, displayNames, missingConsent, upsertPhotos, visiblePhotos } from "./phototheque";
import type { AssetMeta, ClassPhoto, FollowedFolder, Match, OriginalsReport, Phototheque, TagChange } from "./types";

class ClassLibrary {
  /** Classe dont la photothèque est chargée (chemin de son fichier). */
  classPath = $state<string | null>(null);
  /** Tout l'index, photos retirées et absentes comprises (pour les autorisations des photos déjà posées). */
  photos = $state.raw<ClassPhoto[]>([]);
  folders = $state.raw<FollowedFolder[]>([]);
  /** Les photos montrées : ni retirées ni absentes. */
  visible = $derived(visiblePhotos(this.photos));
  /** Dossiers suivis introuvables au dernier parcours (clé USB débranchée…). */
  missingFolders = $derived(this.folders.filter((f) => f.missing).map((f) => f.path));
  /** Parcours en cours : photos nouvelles ou changées reçues jusqu'ici. */
  scanning = $state<{ found: number } | null>(null);

  /** Change à chaque ouverture : un parcours lancé pour une autre classe ne touche plus la liste. */
  #opening = 0;
  /** Parcours en cours (un suivi de dossier peut en lancer un pendant qu'un autre tourne). */
  #scans = 0;

  #set(lib: Phototheque) {
    this.photos = lib.photos;
    this.folders = lib.folders;
  }

  /** Ouverture en cours : une seconde demande pour la même classe (page et tiroir) attend la même. */
  #pending: { classPath: string; done: Promise<void> } | null = null;

  /** L'index tout de suite, puis le parcours des dossiers suivis en arrière-plan (sans l'attendre). */
  open(classPath: string): Promise<void> {
    if (this.#pending?.classPath === classPath) return this.#pending.done;
    const done = this.#open(classPath).finally(() => {
      if (this.#pending?.done === done) this.#pending = null;
    });
    this.#pending = { classPath, done };
    return done;
  }

  async #open(classPath: string) {
    const token = ++this.#opening;
    const lib = await photothequeOpen(classPath);
    if (token !== this.#opening) return;
    this.classPath = classPath;
    this.#set(lib);
    this.scanning = { found: 0 };
    this.scan().catch((e) => {
      if (token === this.#opening) editor.notice = String(e);
    });
  }

  /** La photothèque déjà chargée est rouverte (page, ou vue agrandie du tiroir) : un parcours, pour montrer les
   *  photos ajoutées depuis dans les dossiers suivis ; rien si une ouverture ou un parcours est en cours. Lue
   *  hors suivi, pour qu'un `$effect` qui l'appelle ne se relance pas à chaque lot reçu. */
  refresh() {
    untrack(() => {
      if (this.#pending || this.#scans > 0 || !this.classPath) return;
      const token = this.#opening;
      this.scan().catch((e) => {
        if (token === this.#opening) editor.notice = String(e);
      });
    });
  }

  /** Parcourt les dossiers suivis : les photos arrivent au fil de l'eau, puis l'index complet remplace la liste. */
  async scan() {
    const token = this.#opening;
    this.#scans++;
    this.scanning ??= { found: 0 };
    try {
      await photothequeScan((e) => {
        if (token !== this.#opening) return;
        if (e.type === "Photos") {
          this.photos = upsertPhotos(this.photos, e.photos);
          // Un lot arrivé après la fin du parcours ne rallume pas la barre.
          if (this.scanning) this.scanning = { found: this.scanning.found + e.photos.length };
        } else {
          this.#set(e.phototheque);
        }
      });
    } finally {
      if (--this.#scans === 0) this.scanning = null;
    }
  }

  /** Suit un dossier (choisi par l'enseignant, ou `path` donné), puis le parcourt ; `false` si rien n'est choisi. */
  async follow(path?: string): Promise<boolean> {
    const folder = path ?? (await pickFolder({ directory: true }));
    if (typeof folder !== "string") return false;
    this.#set(await photothequeFollow(folder));
    await this.scan();
    return true;
  }

  /** Ne plus suivre un dossier, après confirmation : ses photos restent sur l'ordinateur. */
  async unfollow(folder: string): Promise<boolean> {
    if (!(await ask(fr.libraryUnfollowConfirm(baseName(folder)), fr.libraryUnfollow))) return false;
    this.#set(await photothequeUnfollow(folder));
    return true;
  }

  async tag(ids: string[], change: Partial<TagChange>) {
    this.#set(await photothequeTag(ids, { add_students: [], remove_students: [], add_tags: [], remove_tags: [], ...change }));
  }

  /** « Retirer » : les photos ne sont plus montrées (aucun fichier touché) ; « Annuler » les remet. */
  async hide(ids: string[]) {
    if (!ids.length) return;
    const token = this.#opening;
    this.#set(await photothequeHide(ids, true));
    editor.notify(fr.libraryHidden(ids.length), {
      label: fr.undo,
      run: () => {
        if (token !== this.#opening) return;
        photothequeHide(ids, false)
          .then((lib) => token === this.#opening && this.#set(lib))
          .catch((e) => (editor.notice = String(e)));
      },
    });
  }

  /** Les originaux des anciennes copies, cherchés dans les dossiers suivis ; rien n'est changé. */
  findOriginals(): Promise<OriginalsReport> {
    return photothequeFindOriginals();
  }

  /** Reporte enfants et activités sur les originaux et met les copies à la corbeille de l'ordinateur. */
  async applyOriginals(matches: Match[]) {
    this.#set(await photothequeApplyOriginals(matches));
  }

  /** Les photos rejoignent la photothèque du document, prêtes à glisser sur la page ; rend leurs descriptions. */
  async useInDocument(ids: string[]): Promise<AssetMeta[]> {
    const metas = await photothequeUse(ids);
    editor.addPhotos(metas.map((meta) => ({ meta, path: "", folder: "Photothèque de la classe" })));
    return metas;
  }
}

export const classLibrary = new ClassLibrary();

/** Des photos de la photothèque posées sur la page : si elles montrent des enfants sans autorisation, on le dit
 *  en un seul avis (les flouter avant d'imprimer). */
export function warnConsent(assetIds: string | string[]) {
  const students = classes.current?.class.students ?? [];
  const missing = missingConsent(classLibrary.photos, students, [assetIds].flat());
  const names = displayNames(students);
  if (missing.length) editor.notice = fr.libraryPlacedNoConsent(missing.map((s) => names.get(s.id) ?? s.first_name));
}

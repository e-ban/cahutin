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
  photothequeRefollow,
  photothequeScan,
  photothequeSettleMigration,
  photothequeStopOriginals,
  photothequeTag,
  photothequeUnfollow,
  photothequeUse,
} from "./api";
import { ask } from "./ask";
import { classes } from "./classStore.svelte";
import { editor } from "./docStore.svelte";
import { fr } from "./i18n/fr";
import { baseName, displayNames, missingConsent, upsertPhotos, visiblePhotos } from "./phototheque";
import type { AssetMeta, ClassPhoto, FollowedFolder, Match, Phototheque, TagChange } from "./types";

class ClassLibrary {
  /** Classe dont la photothèque est chargée (chemin de son fichier). */
  classPath = $state<string | null>(null);
  /** Tout l'index, photos retirées et absentes comprises (pour les autorisations des photos déjà posées). */
  photos = $state.raw<ClassPhoto[]>([]);
  folders = $state.raw<FollowedFolder[]>([]);
  /** Recherche des originaux en cours : photos regardées, sur combien (0 sur 0 tant qu'elle démarre). */
  searching = $state<{ seen: number; total: number } | null>(null);
  /** La question de la migration est tranchée (faite, ou « Non merci ») : son bandeau ne revient plus. */
  migrationSettled = $state(false);
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
    this.migrationSettled = lib.migration_settled;
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
    // Une recherche des originaux de la classe quittée s'arrête : sa barre ne reste pas sur la nouvelle classe.
    if (this.searching) await photothequeStopOriginals().catch(() => {});
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

  /** Ne plus suivre un dossier, après confirmation : ses photos restent sur l'ordinateur ; « Annuler » dans
   *  l'avis remet le dossier et ses photos, avec leurs élèves et activités. */
  async unfollow(folder: string): Promise<boolean> {
    if (!(await ask(fr.libraryUnfollowConfirm(baseName(folder)), fr.libraryUnfollow))) return false;
    const token = this.#opening;
    this.#set(await photothequeUnfollow(folder));
    editor.notify(fr.libraryUnfollowed(baseName(folder)), {
      label: fr.undo,
      run: () => {
        if (token !== this.#opening) return;
        photothequeRefollow(folder)
          .then((lib) => token === this.#opening && this.#set(lib))
          .catch((e) => (editor.notice = String(e)));
      },
    });
    return true;
  }

  /** Les photos changées par un clic rejoignent la liste à leur place ; une réponse arrivée après le
   *  changement de classe est laissée. */
  #merge(token: number, changed: ClassPhoto[]) {
    if (token === this.#opening) this.photos = upsertPhotos(this.photos, changed);
  }

  async tag(ids: string[], change: Partial<TagChange>) {
    const token = this.#opening;
    this.#merge(token, await photothequeTag(ids, { add_students: [], remove_students: [], add_tags: [], remove_tags: [], ...change }));
  }

  /** « Retirer » : les photos ne sont plus montrées (aucun fichier touché) ; « Annuler » les remet. */
  async hide(ids: string[]) {
    if (!ids.length) return;
    const token = this.#opening;
    this.#merge(token, await photothequeHide(ids, true));
    editor.notify(fr.libraryHidden(ids.length), {
      label: fr.undo,
      run: () => {
        if (token !== this.#opening) return;
        photothequeHide(ids, false)
          .then((changed) => this.#merge(token, changed))
          .catch((e) => (editor.notice = String(e)));
      },
    });
  }

  /** « Non merci » : l'enseignant ne veut pas de la migration ; rien n'est touché, le bandeau ne revient plus. */
  async settleMigration() {
    this.#set(await photothequeSettleMigration());
  }

  /** Migration : cherche les originaux des anciennes copies dans les dossiers suivis, dit combien sont
   *  retrouvés, puis demande avant la corbeille. « Non » : la question est tranchée comme « Non merci » (le
   *  bandeau ne revient plus, le lien de la colonne reste). Aucun original retrouvé, ou recherche arrêtée :
   *  rien n'est tranché, l'enseignant peut ajouter un dossier et recommencer. Une seule recherche à la fois. */
  async migrate() {
    if (this.searching) return;
    const token = this.#opening;
    this.searching = { seen: 0, total: 0 };
    let report;
    try {
      report = await photothequeFindOriginals((p) => {
        if (this.searching) this.searching = p;
      });
    } finally {
      this.searching = null;
    }
    if (token !== this.#opening) return;
    if (!report) {
      editor.notice = fr.libraryOriginalsStopped;
      return;
    }
    const n = report.matches.length;
    if (n === 0) {
      editor.notice = fr.libraryOriginalsNone;
      return;
    }
    if (!(await ask(fr.libraryOriginalsFound(n, report.unmatched.length), fr.yes, fr.no))) {
      if (token === this.#opening && !this.migrationSettled) {
        await this.settleMigration();
        editor.notice = fr.libraryMigrationDeclined;
      }
      return;
    }
    if (token !== this.#opening) return;
    const left = await this.applyOriginals(report.matches);
    editor.notice = left ? fr.libraryOriginalsNotTrashed(n - left, left) : fr.libraryOriginalsDone(n);
  }

  /** « Arrêter » la recherche des originaux : elle s'arrête à la photo en cours. */
  async stopSearch() {
    await photothequeStopOriginals();
  }

  /** Met les copies à la corbeille de l'ordinateur et reporte enfants et activités de celles parties sur les
   *  originaux ; rend le nombre de copies que la corbeille a refusées (restées montrées, avec leurs étiquettes). */
  async applyOriginals(matches: Match[]): Promise<number> {
    const done = await photothequeApplyOriginals(matches);
    this.#set(done.phototheque);
    return done.not_trashed;
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

// La photothèque de la classe ouverte : son index, l'ajout d'un lot de photos (avec sa progression), les
// étiquettes, et le passage d'une photo dans le document.
import { photothequeAdd, photothequeOpen, photothequeRemove, photothequeTag, photothequeUse } from "./api";
import { classes } from "./classStore.svelte";
import { editor } from "./docStore.svelte";
import { fr } from "./i18n/fr";
import { withoutConsent } from "./phototheque";
import type { ClassPhoto, TagChange } from "./types";

class ClassLibrary {
  /** Classe dont la photothèque est chargée (chemin de son fichier). */
  classPath = $state<string | null>(null);
  photos = $state.raw<ClassPhoto[]>([]);
  progress = $state<{ done: number; total: number; failed: number } | null>(null);

  async open(classPath: string) {
    const lib = await photothequeOpen(classPath);
    this.classPath = classPath;
    this.photos = lib.photos;
  }

  /** Ajoute un lot ; rend les identifiants des photos ajoutées (à trier ensuite). */
  async add(paths: string[]): Promise<string[]> {
    if (!paths.length) return [];
    const added: string[] = [];
    this.progress = { done: 0, total: paths.length, failed: 0 };
    try {
      const lib = await photothequeAdd(paths, (o) => {
        if (!this.progress) return;
        this.progress = { ...this.progress, done: this.progress.done + 1, failed: this.progress.failed + (o.type === "Err" ? 1 : 0) };
        if (o.type === "Ok") added.push(o.photo.id);
      });
      this.photos = lib.photos;
      return added;
    } finally {
      this.progress = null;
    }
  }

  async tag(ids: string[], change: Partial<TagChange>) {
    const lib = await photothequeTag(ids, { add_students: [], remove_students: [], add_tags: [], remove_tags: [], ...change });
    this.photos = lib.photos;
  }

  async remove(ids: string[]) {
    this.photos = (await photothequeRemove(ids)).photos;
  }

  /** Les photos rejoignent la photothèque du document, prêtes à glisser sur la page. */
  async useInDocument(ids: string[]) {
    const metas = await photothequeUse(ids);
    editor.addPhotos(metas.map((meta) => ({ meta, path: "", folder: "Photothèque de la classe" })));
    return metas.length;
  }
}

export const classLibrary = new ClassLibrary();

/** Une photo de la photothèque posée sur la page : si elle montre un enfant sans autorisation, on le dit
 *  (la flouter avant d'imprimer). */
export function warnConsent(assetId: string) {
  const p = classLibrary.photos.find((x) => x.id === assetId);
  const missing = p ? withoutConsent(p, classes.current?.class.students ?? []) : [];
  if (missing.length) editor.notice = fr.libraryPlacedNoConsent(missing.map((s) => s.first_name));
}

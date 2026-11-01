// « Mes images » : la liste du dossier de l'enseignant, l'ajout, le retrait, et le passage d'une image dans le
// document (copiée dedans, une seule fois).
import { myImagesAdd, myImagesKeep, myImagesList, myImagesRemove, myImagesRemoved, myImagesRename, myImagesRestore, myImagesTheme, myImagesUse } from "./api";
import { classes } from "./classStore.svelte";
import { editor } from "./docStore.svelte";
import { fr } from "./i18n/fr";
import { showsChildren } from "./myImages";
import { classLibrary } from "./phototheque.svelte";
import type { AssetMeta, MyImage } from "./types";

class MyImages {
  images = $state.raw<MyImage[]>([]);
  /** Images retirées (sous-dossier Retirées), chargées par la page de la collection. */
  removed = $state.raw<MyImage[]>([]);

  async load() {
    try {
      this.images = await myImagesList();
    } catch {
      this.images = [];
    }
  }

  /** Copie les fichiers choisis ; rend le nombre de fichiers laissés (pas des images). */
  async add(paths: string[]): Promise<number> {
    if (!paths.length) return 0;
    const r = await myImagesAdd(paths);
    this.images = r.images;
    return r.refused;
  }

  async remove(name: string) {
    this.images = await myImagesRemove(name);
  }

  async loadRemoved() {
    this.removed = await myImagesRemoved().catch(() => []);
  }

  async restore(name: string) {
    this.images = await myImagesRestore(name);
    await this.loadRemoved();
  }

  async rename(name: string, newName: string) {
    this.images = await myImagesRename(name, newName);
  }

  /** Range les images dans un thème (`add`) ou les en sort (`remove`). */
  async theme(names: string[], add: string | null, remove: string | null = null) {
    this.images = await myImagesTheme(names, add, remove);
  }

  use(name: string): Promise<AssetMeta> {
    return myImagesUse(name);
  }

  /** « Garder dans Mes images » : l'image posée rejoint le dossier, sous le nom de son fichier ; jamais une
   *  photo d'enfants. Le résultat est dit en bas de l'écran. */
  async keep(assetId: string) {
    // La photothèque de la classe n'est chargée qu'à son ouverture : sans elle, une photo d'enfant passerait.
    const cls = classes.current?.path;
    if (cls && classLibrary.classPath !== cls) await classLibrary.open(cls).catch(() => {});
    if (showsChildren(assetId, classLibrary.photos, classes.current?.class.photos ?? {})) {
      editor.notice = fr.keepRefusedChildren;
      return;
    }
    const path = editor.photos.find((p) => p.meta.id === assetId)?.path ?? "";
    try {
      const r = await myImagesKeep(assetId, path);
      this.images = r.images;
      editor.notice = r.added ? fr.keptInMyImages : fr.alreadyInMyImages;
    } catch (e) {
      editor.notice = String(e);
    }
  }
}

export const myImages = new MyImages();

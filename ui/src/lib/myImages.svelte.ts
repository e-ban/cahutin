// « Mes images » : la liste du dossier de l'enseignant, l'ajout, le retrait, et le passage d'une image dans le
// document (copiée dedans, une seule fois).
import { myImagesAdd, myImagesList, myImagesRemove, myImagesUse } from "./api";
import type { AssetMeta, MyImage } from "./types";

class MyImages {
  images = $state.raw<MyImage[]>([]);

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

  use(name: string): Promise<AssetMeta> {
    return myImagesUse(name);
  }
}

export const myImages = new MyImages();

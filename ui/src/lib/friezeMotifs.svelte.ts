// Motifs des frises, choisis dans le tiroir Décors et repris par la frise autour d'un bloc : un pictogramme
// (son code) ou une image de la collection (« mine: » puis son nom de fichier). Une frise posée que l'on
// modifie nomme aussi ses images déjà dans le document (« asset: » puis leur id). Avec la taille des motifs et
// l'écart minimal entre eux.
import { assetUrl, myImageUrl } from "./api";
import { FRIEZE_GAP_MM, FRIEZE_SIZE_MM, type Motif } from "./decor";
import { myImages } from "./myImages.svelte";
import { pictoUrl } from "./pictos.svelte";
import type { AssetMeta } from "./types";

export const friezeMotifs = $state({ codes: ["2600"], size: FRIEZE_SIZE_MM, gap: FRIEZE_GAP_MM });

const MINE = "mine:";
const ASSET = "asset:";
export const mineMotif = (name: string) => MINE + name;

/** L'image d'un motif, pour l'aperçu. */
export const motifUrl = (key: string) =>
  key.startsWith(MINE) ? myImageUrl(key.slice(MINE.length)) : key.startsWith(ASSET) ? assetUrl(key.slice(ASSET.length), true) : pictoUrl(key);

/** Les motifs prêts à poser : une image de la collection rejoint les images de l'application. Rend aussi ces
 *  images, à ajouter au document ; celles qui ne sont plus dans la collection sont laissées. */
export async function resolveMotifs(keys: string[], inDoc: (id: string) => AssetMeta | undefined = () => undefined): Promise<{ motifs: Motif[]; assets: AssetMeta[] }> {
  const motifs: Motif[] = [];
  const assets: AssetMeta[] = [];
  for (const k of keys) {
    if (k.startsWith(ASSET)) {
      const meta = inDoc(k.slice(ASSET.length));
      if (meta) motifs.push({ asset: meta });
      continue;
    }
    if (!k.startsWith(MINE)) {
      motifs.push({ code: k });
      continue;
    }
    const meta = await myImages.use(k.slice(MINE.length)).catch(() => null);
    if (meta) {
      motifs.push({ asset: meta });
      assets.push(meta);
    }
  }
  return { motifs, assets };
}

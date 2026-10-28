// Polices ajoutées par l'utilisateur : chargées dans la vue avec l'API FontFace, depuis le protocole carnet-asset.
import { fontUrl } from "./api";
import type { FontFileInfo } from "./types";

const loaded = new Map<string, FontFace>();

/** Charge les fichiers qui ne le sont pas encore et retire ceux qui ont disparu (police supprimée). */
export async function registerUserFonts(files: FontFileInfo[]): Promise<void> {
  const wanted = new Set(files.map((f) => f.url_path));
  for (const [path, face] of loaded) {
    if (!wanted.has(path)) {
      document.fonts.delete(face);
      loaded.delete(path);
    }
  }
  await Promise.all(
    files
      .filter((f) => !loaded.has(f.url_path))
      .map(async (f) => {
        const face = new FontFace(f.family, `url("${fontUrl(f.url_path)}")`, { weight: f.bold ? "bold" : "normal", style: f.italic ? "italic" : "normal" });
        try {
          await face.load();
          document.fonts.add(face);
          loaded.set(f.url_path, face);
        } catch {
          // Fichier illisible par la vue : la famille reste listée, le texte s'affiche en Andika.
        }
      }),
  );
}

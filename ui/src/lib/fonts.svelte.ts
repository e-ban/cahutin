import { fontFiles, listFonts } from "./api";
import { EMBEDDED_FAMILIES, setKnownFamilies } from "./fontNames";
import { registerUserFonts } from "./userFonts.svelte";
import type { FontFamilyInfo } from "./types";

const KINDS: Record<string, string> = { Andika: "scripte", "Atkinson Hyperlegible": "lisible", OpenDyslexic: "dyslexie", "Playwrite FR Moderne": "cursive" };

/** Même catalogue que carnet_core::fonts, utilisé si le moteur ne répond pas (aperçu dans un navigateur). */
const FALLBACK: FontFamilyInfo[] = EMBEDDED_FAMILIES.map((name) => ({
  name,
  has_bold: name !== "Playwrite FR Moderne",
  has_italic: name !== "Playwrite FR Moderne",
  user: false,
  embeddable: true,
  kind: KINDS[name],
}));

class Fonts {
  families = $state.raw<FontFamilyInfo[]>(FALLBACK);

  /** Relit la liste du moteur et charge les fichiers des polices ajoutées. */
  async load() {
    try {
      const [families, files] = await Promise.all([listFonts(), fontFiles()]);
      await registerUserFonts(files);
      this.families = families;
    } catch {
      this.families = FALLBACK;
    }
    setKnownFamilies(this.families.map((f) => f.name));
  }
}

export const fonts = new Fonts();

export const FONT_SIZES = [10, 12, 14, 16, 18, 20, 24, 28, 32, 40, 48, 60, 72];

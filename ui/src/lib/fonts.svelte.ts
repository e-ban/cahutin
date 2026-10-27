import { listFonts } from "./api";
import type { FontFamilyInfo } from "./types";

/** Même catalogue que carnet_core::fonts, utilisé si le moteur ne répond pas (aperçu dans un navigateur). */
const FALLBACK: FontFamilyInfo[] = [
  { name: "Andika", has_bold: true, has_italic: true },
  { name: "Atkinson Hyperlegible", has_bold: true, has_italic: true },
  { name: "OpenDyslexic", has_bold: true, has_italic: true },
  { name: "Playwrite FR Moderne", has_bold: false, has_italic: false },
];

class Fonts {
  families = $state.raw<FontFamilyInfo[]>(FALLBACK);

  async load() {
    try {
      this.families = await listFonts();
    } catch {
      this.families = FALLBACK;
    }
  }
}

export const fonts = new Fonts();

export const FONT_SIZES = [10, 12, 14, 16, 18, 20, 24, 28, 32, 40, 48, 60, 72];

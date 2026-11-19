// Réglages de la page affichée, partagés par la barre du bas et le panneau Réglages (sans sélection).
import { pageSetSize } from "./api";
import { editor } from "./docStore.svelte";
import { changeFormat, type Format } from "./pageFormat";

/** Portrait ou paysage : la page tourne, son contenu suit. */
export async function rotatePage() {
  const p = editor.page;
  try {
    const page = await pageSetSize(p, p.size.height_mm, p.size.width_mm);
    editor.updatePageById(p.id, () => page);
  } catch (e) {
    editor.notice = String(e);
  }
}

/** A5, A4 ou A3 : le contenu suit la taille de la page. */
export async function setPageFormat(format: Format) {
  const p = changeFormat(editor.page, format);
  try {
    const page = await pageSetSize(p, p.size.width_mm, p.size.height_mm);
    editor.updatePageById(p.id, () => page);
  } catch (e) {
    editor.notice = String(e);
  }
}

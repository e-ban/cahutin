// Les catégories des Préférences, dans l'ordre de la colonne de gauche. Pour ajouter des réglages : un composant
// dans components/prefs/, puis une ligne ici (son nom, son icône, et les mots qu'on peut chercher).
import type { Component } from "svelte";
import AssistantPrefs from "../components/assistant/AssistantPrefs.svelte";
import AccessibilityPrefs from "../components/prefs/AccessibilityPrefs.svelte";
import ColorBlindPrefs from "../components/prefs/ColorBlindPrefs.svelte";
import DisplayPrefs from "../components/prefs/DisplayPrefs.svelte";
import ExportPrefs from "../components/prefs/ExportPrefs.svelte";
import GraphismePrefs from "../components/prefs/GraphismePrefs.svelte";
import HelpPrefs from "../components/prefs/HelpPrefs.svelte";
import NewPagePrefs from "../components/prefs/NewPagePrefs.svelte";
import SavingPrefs from "../components/prefs/SavingPrefs.svelte";
import ToolbarPrefs from "../components/prefs/ToolbarPrefs.svelte";
import { fr } from "./i18n/fr";
import type { IconName } from "./icons";

export interface PrefsCategory {
  id: string;
  label: string;
  icon: IconName;
  pane: Component;
  /** Les noms des réglages de la catégorie, pour la recherche. */
  words: string[];
  /** Seulement là où le PDF est proposé (export). */
  needsPdf?: boolean;
}

export const PREFS_CATEGORIES: PrefsCategory[] = [
  { id: "display", label: fr.prefsDisplay, icon: "colour", pane: DisplayPrefs, words: [fr.prefsTheme, ...Object.values(fr.themeNames), "sombre", "couleurs", fr.prefsUiScale, "taille", "zoom", fr.prefsUiFont] },
  { id: "accessibility", label: fr.prefsAccessibility, icon: "eye", pane: AccessibilityPrefs, words: [fr.prefsContrast, fr.prefsBigHandles, fr.prefsReduceMotion, "tactile", "souris"] },
  { id: "colorblind", label: fr.prefsColorBlind, icon: "cvd", pane: ColorBlindPrefs, words: [fr.prefsColorNames, fr.prefsDoubleCues, fr.prefsCvdPalette, fr.prefsCvdPreview, "couleurs"] },
  { id: "toolbar", label: fr.prefsToolbar, icon: "more", pane: ToolbarPrefs, words: [fr.prefsToolbarFull, fr.prefsToolbarCompact, fr.prefsMoreMenu, fr.prefsLabelDelay, "barre", "outils", "icônes"] },
  { id: "newPage", label: fr.prefsNewPage, icon: "fileNew", pane: NewPagePrefs, words: [fr.prefsPageFormat, "A4", "A5", "A3", fr.landscape, fr.portrait, fr.prefsTextFont, "police"] },
  { id: "graphisme", label: fr.prefsGraphisme, icon: "pen", pane: GraphismePrefs, words: [fr.graphismeLevel, "PS", "MS", "GS", "niveau"] },
  { id: "saving", label: fr.prefsSaving, icon: "save", pane: SavingPrefs, words: [fr.prefsAutosave, "automatique", "sauvegarde", "Ctrl+S"] },
  { id: "export", label: fr.prefsExport, icon: "print", pane: ExportPrefs, words: [fr.prefsExportPdf, fr.prefsExportOdg, "LibreOffice", "imprimer"], needsPdf: true },
  { id: "help", label: fr.prefsHelp, icon: "help", pane: HelpPrefs, words: [fr.hintsWhileWorking, fr.updatesMenu, "astuces", "mises à jour"] },
  { id: "assistant", label: fr.assistant.prefsTitle, icon: "assistant", pane: AssistantPrefs, words: ["IA", "intelligence artificielle", fr.assistant.prefsService, fr.assistant.prefsKey, "Claude", "OpenAI", "Gemini", "Ollama"] },
];

/** Sans accents ni capitales : « Thème » se trouve en tapant « theme ». */
const plain = (s: string) => s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();

/** Les catégories dont le nom ou un réglage contient le texte cherché ; toutes si rien n'est tapé. */
export function searchPrefs(categories: PrefsCategory[], query: string): PrefsCategory[] {
  const q = plain(query.trim());
  if (!q) return categories;
  return categories.filter((c) => [c.label, ...c.words].some((w) => plain(w).includes(q)));
}

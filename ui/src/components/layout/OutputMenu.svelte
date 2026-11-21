<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { canExportPdf } from "../../lib/api";
  import * as files from "../../lib/fileActions";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { formatOf } from "../../lib/pageFormat";
  import { chapterOf } from "../../lib/chapters";
  import { settings } from "../../lib/settings.svelte";

  /** Quand la barre du haut manque de place : « Imprimer », puis l'icône seule (le menu reste le même). */
  let { size = "full" }: { size?: "full" | "short" | "icon" } = $props();

  let pdf = $state(false);
  canExportPdf().then((v) => (pdf = v)).catch(() => (pdf = false));

  // L'export choisi dans les Préférences vient en premier.
  const exports = $derived.by<MenuItem[]>(() => {
    const odg: MenuItem = { label: fr.exportOdg, icon: "document", title: fr.exportOdgHint, action: files.exportOdg };
    const both: MenuItem[] = pdf ? [{ label: fr.exportPdf, icon: "documentText", action: files.exportPdf }, odg] : [odg];
    return settings.preferredExport === "odg" ? both.reverse() : both;
  });

  // Le chapitre de la page affichée s'imprime ou s'exporte seul (un mois du cahier de vie pour les familles).
  const chapter = $derived(chapterOf(editor.doc.pages, editor.pageIndex));
  const chapterItems = $derived.by<MenuItem[]>(() => {
    if (!chapter) return [];
    const title = chapter.title;
    const items: MenuItem[] = [{ label: fr.printChapter(title), icon: "print", action: () => files.printChapter(editor.pageIndex) }];
    if (pdf) items.push({ label: fr.exportChapterPdf(title), icon: "documentText", action: () => files.exportChapterPdf(editor.pageIndex) });
    return items;
  });

  const items = $derived<MenuItem[]>([
    { label: fr.print, icon: "print", shortcut: "Ctrl+P", action: files.print },
    ...chapterItems,
    ...(editor.doc.pages.length > 1 ? [{ label: fr.printDuplex, icon: "flipH", title: fr.printDuplexHint, action: files.printDuplex } satisfies MenuItem] : []),
    ...(editor.doc.pages.some((p) => formatOf(p.size) === "A3") ? [{ label: fr.printTiles, icon: "grid", title: fr.printTilesHint, action: files.printTiled } satisfies MenuItem] : []),
    ...exports.map((item, i) => (i === 0 ? { ...item, separator: true } : item)),
  ]);
</script>

<MenuButton label={fr.output} shown={size === "full" ? undefined : size === "short" ? fr.print : null} icon="print" {items} primary />

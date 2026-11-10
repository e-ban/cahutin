<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { canExportPdf } from "../../lib/api";
  import * as files from "../../lib/fileActions";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { formatOf } from "../../lib/pageFormat";
  import { settings } from "../../lib/settings.svelte";

  let pdf = $state(false);
  canExportPdf().then((v) => (pdf = v)).catch(() => (pdf = false));

  // L'export choisi dans les Préférences vient en premier.
  const exports = $derived.by<MenuItem[]>(() => {
    const odg: MenuItem = { label: fr.exportOdg, icon: "document", title: fr.exportOdgHint, action: files.exportOdg };
    const both: MenuItem[] = pdf ? [{ label: fr.exportPdf, icon: "documentText", action: files.exportPdf }, odg] : [odg];
    return settings.preferredExport === "odg" ? both.reverse() : both;
  });

  const items = $derived<MenuItem[]>([
    { label: fr.print, icon: "print", shortcut: "Ctrl+P", action: files.print },
    ...(editor.doc.pages.length > 1 ? [{ label: fr.printDuplex, icon: "flipH", title: fr.printDuplexHint, action: files.printDuplex } satisfies MenuItem] : []),
    ...(editor.doc.pages.some((p) => formatOf(p.size) === "A3") ? [{ label: fr.printTiles, icon: "grid", title: fr.printTilesHint, action: files.printTiled } satisfies MenuItem] : []),
    ...exports.map((item, i) => (i === 0 ? { ...item, separator: true } : item)),
  ]);
</script>

<MenuButton label={fr.output} icon="print" {items} primary />

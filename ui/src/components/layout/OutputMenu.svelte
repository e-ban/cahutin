<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { canExportPdf } from "../../lib/api";
  import * as files from "../../lib/fileActions";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { formatOf } from "../../lib/pageFormat";

  let pdf = $state(false);
  canExportPdf().then((v) => (pdf = v)).catch(() => (pdf = false));

  const items = $derived<MenuItem[]>([
    { label: fr.print, shortcut: "Ctrl+P", action: files.print },
    ...(editor.doc.pages.length > 1 ? [{ label: fr.printDuplex, title: fr.printDuplexHint, action: files.printDuplex }] : []),
    ...(editor.doc.pages.some((p) => formatOf(p.size) === "A3") ? [{ label: fr.printTiles, title: fr.printTilesHint, action: files.printTiled }] : []),
    ...(pdf ? [{ label: fr.exportPdf, action: files.exportPdf }] : []),
    { label: fr.exportOdg, title: fr.exportOdgHint, action: files.exportOdg, separator: true },
  ]);
</script>

<MenuButton label={fr.output} {items} primary />

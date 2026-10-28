<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { canExportPdf } from "../../lib/api";
  import * as files from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";

  let pdf = $state(false);
  canExportPdf().then((v) => (pdf = v)).catch(() => (pdf = false));

  const items = $derived<MenuItem[]>([
    { label: fr.print, shortcut: "Ctrl+P", action: files.print },
    ...(pdf ? [{ label: fr.exportPdf, action: files.exportPdf }] : []),
    { label: fr.exportOdg, title: fr.exportOdgHint, action: files.exportOdg, separator: true },
  ]);
</script>

<MenuButton label={fr.output} {items} primary />

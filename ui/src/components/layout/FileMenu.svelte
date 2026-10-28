<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import * as files from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";

  const items: MenuItem[] = [
    { label: fr.newDoc, shortcut: "Ctrl+N", action: files.newDoc },
    { label: fr.openDoc, shortcut: "Ctrl+O", action: files.openDoc },
    { label: fr.save, shortcut: "Ctrl+S", action: files.save, separator: true },
    { label: fr.saveAs, action: files.saveAs },
    { label: fr.reportProblem, action: files.problemReport, separator: true },
  ];
</script>

<MenuButton label={fr.file} {items} />

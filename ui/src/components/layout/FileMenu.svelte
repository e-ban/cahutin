<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import * as files from "../../lib/fileActions";
  import { listExamples } from "../../lib/api";
  import type { ExampleInfo } from "../../lib/types";
  import { fr } from "../../lib/i18n/fr";

  let examples = $state<ExampleInfo[]>([]);
  listExamples().then((l) => (examples = l)).catch(() => (examples = []));

  const items = $derived<MenuItem[]>([
    { label: fr.newDoc, shortcut: "Ctrl+N", action: files.newDoc },
    { label: fr.openDoc, shortcut: "Ctrl+O", action: files.openDoc },
    { label: fr.importFiles, action: () => files.importFiles() },
    { label: fr.save, shortcut: "Ctrl+S", action: files.save, separator: true },
    { label: fr.saveAs, action: files.saveAs },
    ...(editor.recents.length
      ? [
          { label: fr.recentDocs, disabled: true, separator: true, action: () => {} },
          ...editor.recents.map((r) => ({ label: `   ${r.title}`, action: () => files.openPath(r.path) })),
        ]
      : []),
    { label: fr.examplesMenu, disabled: true, separator: true, action: () => {} },
    ...examples.map((ex) => ({ label: `   ${ex.name}`, action: () => files.openExampleDoc(ex.id) })),
    { label: fr.reportProblem, action: files.problemReport, separator: true },
    { label: fr.about, action: () => (editor.showAbout = true) },
  ]);
</script>

<MenuButton label={fr.file} {items} />

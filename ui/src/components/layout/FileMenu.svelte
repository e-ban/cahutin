<script lang="ts">
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import * as files from "../../lib/fileActions";
  import { listExamples } from "../../lib/api";
  import type { ExampleInfo } from "../../lib/types";
  import { fr } from "../../lib/i18n/fr";
  import { localTime } from "../../lib/autosave";
  import { autosave } from "../../lib/autosave.svelte";
  import { settings } from "../../lib/settings.svelte";

  let examples = $state<ExampleInfo[]>([]);
  listExamples().then((l) => (examples = l)).catch(() => (examples = []));

  const items = $derived<MenuItem[]>([
    { label: fr.homeMenu, icon: "home", action: files.goHome },
    { label: fr.newDoc, icon: "fileNew", shortcut: "Ctrl+N", action: files.newDoc, separator: true },
    { label: fr.openDoc, icon: "folderOpen", shortcut: "Ctrl+O", action: files.openDoc },
    { label: fr.importFiles, icon: "fileImport", action: () => files.importFiles() },
    { label: fr.save, icon: "save", shortcut: "Ctrl+S", action: files.save, separator: true },
    { label: fr.saveAs, icon: "save", action: files.saveAs },
    // La version gardée avant la première écriture de la séance (enregistrement automatique coché).
    ...(settings.autosave && editor.ownFile && autosave.startVersion ? [{ label: fr.revertToStart(localTime(autosave.startVersion)), icon: "undo", action: files.revertToStart } satisfies MenuItem] : []),
    ...(editor.recents.length
      ? [
          { label: fr.recentDocs, heading: true, separator: true },
          ...editor.recents.map((r): MenuItem => ({ label: r.title, icon: "documentText", action: () => files.openPath(r.path) })),
        ]
      : []),
    ...(examples.length
      ? [{ label: fr.examplesMenu, icon: "example", separator: true, children: examples.map((ex) => ({ label: ex.name, action: () => files.openExampleDoc(ex.id) })) } satisfies MenuItem]
      : []),
  ]);
</script>

<MenuButton label={fr.file} {items} />

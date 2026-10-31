<script lang="ts">
  // Menu Aide : visite guidée, astuces, réglage des astuces pendant le travail, avis de nouvelles versions,
  // rapport, à propos.
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import * as files from "../../lib/fileActions";
  import { hints } from "../../lib/hints.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { tour } from "../../lib/tour.svelte";
  import { updates } from "../../lib/updates.svelte";

  const items = $derived<MenuItem[]>([
    { label: fr.tourMenu, action: () => tour.start() },
    { label: fr.tips, action: () => (editor.showTips = true) },
    { label: fr.whatsNewMenu, action: () => (editor.showWhatsNew = true) },
    { label: `${hints.enabled ? "✓" : "   "} ${fr.hintsWhileWorking}`, action: () => hints.toggle() },
    { label: `${updates.consent === true ? "✓" : "   "} ${fr.updatesMenu}`, action: () => updates.setConsent(updates.consent !== true), separator: true },
    { label: fr.reportProblem, action: files.problemReport, separator: true },
    { label: fr.about, action: () => (editor.showAbout = true) },
  ]);
</script>

<MenuButton label={fr.help} {items} />

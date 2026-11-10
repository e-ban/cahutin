<script lang="ts">
  // Menu Aide : visite guidée, astuces, guides, réglage des astuces pendant le travail, avis de nouvelles versions,
  // rapport, à propos.
  import MenuButton, { type MenuItem } from "./MenuButton.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import * as files from "../../lib/fileActions";
  import { guides } from "../../lib/guides.svelte";
  import { hints } from "../../lib/hints.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { tour } from "../../lib/tour.svelte";
  import { updates } from "../../lib/updates.svelte";

  const items = $derived<MenuItem[]>([
    { label: fr.tourMenu, icon: "play", action: () => tour.start() },
    { label: fr.tips, icon: "lightbulb", action: () => (editor.showTips = true) },
    { label: fr.guides, icon: "guide", action: () => guides.show() },
    { label: fr.whatsNewMenu, icon: "star", action: () => (editor.showWhatsNew = true) },
    { label: fr.hintsWhileWorking, checked: hints.enabled, action: () => hints.toggle() },
    ...(updates.store ? [] : [{ label: fr.updatesMenu, checked: updates.consent === true, action: () => updates.setConsent(updates.consent !== true), separator: true }]),
    { label: fr.reportProblem, icon: "flag", action: files.problemReport, separator: true },
    { label: fr.about, icon: "info", action: () => (editor.showAbout = true) },
  ]);
</script>

<MenuButton label={fr.help} {items} />

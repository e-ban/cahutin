<script lang="ts">
  // Fiches de graphisme : le niveau de la classe.
  import { fr } from "../../lib/i18n/fr";
  import type { Level } from "../../lib/graphisme";
  import { settings } from "../../lib/settings.svelte";
</script>

<div class="row">
  <span><b>{fr.graphismeLevel}</b><small>{fr.prefsGraphismeHint}</small></span>
  <select value={settings.graphismeLevel} aria-label={fr.graphismeLevel} onchange={(e) => settings.set("graphismeLevel", e.currentTarget.value as Level)}>
    <option value="PS">PS</option>
    <option value="MS">MS</option>
    <option value="GS">GS</option>
  </select>
</div>

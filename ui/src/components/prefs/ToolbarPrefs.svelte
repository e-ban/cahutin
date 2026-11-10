<script lang="ts">
  // Barre au-dessus de l'élément choisi : noms écrits ou icônes seules, menu « Plus ».
  import { fr } from "../../lib/i18n/fr";
  import { LABEL_DELAYS, settings, type LabelDelay } from "../../lib/settings.svelte";
</script>

<label class="choice">
  <input type="radio" name="toolbar" checked={!settings.compactToolbar} onchange={() => settings.setCompactToolbar(false)} />
  <span><b>{fr.prefsToolbarFull}</b><small>{fr.prefsToolbarFullHint}</small></span>
</label>
<label class="choice">
  <input type="radio" name="toolbar" checked={settings.compactToolbar} onchange={() => settings.setCompactToolbar(true)} />
  <span><b>{fr.prefsToolbarCompact}</b><small>{fr.prefsToolbarCompactHint}</small></span>
</label>
{#if settings.compactToolbar}
  <div class="row sub">
    <span><b>{fr.prefsLabelDelay}</b><small>{fr.prefsLabelDelayHint}</small></span>
    <select value={settings.labelDelay} aria-label={fr.prefsLabelDelay} onchange={(e) => settings.set("labelDelay", Number(e.currentTarget.value) as LabelDelay)}>
      {#each LABEL_DELAYS as d (d)}<option value={d}>{fr.prefsLabelDelays[d]}</option>{/each}
    </select>
  </div>
{/if}
<label class="choice">
  <input type="checkbox" checked={settings.moreMenu} onchange={(e) => settings.setMoreMenu(e.currentTarget.checked)} />
  <span><b>{fr.prefsMoreMenu}</b><small>{fr.prefsMoreMenuHint}</small></span>
</label>

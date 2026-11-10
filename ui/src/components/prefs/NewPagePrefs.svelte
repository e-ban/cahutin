<script lang="ts">
  // Nouvelle page : format, orientation et police des textes.
  import { fr } from "../../lib/i18n/fr";
  import { EMBEDDED_FAMILIES } from "../../lib/fontNames";
  import type { Format } from "../../lib/pageFormat";
  import { settings } from "../../lib/settings.svelte";
</script>

<div class="row">
  <span><b>{fr.prefsPageFormat}</b><small>{fr.prefsNewPageHint}</small></span>
  <span class="controls">
    <select value={settings.pageFormat} aria-label={fr.prefsPageFormat} onchange={(e) => settings.set("pageFormat", e.currentTarget.value as Format)}>
      <option value="A5">A5</option>
      <option value="A4">A4</option>
      <option value="A3">A3</option>
    </select>
    <select value={settings.pageLandscape ? "l" : "p"} aria-label={fr.landscape} onchange={(e) => settings.set("pageLandscape", e.currentTarget.value === "l")}>
      <option value="p">{fr.portrait}</option>
      <option value="l">{fr.landscape}</option>
    </select>
  </span>
</div>
<div class="row">
  <span><b>{fr.prefsTextFont}</b><small>{fr.prefsTextFontHint}</small></span>
  <select value={settings.textFont} style:font-family={`"${settings.textFont}"`} onchange={(e) => settings.set("textFont", e.currentTarget.value)}>
    {#each EMBEDDED_FAMILIES as font (font)}<option value={font} style:font-family={`"${font}"`}>{font}</option>{/each}
  </select>
</div>

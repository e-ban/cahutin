<script lang="ts">
  // Daltonisme : noms des couleurs, repères doublés, couleurs distinctes, aperçu de la page.
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
</script>

<label class="choice">
  <input type="checkbox" checked={settings.colorNames} onchange={(e) => settings.set("colorNames", e.currentTarget.checked)} />
  <span><b>{fr.prefsColorNames}</b><small>{fr.prefsColorNamesHint}</small></span>
</label>
<label class="choice">
  <input type="checkbox" checked={settings.doubleCues} onchange={(e) => settings.set("doubleCues", e.currentTarget.checked)} />
  <span><b>{fr.prefsDoubleCues}</b><small>{fr.prefsDoubleCuesHint}</small></span>
</label>
<label class="choice">
  <input type="checkbox" checked={settings.cvdPalette} onchange={(e) => settings.set("cvdPalette", e.currentTarget.checked)} />
  <span><b>{fr.prefsCvdPalette}</b><small>{fr.prefsCvdPaletteHint}</small></span>
</label>
<label class="choice">
  <input type="checkbox" checked={settings.cvdPreview} onchange={(e) => settings.set("cvdPreview", e.currentTarget.checked)} />
  <span><b>{fr.prefsCvdPreview}</b><small>{fr.prefsCvdPreviewHint}</small></span>
</label>

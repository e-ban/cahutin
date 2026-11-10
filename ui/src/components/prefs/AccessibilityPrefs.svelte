<script lang="ts">
  // Accessibilité : contraste élevé, grandes poignées, animations réduites.
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
</script>

<label class="choice">
  <input type="checkbox" checked={settings.highContrast} onchange={(e) => settings.set("highContrast", e.currentTarget.checked)} />
  <span><b>{fr.prefsContrast}</b><small>{fr.prefsContrastHint}</small></span>
</label>
<label class="choice">
  <input type="checkbox" checked={settings.bigHandles} onchange={(e) => settings.set("bigHandles", e.currentTarget.checked)} />
  <span><b>{fr.prefsBigHandles}</b><small>{fr.prefsBigHandlesHint}</small></span>
</label>
<label class="choice">
  <input type="checkbox" checked={settings.reduceMotion} onchange={(e) => settings.set("reduceMotion", e.currentTarget.checked)} />
  <span><b>{fr.prefsReduceMotion}</b><small>{fr.prefsReduceMotionHint}</small></span>
</label>

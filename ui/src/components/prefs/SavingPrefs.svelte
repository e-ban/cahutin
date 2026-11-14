<script lang="ts">
  // Enregistrement : le document qui a un fichier s'y enregistre tout seul, ou à la main comme avant.
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
</script>

<label class="choice">
  <input type="checkbox" checked={settings.autosave} onchange={(e) => settings.set("autosave", e.currentTarget.checked)} />
  <span><b>{fr.prefsAutosave}</b><small>{fr.prefsAutosaveHint}</small></span>
</label>

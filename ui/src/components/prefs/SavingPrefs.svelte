<script lang="ts">
  // Enregistrement : le document qui a un fichier s'y enregistre tout seul, ou à la main comme avant.
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
</script>

<label class="choice">
  <input type="checkbox" checked={settings.autosave} onchange={(e) => settings.set("autosave", e.currentTarget.checked)} />
  <span><b>{fr.prefsAutosave}</b><small>{fr.prefsAutosaveShort}</small></span>
</label>
<!-- Une phrase suffit pour choisir ; le détail se lit sur demande. -->
<details class="more">
  <summary>{fr.prefsMore}</summary>
  <p>{fr.prefsAutosaveHint}</p>
</details>

<style>
  .more {
    margin: 4px 0 0 28px;
    font-size: 12px;
    color: var(--muted);
  }
  .more summary {
    cursor: pointer;
    color: var(--accent);
    width: fit-content;
  }
  .more p {
    margin: 6px 0 0;
    max-width: 60ch;
    line-height: 1.45;
  }
</style>

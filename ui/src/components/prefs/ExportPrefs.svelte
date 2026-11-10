<script lang="ts">
  // Exporter : le format proposé d'abord, PDF ou LibreOffice.
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
</script>

  <label class="choice">
    <input type="radio" name="export" checked={settings.preferredExport === "pdf"} onchange={() => settings.set("preferredExport", "pdf")} />
    <span><b>{fr.prefsExportPdf}</b><small>{fr.prefsExportHint}</small></span>
  </label>
  <label class="choice">
    <input type="radio" name="export" checked={settings.preferredExport === "odg"} onchange={() => settings.set("preferredExport", "odg")} />
    <span><b>{fr.prefsExportOdg}</b></span>
  </label>

<script lang="ts">
  // Aide : astuces pendant le travail, avis de nouvelles versions (hors Microsoft Store).
  import { fr } from "../../lib/i18n/fr";
  import { hints } from "../../lib/hints.svelte";
  import { updates } from "../../lib/updates.svelte";
</script>

<label class="choice">
  <input type="checkbox" checked={hints.enabled} onchange={() => hints.toggle()} />
  <span><b>{fr.hintsWhileWorking}</b></span>
</label>
{#if !updates.store}
  <label class="choice">
    <input type="checkbox" checked={updates.consent === true} onchange={(e) => updates.setConsent(e.currentTarget.checked)} />
    <span><b>{fr.updatesMenu}</b></span>
  </label>
{/if}

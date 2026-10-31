<script lang="ts">
  // Bandeau discret, en bas à droite : la question des nouvelles versions (une seule fois), puis l'avis
  // quand une version plus récente existe. Il n'interrompt jamais le travail.
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { tour } from "../../lib/tour.svelte";
  import { bannerVersion, shouldAskConsent, updates } from "../../lib/updates.svelte";
  import { openDownloadPage } from "../../lib/api";

  // Version du Microsoft Store : le Store met à jour, ni question ni avis.
  const ask = $derived(!updates.store && shouldAskConsent(updates.consent, { tourDone: tour.done, tourRunning: tour.step !== null, welcomeOpen: editor.showWelcome }));
  const version = $derived(updates.store ? null : bannerVersion(updates.consent, updates.state, updates.appVersion));

  function download() {
    openDownloadPage().catch((e) => (editor.notice = String(e)));
  }
</script>

{#if ask}
  <div class="banner" role="status">
    <p>{fr.updateConsent}</p>
    <div class="actions">
      <button onclick={() => updates.setConsent(false)}>{fr.updateConsentNo}</button>
      <button class="primary" onclick={() => updates.setConsent(true)}>{fr.updateConsentYes}</button>
    </div>
  </div>
{:else if version}
  <div class="banner" role="status">
    <p>{fr.updateAvailable(version)}</p>
    <div class="actions">
      <button class="primary" onclick={download}>{fr.updateDownload}</button>
      <button aria-label={fr.close} title={fr.close} onclick={() => updates.dismiss(version)}>✕</button>
    </div>
  </div>
{/if}

<style>
  .banner {
    position: fixed;
    right: 16px;
    bottom: 70px;
    max-width: 360px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 10px 12px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.15);
    z-index: 40;
  }
  p {
    margin: 0 0 8px;
  }
  .actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
</style>

<script lang="ts">
  // Préférences de l'application : affichage de la barre d'outils, astuces, avis de nouvelles versions. Chaque
  // choix s'applique tout de suite et reste sur le poste.
  import { editor } from "../../lib/docStore.svelte";
  import { hints } from "../../lib/hints.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
  import { updates } from "../../lib/updates.svelte";

  const close = () => (editor.showPreferences = false);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="backdrop" role="presentation" onclick={close}>
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="prefs-title" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <h2 id="prefs-title">⚙ {fr.preferences}</h2>

    <section>
      <h3>{fr.prefsToolbar}</h3>
      <label class="choice">
        <input type="radio" name="toolbar" checked={!settings.compactToolbar} onchange={() => settings.setCompactToolbar(false)} />
        <span><b>{fr.prefsToolbarFull}</b><small>{fr.prefsToolbarFullHint}</small></span>
      </label>
      <label class="choice">
        <input type="radio" name="toolbar" checked={settings.compactToolbar} onchange={() => settings.setCompactToolbar(true)} />
        <span><b>{fr.prefsToolbarCompact}</b><small>{fr.prefsToolbarCompactHint}</small></span>
      </label>
      <label class="choice">
        <input type="checkbox" checked={settings.moreMenu} onchange={(e) => settings.setMoreMenu(e.currentTarget.checked)} />
        <span><b>{fr.prefsMoreMenu}</b><small>{fr.prefsMoreMenuHint}</small></span>
      </label>
    </section>

    <section>
      <h3>{fr.prefsHelp}</h3>
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
    </section>

    <div class="buttons"><button class="primary" onclick={close}>{fr.close}</button></div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    width: min(520px, 92vw);
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
  }
  h2 {
    font: 700 22px var(--title-font);
    color: var(--accent-strong);
    margin: 0 0 8px;
  }
  h3 {
    margin: 14px 0 6px;
    font-size: 13px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .choice {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 6px 0;
  }
  .choice input {
    margin-top: 3px;
  }
  .choice span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .choice b {
    font-weight: 500;
  }
  .choice small {
    color: var(--muted);
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    padding: 7px 18px;
  }
</style>

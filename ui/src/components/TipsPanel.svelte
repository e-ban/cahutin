<script lang="ts">
  // Panneau « Astuces » (menu Aide) : toutes les astuces, rangées par thème. Échap ou Fermer le ferme.
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { TIP_THEMES, TIPS } from "../lib/tips";

  const close = () => (editor.showTips = false);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && close()}>
  <div class="panel" role="dialog" aria-modal="true" aria-labelledby="tips-title">
    <header>
      <h2 id="tips-title">💡 {fr.tips}</h2>
      <button onclick={close}>{fr.close}</button>
    </header>
    <div class="body">
      {#each TIP_THEMES as theme (theme)}
        <section>
          <h3>{theme}</h3>
          {#each TIPS.filter((t) => t.theme === theme) as tip (tip.id)}
            <div class="tip">
              <span class="icon" aria-hidden="true">{tip.icon}</span>
              <div>
                <b>{tip.title}</b>
                <p>{tip.text}</p>
              </div>
            </div>
          {/each}
        </section>
      {/each}
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: grid;
    place-items: center;
    z-index: 4500; /* au-dessus de la barre flottante et des menus, sous les boîtes de dialogue */
  }
  .panel {
    width: min(720px, calc(100vw - 32px));
    max-height: calc(100vh - 64px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-radius: 14px;
    box-shadow: 0 12px 36px rgba(0, 0, 0, 0.3);
    font-family: var(--ui-font);
    color: var(--text);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    margin: 0;
    font-size: 19px;
  }
  header button {
    font: inherit;
    padding: 6px 12px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: #fff;
    cursor: pointer;
  }
  .body {
    overflow-y: auto;
    padding: 6px 20px 18px;
  }
  h3 {
    margin: 16px 0 8px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .tip {
    display: flex;
    gap: 12px;
    padding: 8px 0;
  }
  .icon {
    font-size: 22px;
    width: 30px;
    text-align: center;
    flex: none;
  }
  .tip p {
    margin: 2px 0 0;
    font-size: 14px;
    line-height: 1.4;
  }
</style>

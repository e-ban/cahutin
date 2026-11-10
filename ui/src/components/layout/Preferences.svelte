<script lang="ts">
  // Préférences de l'application : les catégories à gauche (avec une recherche), les réglages de celle choisie à
  // droite. Chaque choix s'applique tout de suite et reste sur le poste. Les catégories et leurs réglages sont dans
  // lib/prefsCategories.ts et components/prefs/.
  import Icon from "../Icon.svelte";
  import { canExportPdf } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { PREFS_CATEGORIES, searchPrefs } from "../../lib/prefsCategories";

  const close = () => (editor.showPreferences = false);

  // Le choix de l'export n'a de sens que là où le PDF est proposé.
  let pdf = $state(false);
  canExportPdf().then((v) => (pdf = v)).catch(() => (pdf = false));

  let query = $state("");
  const available = $derived(PREFS_CATEGORIES.filter((c) => !c.needsPdf || pdf));
  const shown = $derived(searchPrefs(available, query));
  // La catégorie ouverte : celle choisie, ou la première trouvée par la recherche.
  const current = $derived(shown.find((c) => c.id === editor.prefsCategory) ?? shown[0] ?? null);
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="backdrop" role="presentation" onclick={close}>
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="prefs-title" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <header>
      <h2 id="prefs-title"><Icon name="settings" size={20} /> {fr.preferences}</h2>
      <button class="close" title={fr.close} aria-label={fr.close} onclick={close}><Icon name="close" size={18} /></button>
    </header>
    <div class="body">
      <nav aria-label={fr.prefsCategories}>
        <input class="search" type="search" bind:value={query} placeholder={fr.prefsSearch} aria-label={fr.prefsSearch} />
        <div class="cats" role="tablist" aria-orientation="vertical">
          {#each shown as c (c.id)}
            <button role="tab" class="cat" class:on={current?.id === c.id} aria-selected={current?.id === c.id} onclick={() => (editor.prefsCategory = c.id)}>
              <Icon name={c.icon} size={18} /><span>{c.label}</span>
            </button>
          {/each}
          {#if shown.length === 0}<p class="none">{fr.prefsSearchNone}</p>{/if}
        </div>
      </nav>
      <div class="pane" role="tabpanel" tabindex="0" aria-label={current?.label}>
        {#if current}
          <h3>{current.label}</h3>
          <current.pane />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  /* Une hauteur fixe : la fenêtre ne saute pas d'une catégorie à l'autre ; seul le panneau défile. */
  .box {
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-radius: 12px;
    width: min(1100px, 96vw);
    height: min(680px, 92vh);
    box-sizing: border-box;
    box-shadow: 0 10px 30px var(--shadow);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 20px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 8px;
    font: 700 22px var(--title-font);
    color: var(--accent-strong);
    margin: 0;
  }
  .close {
    display: grid;
    place-items: center;
    padding: 6px;
    border-color: transparent;
    background: transparent;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 250px 1fr;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 12px;
    border-right: 1px solid var(--line);
    overflow-y: auto;
  }
  .search {
    width: 100%;
    box-sizing: border-box;
  }
  .cats {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .cat {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-color: transparent;
    background: transparent;
    text-align: left;
  }
  .cat.on {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .none {
    margin: 4px 6px;
    color: var(--muted);
    font-size: 13px;
  }
  .pane {
    overflow-y: auto;
    padding: 16px 28px 24px;
  }
  h3 {
    margin: 0 0 10px;
    font-size: 18px;
    font-weight: 600;
    text-transform: none;
    letter-spacing: 0;
    color: var(--text);
  }
  /* Petite fenêtre : les catégories passent en une rangée au-dessus des réglages. */
  @media (max-width: 720px) {
    .body {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }
    nav {
      border-right: 0;
      border-bottom: 1px solid var(--line);
    }
    .cats {
      flex-direction: row;
      overflow-x: auto;
    }
    .cat {
      flex: none;
    }
  }
  /* Les réglages de chaque catégorie (components/prefs) partagent ces formes. */
  .pane :global(.choice) {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 6px 0;
  }
  .pane :global(.choice input) {
    margin-top: 3px;
  }
  .pane :global(.choice span) {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pane :global(.row) {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 6px 0;
  }
  .pane :global(.row > span:first-child) {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .pane :global(.row b) {
    font-weight: 500;
  }
  .pane :global(.row small) {
    color: var(--muted);
  }
  .pane :global(.row.sub) {
    padding-left: 26px;
  }
  .pane :global(.controls) {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .pane :global(.row select) {
    flex-shrink: 0;
    max-width: 260px;
  }
  .pane :global(.seg) {
    display: inline-flex;
    flex-shrink: 0;
  }
  .pane :global(.seg button) {
    border-radius: 0;
    margin-left: -1px;
  }
  .pane :global(.seg button:first-child) {
    border-radius: 6px 0 0 6px;
  }
  .pane :global(.seg button:last-child) {
    border-radius: 0 6px 6px 0;
  }
  .pane :global(.seg button.on) {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .pane :global(.choice b) {
    font-weight: 500;
  }
  .pane :global(.choice small) {
    color: var(--muted);
  }
</style>

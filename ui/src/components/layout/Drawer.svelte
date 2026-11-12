<script lang="ts">
  import type { Snippet } from "svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import Icon from "../Icon.svelte";

  // `wide` : le tiroir s'étend par-dessus la page sans la déplacer (sa colonne garde 260 px).
  let { title, children, wide = false }: { title: string; children: Snippet; wide?: boolean } = $props();
</script>

<aside class="drawer" class:wide aria-label={title}>
  <header>
    <h3>{title}</h3>
    <button class="close" title={fr.closeDrawer} aria-label={fr.closeDrawer} onclick={() => (editor.drawer = null)}><Icon name="close" size={18} /></button>
  </header>
  <div class="body">{@render children()}</div>
</aside>

<style>
  .drawer {
    grid-area: drawer;
    width: 260px;
    background: var(--panel);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
    --wide-w: min(1240px, calc(100vw - 140px));
    transition: width 0.15s ease, margin-right 0.15s ease;
  }
  /* Plus large que sa colonne : la marge négative rend la différence, la page ne bouge pas. */
  /* Au-dessus des barres flottantes de la page (3500), sous les menus et les boîtes de dialogue (4000 et plus). */
  .drawer.wide {
    position: relative;
    z-index: 3600;
    width: var(--wide-w);
    margin-right: calc(260px - var(--wide-w));
    box-shadow: 6px 0 18px rgb(0 0 0 / 0.18);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px 0 12px;
  }
  header h3 {
    margin: 0;
  }
  .close {
    border: 0;
    background: transparent;
    display: flex;
    padding: 4px;
  }
  .body {
    overflow-y: auto;
    padding: 8px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
  /* Le tiroir défile : ses blocs gardent leur hauteur (le WebKit écrasait une grille de boutons sous le texte
     qui la suit quand la fenêtre est haute). */
  .body > :global(*) {
    flex-shrink: 0;
  }
</style>

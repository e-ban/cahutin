<script lang="ts">
  import type { Snippet } from "svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import Icon from "../Icon.svelte";

  // `wide` : le tiroir s'étend par-dessus la page sans la déplacer (sa colonne garde 260 px).
  // `edge` : un bord droit cliquable qui agrandit le tiroir, ou le réduit quand il est agrandi.
  let {
    title,
    children,
    wide = false,
    edge,
  }: { title: string; children: Snippet; wide?: boolean; edge?: { label: string; ontoggle: () => void } } = $props();
</script>

<aside class="drawer" class:wide class:has-edge={!!edge} aria-label={title}>
  {#if edge}
    <button class="edge" title={edge.label} aria-label={edge.label} onclick={edge.ontoggle}>
      <span>{wide ? "‹" : "›"}</span>
    </button>
  {/if}
  <header>
    <h3>{title}</h3>
    <button class="close" title={fr.closeDrawer} aria-label={fr.closeDrawer} onclick={() => (editor.drawer = null)}><Icon name="close" size={18} /></button>
  </header>
  <div class="body">{@render children()}</div>
</aside>

<style>
  .drawer {
    grid-area: drawer;
    --base-w: 260px;
    width: var(--base-w);
    box-sizing: border-box;
    background: var(--panel);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
    --wide-w: min(1240px, calc(100vw - 140px));
    transition: width 0.15s ease, margin-right 0.15s ease;
    position: relative;
  }
  /* Avec le bord cliquable : un peu plus large, et une marge à droite pour que rien (barre de défilement
     comprise) ne le touche. Au-dessus de la page, pour que le bord puisse s'élargir vers elle. */
  .drawer.has-edge {
    --base-w: 288px;
    padding-right: 26px;
    border-right: 0;
    z-index: 5;
  }
  /* Le bord droit, plus épais que celui des autres tiroirs ; au survol, il s'allume et s'élargit vers la droite. */
  .edge {
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    width: 16px;
    padding: 0;
    border: 0;
    border-left: 1px solid var(--line);
    border-radius: 0;
    background: var(--surface);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: width 0.12s ease, right 0.12s ease, background 0.12s ease;
  }
  .edge span {
    font-size: 15px;
    line-height: 1;
    color: var(--muted);
    font-weight: 700;
  }
  .edge:hover,
  .edge:focus-visible {
    right: -10px;
    width: 26px;
    background: var(--accent);
    border-left-color: var(--accent);
    border-radius: 0 6px 6px 0;
  }
  .edge:hover span,
  .edge:focus-visible span {
    color: white;
  }
  /* Plus large que sa colonne : la marge négative rend la différence, la page ne bouge pas. */
  /* Au-dessus des barres flottantes de la page (3500), sous les menus et les boîtes de dialogue (4000 et plus). */
  .drawer.wide {
    position: relative;
    z-index: 3600;
    width: var(--wide-w);
    margin-right: calc(var(--base-w) - var(--wide-w));
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

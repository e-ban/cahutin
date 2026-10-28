<script lang="ts">
  import type { Snippet } from "svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  let { title, children }: { title: string; children: Snippet } = $props();
</script>

<aside class="drawer" aria-label={title}>
  <header>
    <h3>{title}</h3>
    <button class="close" title={fr.closeDrawer} aria-label={fr.closeDrawer} onclick={() => (editor.drawer = null)}>×</button>
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
    font-size: 18px;
    padding: 0 6px;
  }
  .body {
    overflow-y: auto;
    padding: 8px 12px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }
</style>

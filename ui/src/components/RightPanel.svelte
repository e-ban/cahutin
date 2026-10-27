<script lang="ts">
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";

  const selected = $derived(editor.selected);
</script>

<aside class="right">
  {#if !selected}
    <p class="hint">{fr.noSelection}</p>
  {:else}
    <h3>{selected.kind.type === "Photo" ? fr.photo : selected.kind.type === "Text" ? fr.text : fr.shape}</h3>
  {/if}
</aside>

<style>
  .right {
    grid-area: right;
    background: var(--panel);
    border-left: 1px solid var(--line);
    overflow-y: auto;
    padding: 12px;
  }
  .hint {
    color: var(--muted);
  }
</style>

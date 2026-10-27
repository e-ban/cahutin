<script lang="ts">
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
</script>

<footer class="pagebar">
  <span class="label">{fr.pages}</span>
  {#each editor.doc.pages as p, i (p.id)}
    <button class="thumb" class:current={i === editor.pageIndex} onclick={() => (editor.pageIndex = i)}>{i + 1}</button>
  {/each}
</footer>

<style>
  .pagebar {
    grid-area: bottom;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    background: var(--panel);
    border-top: 1px solid var(--line);
  }
  .thumb {
    width: 34px;
    height: 46px;
    background: white;
  }
  .thumb.current {
    outline: 2px solid var(--accent);
  }
  .label {
    color: var(--muted);
  }
</style>

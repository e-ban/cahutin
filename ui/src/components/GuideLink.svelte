<script lang="ts">
  // « ? Comment faire » : ouvre le guide d'une fonction, là où elle se trouve.
  import { guides } from "../lib/guides.svelte";
  import { fr } from "../lib/i18n/fr";

  let { id }: { id: string } = $props();
</script>

<button class="guide-link" onclick={() => guides.show(id)}>? {fr.guideHelp}</button>

<style>
  .guide-link {
    border: none;
    background: none;
    color: var(--accent);
    padding: 2px 4px;
    font-size: 13px;
    white-space: nowrap;
  }
  .guide-link:hover {
    text-decoration: underline;
  }
</style>

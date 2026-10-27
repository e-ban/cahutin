<script lang="ts">
  // Toutes les pages à taille réelle (zoom 1 : 1 mm CSS = 1 mm), visibles seulement à l'impression.
  import PageView from "./PageView.svelte";
  import { editor } from "../lib/docStore.svelte";
</script>

<div class="print-root">
  {#each editor.doc.pages as p (p.id)}
    <div class="print-page" class:landscape={p.size.width_mm > p.size.height_mm}>
      <PageView page={p} doc={editor.doc} zoom={1} />
    </div>
  {/each}
</div>

<script lang="ts">
  // Toutes les pages à taille réelle (zoom 1 : 1 mm CSS = 1 mm), visibles seulement à l'impression. Chaque
  // page prend la feuille de son format (print.css) ; en tuiles, une page A3 sort en deux feuilles A4.
  import PageView from "./PageView.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { a3Tiles, formatOf, printPageName } from "../lib/pageFormat";
</script>

<div class="print-root">
  {#each editor.doc.pages as p (p.id)}
    {#if editor.printTiles && formatOf(p.size) === "A3"}
      {#each a3Tiles(p.size) as t, i (i)}
        <div class="print-page tile" style:page={printPageName(t.sheet)} style:width="{t.sheet.width_mm}mm" style:height="{t.sheet.height_mm}mm">
          <div class="tile-content" style:transform="translate({t.dx_mm}mm, {t.dy_mm}mm) scale({t.scale})">
            <PageView page={p} doc={editor.doc} zoom={1} />
          </div>
        </div>
      {/each}
    {:else}
      <div class="print-page" style:page={printPageName(p.size)}>
        <PageView page={p} doc={editor.doc} zoom={1} />
      </div>
    {/if}
  {/each}
</div>

<style>
  .tile {
    position: relative;
    overflow: hidden;
  }
  .tile-content {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
  }
</style>

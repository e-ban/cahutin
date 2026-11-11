<script lang="ts">
  // Les motifs vides des lignes de graphisme sélectionnées, en gris très clair : à l'écran seulement (l'impression
  // et les exports ne passent pas par là) et sans prendre les clics.
  import ShapeEl from "./elements/ShapeEl.svelte";
  import { ghostsFor } from "../lib/graphismeEdit";
  import { mmToPx } from "../lib/units";
  import type { Page, ShapeProps } from "../lib/types";

  let { page, selection, zoom }: { page: Page; selection: string[]; zoom: number } = $props();

  const ghosts = $derived(ghostsFor(page, selection));
  const look = (path: string): ShapeProps => ({
    shape: "Path",
    fill: null,
    stroke: { width_mm: 0.6, color: "#e6e6e6", style: "Dotted" },
    corner_radius_mm: 0,
    path,
  });
</script>

{#each ghosts as g, i (i)}
  <div
    class="ghost"
    aria-hidden="true"
    style:left="{mmToPx(g.frame.x_mm, zoom)}px"
    style:top="{mmToPx(g.frame.y_mm, zoom)}px"
    style:width="{mmToPx(g.frame.w_mm, zoom)}px"
    style:height="{mmToPx(g.frame.h_mm, zoom)}px"
  >
    <ShapeEl props={look(g.path)} frame={g.frame} {zoom} />
  </div>
{/each}

<style>
  .ghost {
    position: absolute;
    pointer-events: none;
    z-index: 2;
  }
</style>

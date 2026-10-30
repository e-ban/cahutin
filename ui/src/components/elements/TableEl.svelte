<script lang="ts">
  // Traits d'un tableau, en SVG, avec les segments de l'export. Les cases sont des éléments à part ;
  // le tableau est au-dessus d'elles et laisse passer les clics.
  import { dashArray } from "../../lib/strokes";
  import { borderSegments } from "../../lib/table";
  import { mmToPx } from "../../lib/units";
  import type { Frame, TableProps } from "../../lib/types";

  let { props, frame, zoom }: { props: TableProps; frame: Frame; zoom: number } = $props();

  const strokePx = $derived(props.border ? mmToPx(props.border.width_mm, zoom) : 0);
  const dash = $derived(dashArray(props.border, strokePx));
  const segments = $derived(props.border ? borderSegments(props, { ...frame, x_mm: 0, y_mm: 0 }).map((s) => s.map((v) => mmToPx(v, zoom))) : []);
</script>

<svg class="table" aria-hidden="true">
  {#each segments as [x1, y1, x2, y2], i (i)}
    <line {x1} {y1} {x2} {y2} stroke={props.border?.color} stroke-width={strokePx} stroke-dasharray={dash} stroke-linecap="square" />
  {/each}
</svg>

<style>
  .table {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
  }
</style>

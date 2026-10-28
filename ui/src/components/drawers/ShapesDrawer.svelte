<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { centeredFrame, newId } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";
  import type { ShapeKind } from "../../lib/types";

  const shapes: { shape: ShapeKind; radius: number; icon: string; label: string }[] = [
    { shape: "Rect", radius: 0, icon: "▭", label: fr.rect },
    { shape: "Rect", radius: 6, icon: "▢", label: fr.roundedRect },
    { shape: "Ellipse", radius: 0, icon: "◯", label: fr.ellipse },
  ];

  function add(shape: ShapeKind, corner_radius_mm: number) {
    const id = newId();
    const frame = centeredFrame(editor.page, 60, 40);
    editor.updatePage((p) => ({
      ...p,
      elements: [...p.elements, { id, frame, locked: false, zone: false, kind: { type: "Shape", shape, fill: "#f6e3c5", stroke: null, corner_radius_mm } }],
    }));
    editor.selection = [id];
  }
</script>

<Drawer title={fr.shapes}>
  <div class="grid">
    {#each shapes as s (s.label)}
      <button onclick={() => add(s.shape, s.radius)}><b>{s.icon}</b>{s.label}</button>
    {/each}
  </div>
</Drawer>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .grid button {
    padding: 10px 4px;
    font-size: 12px;
  }
  .grid b {
    display: block;
    font-size: 28px;
    font-weight: 400;
    color: #c98f3d;
  }
</style>

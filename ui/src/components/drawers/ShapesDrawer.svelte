<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import ShapeEl from "../elements/ShapeEl.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { shapeElement } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";
  import type { Element, ShapeKind } from "../../lib/types";

  const shapes: { shape: ShapeKind; radius: number; label: string }[] = [
    { shape: "Rect", radius: 0, label: fr.rect },
    { shape: "Rect", radius: 6, label: fr.roundedRect },
    { shape: "Ellipse", radius: 0, label: fr.ellipse },
    { shape: "Line", radius: 0, label: fr.line },
    { shape: "Arrow", radius: 0, label: fr.arrow },
    { shape: "Star", radius: 0, label: fr.star },
    { shape: "Heart", radius: 0, label: fr.heart },
    { shape: "Bubble", radius: 0, label: fr.bubble },
    { shape: "Cloud", radius: 0, label: fr.cloud },
  ];

  // Aperçu dessiné par le même composant que la page.
  const previews: Element[] = shapes.map((s) => shapeElement(editor.page, s.shape, s.radius));

  function add(shape: ShapeKind, radius: number) {
    const el = shapeElement(editor.page, shape, radius);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
</script>

<Drawer title={fr.shapes}>
  <div class="grid">
    {#each shapes as s, i (s.label)}
      {@const el = previews[i]}
      <button onclick={() => add(s.shape, s.radius)}>
        <span class="preview" style:width="{el.frame.w_mm * 0.75}px" style:height="{el.frame.h_mm * 0.75}px">
          {#if el.kind.type === "Shape"}<ShapeEl props={el.kind} frame={el.frame} zoom={0.75 / 3.7795} />{/if}
        </span>
        {s.label}
      </button>
    {/each}
  </div>
</Drawer>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }
  .grid button {
    padding: 10px 4px;
    font-size: 12px;
  }
  .grid button {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .preview {
    position: relative;
    display: block;
    margin: 4px auto 0;
  }
</style>

<script lang="ts">
  import { editor } from "../lib/docStore.svelte";
  import { centeredFrame, newId } from "../lib/factory";
  import { fr } from "../lib/i18n/fr";
  import type { ShapeKind } from "../lib/types";

  function addShape(shape: ShapeKind) {
    const frame = centeredFrame(editor.page, 60, 40);
    const id = newId();
    editor.updatePage((p) => ({
      ...p,
      elements: [
        ...p.elements,
        { id, frame, locked: false, zone: false, kind: { type: "Shape", shape, fill: "#f6e3c5", stroke: null, corner_radius_mm: 0 } },
      ],
    }));
    editor.selection = [id];
  }
</script>

<aside class="left">
  <section>
    <h3>{fr.photos}</h3>
  </section>
  <section>
    <h3>{fr.shapes}</h3>
    <div class="row">
      <button onclick={() => addShape("Rect")}>▭ {fr.rect}</button>
      <button onclick={() => addShape("Ellipse")}>◯ {fr.ellipse}</button>
    </div>
  </section>
</aside>

<style>
  .left {
    grid-area: left;
    background: var(--panel);
    border-right: 1px solid var(--line);
    overflow-y: auto;
    padding: 12px;
  }
  section {
    margin-bottom: 18px;
  }
  .row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
</style>

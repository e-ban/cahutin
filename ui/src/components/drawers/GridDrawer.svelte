<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import { gridLiberate, gridPreset, gridSet } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import type { GridPreset, Page } from "../../lib/types";

  // Aperçu de chaque grille : colonnes, lignes, et les cases (colonne, ligne, largeur, hauteur).
  const presets: { id: GridPreset; label: string; cols: number; rows: number; cells: [number, number, number, number][] }[] = [
    { id: "One", label: fr.presetOne, cols: 1, rows: 1, cells: [[1, 1, 1, 1]] },
    { id: "TwoByTwo", label: fr.presetTwoByTwo, cols: 2, rows: 2, cells: [[1, 1, 1, 1], [2, 1, 1, 1], [1, 2, 1, 1], [2, 2, 1, 1]] },
    { id: "TwoByThree", label: fr.presetTwoByThree, cols: 2, rows: 3, cells: [1, 2, 3].flatMap((r) => [[1, r, 1, 1], [2, r, 1, 1]] as [number, number, number, number][]) },
    { id: "OnePlusTwo", label: fr.presetOnePlusTwo, cols: 2, rows: 2, cells: [[1, 1, 2, 1], [1, 2, 1, 1], [2, 2, 1, 1]] },
    { id: "OnePlusTwoSide", label: fr.presetOnePlusTwoSide, cols: 2, rows: 2, cells: [[1, 1, 1, 2], [2, 1, 1, 1], [2, 2, 1, 1]] },
    { id: "ThreeByThree", label: fr.presetThreeByThree, cols: 3, rows: 3, cells: [1, 2, 3].flatMap((r) => [1, 2, 3].map((c) => [c, r, 1, 1] as [number, number, number, number])) },
  ];

  let cols = $state(2);
  let rows = $state(3);

  async function run(op: (p: Page) => Promise<Page>, confirmReplace = true) {
    if (confirmReplace && editor.page.grid && !(await ask(fr.replaceGridConfirm))) return;
    const id = editor.page.id;
    try {
      const page = await op(editor.page);
      editor.updatePageById(id, () => page);
      editor.selection = [];
    } catch (e) {
      editor.notice = String(e);
    }
  }

  async function liberate() {
    if (!(await ask(fr.freeZonesConfirm))) return;
    await run(gridLiberate, false);
  }
</script>

<Drawer title={fr.grid}>
  <h3>{fr.readyGrids}</h3>
  <div class="presets">
    {#each presets as p (p.id)}
      <button class="preset" onclick={() => run((page) => gridPreset(page, p.id))}>
        <i style:grid-template-columns="repeat({p.cols}, 1fr)" style:grid-template-rows="repeat({p.rows}, 1fr)">
          {#each p.cells as [c, r, w, h], k (k)}<b style:grid-area="{r} / {c} / span {h} / span {w}"></b>{/each}
        </i>
        {p.label}
      </button>
    {/each}
  </div>
  <h3>{fr.customGrid}</h3>
  <div class="custom">
    <label>{fr.columns}
      <span class="counter"><button onclick={() => (cols = Math.max(1, cols - 1))}>−</button><output>{cols}</output><button onclick={() => (cols = Math.min(4, cols + 1))}>+</button></span>
    </label>
    <label>{fr.rows}
      <span class="counter"><button onclick={() => (rows = Math.max(1, rows - 1))}>−</button><output>{rows}</output><button onclick={() => (rows = Math.min(8, rows + 1))}>+</button></span>
    </label>
    <button onclick={() => run((page) => gridSet(page, cols, rows))}>▦ {fr.applyGrid}</button>
  </div>
  <button class="free" disabled={!editor.page.grid} onclick={liberate}>✥ {fr.freeZones}</button>
</Drawer>

<style>
  h3 {
    margin: 4px 0 0;
  }
  .presets {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px;
  }
  .preset {
    padding: 5px;
    font-size: 11px;
    color: #666;
  }
  .preset i {
    display: grid;
    gap: 2px;
    height: 44px;
    margin-bottom: 3px;
  }
  .preset b {
    background: #dbe9f7;
    border-radius: 2px;
  }
  .custom {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .custom label {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .counter {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .counter button {
    padding: 0 8px;
  }
  output {
    min-width: 18px;
    text-align: center;
  }
</style>

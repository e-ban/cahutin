<script lang="ts">
  // Choix d'un modèle pour commencer : le chemin le plus court vers une page de cahier de vie.
  import PageView from "./PageView.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { templates } from "../lib/templates.svelte";

  const MINI = 0.24;

  async function choose(id: string | null) {
    if (id) await templates.addPage(id, true);
    editor.showPicker = false;
  }
</script>

<div class="picker">
  <h2>Choisissez un modèle pour commencer</h2>
  <div class="grid">
    {#each templates.infos as t (t.id)}
      <button class="card" onclick={() => choose(t.id)}>
        <div class="mini">
          {#if templates.previews[t.id]}
            <PageView page={templates.previews[t.id]} doc={editor.doc} zoom={MINI} />
          {/if}
        </div>
        <span>{t.name}</span>
      </button>
    {/each}
    <button class="card" onclick={() => choose(null)}>
      <div class="mini blank"></div>
      <span>Page vierge</span>
    </button>
  </div>
</div>

<style>
  .picker {
    position: absolute;
    inset: 0;
    background: var(--bg);
    padding: 24px;
    overflow: auto;
    z-index: 20;
  }
  h2 {
    font: 600 22px var(--ui-font);
    margin: 0 0 20px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 18px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
    padding: 8px;
    background: var(--panel);
    font-weight: 600;
  }
  .mini {
    width: 190px;
    height: 270px;
    overflow: hidden;
    pointer-events: none;
    display: flex;
    justify-content: center;
  }
  .mini.blank {
    width: 190px;
    background: white;
    box-shadow: 0 2px 14px rgba(60, 40, 20, 0.18);
  }
</style>

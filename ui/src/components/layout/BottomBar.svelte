<script lang="ts">
  import PageView from "../PageView.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { nextZoom } from "../../lib/units";
  import { newPage } from "../../lib/factory";
  import { pageSetSize } from "../../lib/api";
  import { deletePage } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import type { Page } from "../../lib/types";

  const MINI = 0.07;

  function insert(page: Page) {
    const pages = [...editor.doc.pages];
    pages.splice(editor.pageIndex + 1, 0, page);
    editor.commit({ ...editor.doc, pages });
    editor.pageIndex += 1;
    editor.selection = [];
  }

  function duplicate() {
    const p = editor.page;
    const ids = new Map(p.elements.map((e) => [e.id, crypto.randomUUID()]));
    insert({
      ...structuredClone(p),
      id: crypto.randomUUID(),
      elements: p.elements.map((e) => ({ ...structuredClone(e), id: ids.get(e.id)! })),
      grid: p.grid ? { ...p.grid, cells: p.grid.cells.map((c) => ({ ...c, element_id: ids.get(c.element_id)! })) } : null,
    });
  }

  // Le moteur recalcule la grille éventuelle, pour que les zones restent sur la page.
  async function rotate() {
    const p = editor.page;
    try {
      const page = await pageSetSize(p, p.size.height_mm, p.size.width_mm);
      editor.updatePageById(p.id, () => page);
    } catch (e) {
      editor.notice = String(e);
    }
  }
</script>

<footer class="pagebar">
  <div class="pages">
    {#each editor.doc.pages as p, i (p.id)}
      <button class="thumb" class:current={i === editor.pageIndex} title={fr.pageN(i + 1)} onclick={() => ((editor.pageIndex = i), (editor.selection = []))}>
        <PageView page={p} doc={editor.doc} zoom={MINI} />
      </button>
    {/each}
  </div>
  <button onclick={() => insert(newPage())} title={fr.addPage}>＋ {fr.addPage}</button>
  <button class="ghost" onclick={duplicate}>⧉ {fr.duplicatePage}</button>
  <button class="ghost" class:on={editor.showBackground} onclick={() => (editor.showBackground = !editor.showBackground)}>▤ {fr.backgroundButton}</button>
  <button class="ghost" onclick={rotate}>⟳ {editor.page.size.width_mm > editor.page.size.height_mm ? fr.portrait : fr.landscape}</button>
  <button class="ghost" disabled={editor.doc.pages.length === 1} onclick={() => deletePage(editor.pageIndex)}>🗑 {fr.deletePage}</button>
  <span class="spacer"></span>
  <div class="zoom" aria-label={fr.zoom}>
    <button class="ghost" title={fr.zoomOut} aria-label={fr.zoomOut} onclick={() => (editor.zoom = nextZoom(editor.effectiveZoom, -1))}>−</button>
    <span class="pct">{Math.round(editor.effectiveZoom * 100)} %</span>
    <button class="ghost" title={fr.zoomIn} aria-label={fr.zoomIn} onclick={() => (editor.zoom = nextZoom(editor.effectiveZoom, 1))}>+</button>
    <button class:on={editor.zoom === "fit"} onclick={() => (editor.zoom = "fit")}>{fr.zoomFit}</button>
  </div>
</footer>

<style>
  .pagebar {
    grid-area: bottom;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    background: var(--panel);
    border-top: 1px solid var(--line);
  }
  .pages {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    max-width: 50vw;
    padding: 2px;
  }
  .thumb {
    padding: 2px;
    background: white;
    flex: none;
  }
  .thumb :global(.page) {
    pointer-events: none;
    box-shadow: none;
  }
  .thumb.current {
    outline: 2px solid var(--accent);
  }
  .ghost {
    border-color: transparent;
    background: transparent;
  }
  .spacer {
    flex: 1;
  }
  .zoom {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .pct {
    min-width: 48px;
    text-align: center;
    color: var(--muted);
  }
  .on {
    background: #e8f1fb;
    border-color: var(--accent);
  }
</style>

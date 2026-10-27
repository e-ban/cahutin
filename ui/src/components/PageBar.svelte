<script lang="ts">
  import PageView from "./PageView.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { newPage } from "../lib/factory";
  import { deletePage } from "../lib/fileActions";
  import { fr } from "../lib/i18n/fr";
  import type { Page } from "../lib/types";

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

  function rotate() {
    editor.updatePage((p) => ({ ...p, size: { width_mm: p.size.height_mm, height_mm: p.size.width_mm } }));
  }
</script>

<footer class="pagebar">
  <span class="label">{fr.pages}</span>
  <div class="pages">
    {#each editor.doc.pages as p, i (p.id)}
      <button class="thumb" class:current={i === editor.pageIndex} title="Page {i + 1}" onclick={() => ((editor.pageIndex = i), (editor.selection = []))}>
        <PageView page={p} doc={editor.doc} zoom={MINI} />
      </button>
    {/each}
  </div>
  <button onclick={() => insert(newPage())}>➕ {fr.addPage}</button>
  <button onclick={duplicate}>⧉ Dupliquer</button>
  <button onclick={rotate}>⟳ {editor.page.size.width_mm > editor.page.size.height_mm ? "Portrait" : "Paysage"}</button>
  <button disabled={editor.doc.pages.length === 1} onclick={() => deletePage(editor.pageIndex)}>🗑 Supprimer la page</button>
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
  .label {
    color: var(--muted);
  }
</style>

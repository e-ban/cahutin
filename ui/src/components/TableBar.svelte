<script lang="ts">
  // Barre d'une case de tableau modifiée seule (après un double-clic), sous la case : lignes, colonnes,
  // fusion et contenu. La barre flottante de la case (texte, fond) reste au-dessus.
  import { tick } from "svelte";
  import { tableInsertCol, tableInsertRow, tableMerge, tableRemoveCol, tableRemoveRow, tableSetCell, tableSplit } from "../lib/api";
  import { selectionBox } from "../lib/contextBar";
  import { editor } from "../lib/docStore.svelte";
  import { runOnPage } from "../lib/editActions";
  import { fr } from "../lib/i18n/fr";
  import { tableOf } from "../lib/table";
  import type { CellContent, GridCell } from "../lib/types";

  let { pageEl, stageEl }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined } = $props();

  /** Image d'une case passée en pictogramme, à remplacer dans le tiroir Décors. */
  const DEFAULT_PICTO = "2B50";

  let barEl = $state<HTMLElement>();
  let pos = $state<{ left: number; top: number } | null>(null);

  const cellId = $derived(editor.selection.length === 1 && editor.focusedMember === editor.selection[0] ? editor.selection[0] : null);
  const table = $derived(cellId ? tableOf(editor.page, cellId) : undefined);
  const grid = $derived(table?.kind.type === "Table" && table.id !== cellId ? table.kind : null);
  const cell = $derived(grid?.cells.find((c) => c.element_id === cellId));
  const element = $derived(cellId ? editor.page.elements.find((e) => e.id === cellId) : undefined);
  // Un tableau ou une case verrouillés ne se modifient pas.
  const visible = $derived(!!cell && !table?.locked && !element?.locked && !editor.gesture && !editor.printing);

  /** Case voisine à droite ou en dessous, pour fusionner. */
  function neighbour(dir: "right" | "below"): GridCell | undefined {
    if (!grid || !cell) return undefined;
    return dir === "right"
      ? grid.cells.find((c) => c.col === cell.col + cell.col_span && c.row === cell.row)
      : grid.cells.find((c) => c.row === cell.row + cell.row_span && c.col === cell.col);
  }

  async function reposition() {
    await tick();
    const box = visible && pageEl && stageEl && barEl && cellId ? selectionBox(pageEl, [cellId]) : null;
    if (!box || !stageEl || !barEl) {
      pos = null;
      return;
    }
    const s = stageEl.getBoundingClientRect();
    const top = Math.min(box.top + box.height + 8, s.top + s.height - barEl.offsetHeight - 8);
    const left = Math.max(s.left + 8, Math.min(box.left + box.width / 2 - barEl.offsetWidth / 2, s.left + s.width - barEl.offsetWidth - 8));
    pos = { left, top };
  }

  $effect(() => {
    void [editor.doc, editor.selection, editor.effectiveZoom, editor.drawer, editor.inspectorOpen, visible, barEl];
    reposition();
  });

  $effect(() => {
    const el = stageEl;
    if (!el) return;
    const onScroll = () => reposition();
    el.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onScroll);
    return () => {
      el.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", onScroll);
    };
  });

  /** Fin de l'écriture d'abord : le texte tapé est enregistré avant que la page change. */
  async function run(op: Parameters<typeof runOnPage>[0]) {
    editor.editingText = null;
    await tick();
    await runOnPage(op);
  }

  function setContent(content: CellContent) {
    const id = cellId;
    if (!id) return;
    // Le pictogramme et la photo se choisissent ensuite dans leur tiroir.
    run((p) => tableSetCell(p, id, content)).then(() => {
      if (content === "Photo") editor.drawer = "photos";
      else if (content !== "Text") editor.drawer = "decor";
    });
  }

  const merge = (dir: "right" | "below") => {
    const other = neighbour(dir);
    const id = cellId;
    if (other && id) run((p) => tableMerge(p, id, other.element_id));
  };
  const withCell = (op: (id: string) => Parameters<typeof runOnPage>[0]) => () => cellId && run(op(cellId));
</script>

{#if visible && cell}
  <div class="ctx" role="toolbar" tabindex="-1" aria-label={fr.tableBar} bind:this={barEl} style:left={pos ? `${pos.left}px` : "-9999px"} style:top={pos ? `${pos.top}px` : "-9999px"}>
    <button title={fr.rowAbove} aria-label={fr.rowAbove} onclick={withCell((id) => (p) => tableInsertRow(p, id, false))}>⤒ {fr.row}</button>
    <button title={fr.rowBelow} aria-label={fr.rowBelow} onclick={withCell((id) => (p) => tableInsertRow(p, id, true))}>⤓ {fr.row}</button>
    <button title={fr.colLeft} aria-label={fr.colLeft} onclick={withCell((id) => (p) => tableInsertCol(p, id, false))}>⇤ {fr.col}</button>
    <button title={fr.colRight} aria-label={fr.colRight} onclick={withCell((id) => (p) => tableInsertCol(p, id, true))}>⇥ {fr.col}</button>
    <span class="sep"></span>
    <button title={fr.removeRow} disabled={grid!.rows.length < 2} onclick={withCell((id) => (p) => tableRemoveRow(p, id))}>✕ {fr.row}</button>
    <button title={fr.removeCol} disabled={grid!.cols.length < 2} onclick={withCell((id) => (p) => tableRemoveCol(p, id))}>✕ {fr.col}</button>
    <span class="sep"></span>
    <button title={fr.mergeRight} disabled={!neighbour("right")} onclick={() => merge("right")}>{fr.mergeRightShort}</button>
    <button title={fr.mergeBelow} disabled={!neighbour("below")} onclick={() => merge("below")}>{fr.mergeBelowShort}</button>
    <button disabled={cell.col_span === 1 && cell.row_span === 1} onclick={withCell((id) => (p) => tableSplit(p, id))}>⊟ {fr.splitCell}</button>
    <span class="sep"></span>
    <button class:on={element?.kind.type === "Text"} onclick={() => setContent("Text")}>T {fr.cellText}</button>
    <button class:on={element?.kind.type === "Image"} onclick={() => setContent({ Picto: DEFAULT_PICTO })}>✿ {fr.cellPicto}</button>
    <button class:on={element?.kind.type === "Photo"} onclick={() => setContent("Photo")}>🖼 {fr.cellPhoto}</button>
  </div>
{/if}

<style>
  .ctx {
    position: fixed;
    display: flex;
    gap: 2px;
    align-items: center;
    background: white;
    border: 1px solid var(--line);
    border-radius: 9px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.12);
    padding: 3px;
    white-space: nowrap;
    z-index: 3500;
  }
  button {
    border-color: transparent;
    background: transparent;
    padding: 4px 8px;
  }
  button:not(:disabled):hover {
    background: #f3efe8;
  }
  .on {
    background: #e8f1fb;
  }
  .sep {
    width: 1px;
    align-self: stretch;
    background: var(--line);
    margin: 3px 2px;
  }
</style>

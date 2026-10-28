<script lang="ts">
  // Barre d'une case de grille en mode modèle : type de zone, diviser, fixer, vider.
  import { tick } from "svelte";
  import { gridSetKind, gridSplit } from "../lib/api";
  import { placeContextBar, selectionBox } from "../lib/contextBar";
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { spannedCell } from "../lib/templateMode";
  import type { Page, ZoneKind } from "../lib/types";

  let { pageEl, stageEl }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined } = $props();

  let barEl = $state<HTMLElement>();
  let pos = $state<{ left: number; top: number } | null>(null);

  const cell = $derived(
    editor.templateMode && editor.selection.length === 1 && editor.page.grid?.cells.some((c) => c.element_id === editor.selection[0])
      ? editor.page.elements.find((e) => e.id === editor.selection[0])
      : undefined,
  );
  const visible = $derived(!!cell && !editor.mergePreview && !editor.editingText);
  const spanned = $derived(spannedCell(editor.selection, editor.page));

  async function reposition() {
    await tick();
    const box = visible && pageEl && stageEl && barEl ? selectionBox(pageEl, editor.selection) : null;
    if (!box || !stageEl || !barEl) {
      pos = null;
      return;
    }
    const s = stageEl.getBoundingClientRect();
    pos = placeContextBar(box, { width: barEl.offsetWidth, height: barEl.offsetHeight }, { left: s.left, top: s.top, width: s.width, height: s.height });
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

  async function run(op: (p: Page) => Promise<Page>) {
    const id = editor.page.id;
    try {
      const page = await op(editor.page);
      editor.updatePageById(id, () => page);
    } catch (e) {
      editor.notice = String(e);
    }
  }

  const setKind = (kind: ZoneKind) => cell && run((p) => gridSetKind(p, cell.id, kind));

  function empty() {
    if (!cell) return;
    editor.updateElement(cell.id, (e) =>
      e.kind.type === "Photo"
        ? { ...e, kind: { ...e.kind, asset: null, crop: { scale: 1, center_x: 0.5, center_y: 0.5 } } }
        : e.kind.type === "Text"
          ? { ...e, kind: { ...e.kind, content: { paragraphs: [] } } }
          : e,
    );
  }
</script>

{#if visible && cell}
  <div class="ctx" role="toolbar" tabindex="-1" aria-label="Barre de la case" bind:this={barEl} style:left={pos ? `${pos.left}px` : "-9999px"} style:top={pos ? `${pos.top}px` : "-9999px"}>
    <button class="photo" class:on={cell.kind.type === "Photo"} onclick={() => setKind("Photo")}>🖼 {fr.cellPhoto}</button>
    <button class="text" class:on={cell.kind.type === "Text"} onclick={() => setKind("Text")}>T {fr.cellText}</button>
    <span class="sep"></span>
    <button disabled={!spanned} onclick={() => spanned && run((p) => gridSplit(p, spanned))}>⊟ {fr.splitCell}</button>
    <button class:on={cell.locked} onclick={() => editor.updateElement(cell.id, (e) => ({ ...e, locked: !e.locked }))}>📌 {cell.locked ? fr.unfixCell : fr.fixCell}</button>
    <button onclick={empty}>⌫ {fr.emptyCell}</button>
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
  .photo.on {
    background: rgba(46, 134, 222, 0.12);
    color: var(--zone-photo);
  }
  .text.on {
    background: rgba(230, 126, 34, 0.12);
    color: var(--zone-text);
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

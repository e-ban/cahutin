<script lang="ts">
  import PageView from "../PageView.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { nextZoom } from "../../lib/units";
  import { newPage } from "../../lib/factory";
  import { pageSetSize } from "../../lib/api";
  import { changeFormat, formatOf, type Format } from "../../lib/pageFormat";
  import { remapTables } from "../../lib/table";
  import { deletePage } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import { classes } from "../../lib/classStore.svelte";
  import { childOfPage, findChildPage } from "../../lib/classFill";
  import type { Page } from "../../lib/types";

  const MINI = 0.07;

  let strip = $state<HTMLElement>();
  const count = $derived(editor.doc.pages.length);
  const students = $derived(classes.current?.class.students ?? []);

  // La miniature de la page courante reste visible dans la bande, même à la page 30.
  $effect(() => {
    strip?.children[editor.pageIndex]?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });

  // L'enfant de chaque page (pages générées pour la classe), pour les info-bulles et la recherche.
  const children = $derived(editor.doc.pages.map((p) => childOfPage(p, students)));

  /** Un numéro mène à cette page ; un prénom, à la page suivante faite pour cet enfant. */
  function goTo(input: HTMLInputElement) {
    const text = input.value.trim();
    const n = Number(text);
    const target = /^\d+$/.test(text) ? Math.min(count, Math.max(1, n)) - 1 : findChildPage(editor.doc.pages, students, text, editor.pageIndex);
    if (target === undefined) editor.notice = fr.pageChildNotFound(text);
    else {
      editor.pageIndex = target;
      editor.selection = [];
    }
    input.value = String(editor.pageIndex + 1);
  }

  /** « Page 3 · Léa » pour une page faite pour un enfant. */
  const thumbTitle = (i: number) => (children[i] ? fr.pageThumbFor(i + 1, children[i]!) : fr.pageN(i + 1));

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
      elements: remapTables(p.elements.map((e) => ({ ...structuredClone(e), id: ids.get(e.id)! })), ids),
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

  /** A5, A4 ou A3 : le contenu suit la taille de la page. */
  async function setFormat(format: Format) {
    const p = changeFormat(editor.page, format);
    try {
      const page = await pageSetSize(p, p.size.width_mm, p.size.height_mm);
      editor.updatePageById(p.id, () => page);
    } catch (e) {
      editor.notice = String(e);
    }
  }
</script>

<footer class="pagebar">
  {#if count > 1}
    <label class="count" title={fr.pageGo}>
      <input value={editor.pageIndex + 1} list="page-children" aria-label={fr.pageGo} onchange={(e) => goTo(e.currentTarget)} onfocus={(e) => e.currentTarget.select()} />
      <span>/ {count}</span>
    </label>
  {/if}
  <datalist id="page-children">
    {#each [...new Set(children.filter(Boolean))] as c (c)}<option value={c}></option>{/each}
  </datalist>
  <div class="pages" bind:this={strip}>
    {#each editor.doc.pages as p, i (p.id)}
      <button class="thumb" class:current={i === editor.pageIndex} title={thumbTitle(i)} onclick={() => ((editor.pageIndex = i), (editor.selection = []))}>
        <PageView page={p} doc={editor.doc} zoom={MINI} />
      </button>
    {/each}
  </div>
  <button onclick={() => insert(newPage())} title={fr.addPage}>＋ {fr.addPage}</button>
  <button class="ghost" onclick={duplicate}>⧉ {fr.duplicatePage}</button>
  <button class="ghost" class:on={editor.showBackground} onclick={() => (editor.showBackground = !editor.showBackground)}>▤ {fr.backgroundButton}</button>
  <select class="format" title={fr.pageFormat} aria-label={fr.pageFormat} value={formatOf(editor.page.size) ?? ""} onchange={(e) => setFormat(e.currentTarget.value as Format)}>
    {#if !formatOf(editor.page.size)}<option value="">{fr.pageFormatOther}</option>{/if}
    <option value="A5">A5</option>
    <option value="A4">A4</option>
    <option value="A3">A3</option>
  </select>
  <button class="ghost" class:on={editor.spread} aria-pressed={editor.spread} title={fr.spreadHint} onclick={() => (editor.spread = !editor.spread)}>📖 {fr.spread}</button>
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
    min-width: 150px;
    flex: 0 1 auto;
    padding: 2px;
  }
  .count {
    display: flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
    white-space: nowrap;
    flex-shrink: 0;
  }
  .count input {
    width: 4.5em;
    text-align: center;
  }
  /* Les boutons gardent leur libellé sur une ligne ; la bande des miniatures cède la place. */
  .pagebar > button,
  .pagebar > select {
    white-space: nowrap;
    flex-shrink: 0;
  }
  @media (max-width: 1150px) {
    .pagebar > button {
      padding-left: 6px;
      padding-right: 6px;
    }
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

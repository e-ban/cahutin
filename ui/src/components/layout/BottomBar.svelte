<script lang="ts">
  import PageView from "../PageView.svelte";
  import Icon from "../Icon.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { nextZoom } from "../../lib/units";
  import { newPage } from "../../lib/factory";
  import { pageSetSize } from "../../lib/api";
  import { changeFormat, formatOf, type Format } from "../../lib/pageFormat";
  import { remapTables } from "../../lib/table";
  import { deletePage } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
  import { classes } from "../../lib/classStore.svelte";
  import { childOfPage, findChildPage } from "../../lib/classFill";
  import { dropSlot, slotTarget } from "../../lib/pageOrder";
  import { DRAG_DEAD_ZONE_PX } from "../../lib/geometry";
  import type { Page } from "../../lib/types";

  // Miniatures basses : la bande tient sur une ligne, sous les boutons de la page.
  const MINI = 0.05;

  let strip = $state<HTMLElement>();
  const count = $derived(editor.doc.pages.length);
  const students = $derived(classes.current?.class.students ?? []);

  // La miniature de la page courante reste visible dans la bande, même à la page 30.
  $effect(() => {
    strip?.children[editor.pageIndex]?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });

  /** La molette fait défiler la bande des miniatures de gauche à droite ; Ctrl+molette reste le zoom. */
  function scrollStrip(ev: WheelEvent) {
    if (!strip || ev.ctrlKey || ev.metaKey || Math.abs(ev.deltaY) <= Math.abs(ev.deltaX)) return;
    ev.preventDefault();
    strip.scrollLeft += ev.deltaMode === 1 ? ev.deltaY * 40 : ev.deltaMode === 2 ? ev.deltaY * strip.clientWidth : ev.deltaY;
  }

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

  // Glisser une miniature change la page de place. À la souris, le geste commence après quelques pixels (un clic
  // ouvre toujours la page) ; au doigt, après un appui d'un instant sans bouger, pour que la bande défile encore au
  // doigt. Un trait montre où la page tombera ; près des bords, la bande défile ; Échap annule.
  const HOLD_MS = 400;
  const EDGE_PX = 48;
  /** Au-delà, au-dessus ou au-dessous de la bande, la page ne tombe nulle part. */
  const REACH_PX = 60;
  let press: { from: number; x: number; y: number; pointer: number; touch: boolean; timer: number } | null = null;
  let drag = $state<{ from: number; slot: number | null } | null>(null);
  let pointer = { x: 0, y: 0 };
  let swallowClick = false;
  let frame = 0;

  function thumbDown(ev: PointerEvent, i: number) {
    swallowClick = false;
    if (ev.button !== 0 || count < 2 || press) return;
    const touch = ev.pointerType !== "mouse";
    pointer = { x: ev.clientX, y: ev.clientY };
    press = { from: i, x: ev.clientX, y: ev.clientY, pointer: ev.pointerId, touch, timer: touch ? window.setTimeout(startDrag, HOLD_MS) : 0 };
  }

  function startDrag() {
    if (!press || drag) return;
    drag = { from: press.from, slot: null };
    frame = requestAnimationFrame(follow);
  }

  function moved(ev: PointerEvent) {
    if (!press || ev.pointerId !== press.pointer) return;
    pointer = { x: ev.clientX, y: ev.clientY };
    if (!drag) {
      if (Math.hypot(ev.clientX - press.x, ev.clientY - press.y) < DRAG_DEAD_ZONE_PX) return;
      // Au doigt, bouger tout de suite fait défiler la bande, comme avant.
      if (press.touch) return stop();
      startDrag();
    }
    aim();
  }

  /** La fente visée : entre les deux miniatures autour du pointeur, s'il est sur la bande et que la page y bouge. */
  function aim() {
    if (!drag || !strip) return;
    const r = strip.getBoundingClientRect();
    const near = pointer.y > r.top - REACH_PX && pointer.y < r.bottom + REACH_PX && pointer.x > r.left - REACH_PX && pointer.x < r.right + REACH_PX;
    const mids = [...strip.querySelectorAll<HTMLElement>(".thumb")].map((t) => {
      const b = t.getBoundingClientRect();
      return b.left + b.width / 2;
    });
    const slot = dropSlot(mids, pointer.x);
    drag = { ...drag, slot: near && slotTarget(drag.from, slot) !== null ? slot : null };
  }

  /** Pendant le geste, près d'un bord de la bande (ou au-delà), elle défile, d'autant plus vite qu'on en est près. */
  function follow() {
    if (!drag || !strip) return;
    const r = strip.getBoundingClientRect();
    if (pointer.y > r.top - REACH_PX && pointer.y < r.bottom + REACH_PX) {
      // Une bande étroite (grande interface, petite fenêtre) garde de la place entre ses deux bords.
      const edge = Math.min(EDGE_PX, r.width / 6);
      const speed = (d: number) => Math.min(16, Math.ceil((edge - d) / 3));
      if (pointer.x < r.left + edge) strip.scrollLeft -= speed(Math.max(0, pointer.x - r.left));
      else if (pointer.x > r.right - edge) strip.scrollLeft += speed(Math.max(0, r.right - pointer.x));
    }
    aim();
    frame = requestAnimationFrame(follow);
  }

  function stop() {
    if (press) clearTimeout(press.timer);
    cancelAnimationFrame(frame);
    press = null;
    drag = null;
  }

  function released(ev: PointerEvent) {
    // Après Échap : le clic de ce relâchement est encore ignoré, pas le suivant.
    if (swallowClick && !press) setTimeout(() => (swallowClick = false));
    if (!press || ev.pointerId !== press.pointer) return;
    const d = drag;
    stop();
    if (!d) return;
    // Le clic qui suit le relâchement n'ouvre pas la page sous le pointeur.
    swallowClick = true;
    setTimeout(() => (swallowClick = false));
    const to = d.slot === null ? null : slotTarget(d.from, d.slot);
    if (to !== null) editor.movePage(d.from, to);
  }

  /** Échap pendant le geste : la page reste où elle était ; la touche ne va pas plus loin (la sélection reste). */
  function escape(ev: KeyboardEvent) {
    if (ev.key !== "Escape" || !drag) return;
    ev.preventDefault();
    ev.stopPropagation();
    swallowClick = true;
    stop();
  }

  // Au doigt, une fois le geste commencé, la bande ne défile plus sous lui (un écouteur non passif le peut seul).
  $effect(() => {
    const el = strip;
    if (!el) return;
    const hold = (ev: TouchEvent) => drag && ev.preventDefault();
    el.addEventListener("touchmove", hold, { passive: false });
    return () => el.removeEventListener("touchmove", hold);
  });

  function open(i: number) {
    if (swallowClick) return void (swallowClick = false);
    editor.pageIndex = i;
    editor.selection = [];
  }

  const landscape = $derived(editor.page.size.width_mm > editor.page.size.height_mm);
  const formats: Format[] = ["A5", "A4", "A3"];

</script>

<svelte:window onpointermove={moved} onpointerup={released} onpointercancel={() => press && stop()} onkeydowncapture={escape} onblur={() => press && stop()} />

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
  <div class="pages" class:moving={drag} bind:this={strip} onwheel={scrollStrip} onscroll={aim}>
    {#each editor.doc.pages as p, i (p.id)}
      <button
        class="thumb"
        class:current={i === editor.pageIndex}
        class:dragged={drag?.from === i}
        class:drop-before={drag?.slot === i}
        class:drop-after={drag?.slot === count && i === count - 1}
        title={count > 1 ? `${thumbTitle(i)}\n${fr.pageThumbDrag}` : thumbTitle(i)}
        aria-label={thumbTitle(i)}
        aria-current={i === editor.pageIndex ? "page" : undefined}
        onpointerdown={(e) => thumbDown(e, i)}
        onclick={() => open(i)}
      >
        <PageView page={p} doc={editor.doc} zoom={MINI} />
        <span class="num" aria-hidden="true">{i + 1}</span>
      </button>
    {/each}
  </div>
  <!-- Toutes les commandes de la page restent visibles : deux groupes, ajouter et retirer, puis son allure. -->
  <button class="add" onclick={() => insert(newPage())} title={fr.addPage}><Icon name="add" /> {fr.addPage}</button>
  <button class="ghost" onclick={duplicate}><Icon name="duplicate" /> {fr.duplicatePage}</button>
  <button class="ghost" title={fr.deletePage} disabled={count === 1} onclick={() => deletePage(editor.pageIndex)}><Icon name="trash" /> {fr.delete}</button>
  <span class="move">
    <button class="ghost" title={fr.movePageBefore} aria-label={fr.movePageBefore} disabled={editor.pageIndex === 0} onclick={() => editor.movePage(editor.pageIndex, editor.pageIndex - 1)}><Icon name="arrowLeft" /></button>
    <button class="ghost" title={fr.movePageAfter} aria-label={fr.movePageAfter} disabled={editor.pageIndex >= count - 1} onclick={() => editor.movePage(editor.pageIndex, editor.pageIndex + 1)}><Icon name="arrowRight" /></button>
  </span>
  <span class="sep"></span>
  <button class="ghost" class:on={editor.showBackground} aria-pressed={editor.showBackground} onclick={() => (editor.showBackground = !editor.showBackground)}><Icon name="background" /> {fr.backgroundButton}</button>
  <select class="format" title={fr.pageFormat} aria-label={fr.pageFormat} value={formatOf(editor.page.size) ?? ""} onchange={(e) => setFormat(e.currentTarget.value as Format)}>
    {#if !formatOf(editor.page.size)}<option value="">{fr.pageFormatOther}</option>{/if}
    {#each formats as f (f)}<option value={f}>{f}</option>{/each}
  </select>
  <button class="ghost" onclick={rotate}><Icon name={landscape ? "portrait" : "landscape"} /> {landscape ? fr.portrait : fr.landscape}</button>
  <button class="ghost" class:on={editor.spread} aria-pressed={editor.spread} title={fr.spreadHint} onclick={() => (editor.spread = !editor.spread)}><Icon name="spread" /> {fr.spread}</button>
  {#if settings.cvdPreview}<button class="ghost" class:on={editor.cvdView} aria-pressed={editor.cvdView} title={fr.cvdViewHint} onclick={() => (editor.cvdView = !editor.cvdView)}><Icon name="cvd" /> {fr.cvdView}</button>{/if}
  <span class="spacer"></span>
  <div class="zoom" aria-label={fr.zoom}>
    <button class="ghost" title={fr.zoomOut} aria-label={fr.zoomOut} onclick={() => (editor.zoom = nextZoom(editor.effectiveZoom, -1))}><Icon name="zoomOut" /></button>
    <span class="pct">{Math.round(editor.effectiveZoom * 100)} %</span>
    <button class="ghost" title={fr.zoomIn} aria-label={fr.zoomIn} onclick={() => (editor.zoom = nextZoom(editor.effectiveZoom, 1))}><Icon name="zoomIn" /></button>
    <button class:on={editor.zoom === "fit"} onclick={() => (editor.zoom = "fit")}>{fr.zoomFit}</button>
  </div>
</footer>

<style>
  .pagebar {
    grid-area: bottom;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 14px;
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
    /* De quoi montrer le trait de dépôt avant la première miniature et après la dernière. */
    padding: 2px 6px;
  }
  .pages.moving,
  .pages.moving .thumb {
    cursor: grabbing;
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
  /* « Ajouter une page » : la commande qu'on cherche, bien visible sans être un bouton plein. */
  .add {
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }
  @media (max-width: 1150px) {
    .pagebar > button {
      padding-left: 6px;
      padding-right: 6px;
    }
  }
  .thumb {
    position: relative;
    padding: 2px;
    background: white;
    flex: none;
    /* Au doigt, l'appui long sert à glisser la page : ni bulle du système, ni sélection. */
    -webkit-touch-callout: none;
    user-select: none;
  }
  /* La miniature qu'on glisse s'efface à moitié ; un trait épais marque l'endroit où elle tombera. */
  .thumb.dragged {
    opacity: 0.4;
  }
  .thumb.drop-before::before,
  .thumb.drop-after::after {
    content: "";
    position: absolute;
    top: -2px;
    bottom: -2px;
    width: 4px;
    border-radius: 2px;
    background: var(--accent);
    box-shadow: 0 0 0 1px var(--panel);
  }
  .thumb.drop-before::before {
    left: -6px;
  }
  .thumb.drop-after::after {
    right: -6px;
  }
  .move {
    display: flex;
    flex-shrink: 0;
  }
  /* Deux flèches serrées : la bande des miniatures garde sa place. */
  .move > button {
    display: flex;
    padding-left: 5px;
    padding-right: 5px;
  }
  /* Le numéro de la page, dans le coin de sa miniature. */
  .num {
    position: absolute;
    right: 3px;
    bottom: 3px;
    min-width: 14px;
    padding: 0 3px;
    border-radius: 4px;
    background: var(--panel);
    border: 1px solid var(--line);
    color: var(--text);
    font-size: 10px;
    line-height: 13px;
    text-align: center;
  }
  .thumb.current .num {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
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
  .sep {
    width: 1px;
    align-self: stretch;
    margin: 6px 2px;
    background: var(--line);
    flex: none;
  }
  .spacer {
    flex: 1;
  }
  .zoom > button {
    display: flex;
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
    background: var(--accent-soft);
    border-color: var(--accent);
  }
</style>

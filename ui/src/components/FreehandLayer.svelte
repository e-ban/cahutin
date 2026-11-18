<script lang="ts">
  // Dessin à main levée : pendant que le crayon est pris (lib/pen.svelte.ts), appuyer sur la page et glisser trace
  // un chemin (lib/freehand.ts), un pas d'annulation par tracé. Un bouton, un menu ou Échap rangent le crayon ; la
  // marge grise ne fait rien.
  import { editor } from "../lib/docStore.svelte";
  import { strokeElement } from "../lib/freehand";
  import { fr } from "../lib/i18n/fr";
  import type { Pt } from "../lib/linePath";
  import { pen } from "../lib/pen.svelte";
  import { PX_PER_MM, mmToPx } from "../lib/units";

  let { pageEl, stageEl, zoom }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined; zoom: number } = $props();

  /** Un point n'est gardé qu'à cette distance du précédent (mm). */
  const STEP_MM = 0.3;

  let stroke = $state<Pt[] | null>(null);
  let pointer = -1;
  /** Bulle d'aide au premier usage du crayon dans la séance. */
  let hintShown = false;
  let hint = $state<{ x: number; y: number } | null>(null);
  let swallowUntil = 0;

  const toPage = (ev: { clientX: number; clientY: number }): Pt | null => {
    if (!pageEl) return null;
    const r = pageEl.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / (PX_PER_MM * zoom), y: (ev.clientY - r.top) / (PX_PER_MM * zoom) };
  };
  const onPage = (t: EventTarget | null) => !!pageEl && t instanceof Node && pageEl.contains(t);
  const inStage = (t: EventTarget | null) => !!stageEl && t instanceof Node && stageEl.contains(t);
  const control = (t: EventTarget | null) => t instanceof Element && !!t.closest("button, input, select, textarea, a, [role='menu'], [role='dialog'], .pop");

  function stop() {
    pen.active = false;
    stroke = null;
    hint = null;
  }

  $effect(() => {
    if (!pen.active) return;
    if (!hintShown) {
      hintShown = true;
      hint = { x: 0, y: 0 };
    }
    // Le crayon pris, rien n'est sélectionné : le tracé ne déplace pas un élément.
    editor.selection = [];
    const down = (ev: PointerEvent) => {
      if (ev.target instanceof Element && ev.target.closest("[data-pen-toggle]")) return;
      if (control(ev.target) || !inStage(ev.target)) return stop();
      if (ev.button !== 0) return;
      ev.preventDefault();
      ev.stopPropagation();
      swallowUntil = performance.now() + 400;
      if (!onPage(ev.target)) return;
      const p = toPage(ev);
      if (!p) return;
      pointer = ev.pointerId;
      stroke = [p];
      hint = null;
    };
    const moveAt = (ev: PointerEvent) => {
      if (hint) hint = { x: ev.clientX, y: ev.clientY };
      if (!stroke || ev.pointerId !== pointer) return;
      const p = toPage(ev);
      const last = stroke[stroke.length - 1];
      if (p && Math.hypot(p.x - last.x, p.y - last.y) >= STEP_MM) stroke = [...stroke, p];
    };
    const up = (ev: PointerEvent) => {
      if (!stroke || ev.pointerId !== pointer) return;
      const done = stroke;
      stroke = null;
      const p = toPage(ev);
      const el = strokeElement(editor.page, p ? [...done, p] : done);
      if (el) editor.updatePage((page) => ({ ...page, elements: [...page.elements, { ...el, locked: editor.templateMode }] }));
    };
    const swallow = (ev: Event) => {
      if (performance.now() < swallowUntil) {
        ev.preventDefault();
        ev.stopPropagation();
      }
    };
    const key = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") {
        ev.preventDefault();
        ev.stopPropagation();
        stop();
      }
    };
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("pointermove", moveAt, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("click", swallow, true);
    window.addEventListener("dblclick", swallow, true);
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointermove", moveAt, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("click", swallow, true);
      window.removeEventListener("dblclick", swallow, true);
      window.removeEventListener("keydown", key, true);
    };
  });

  // Une autre page range le crayon.
  let lastPage = editor.pageIndex;
  $effect(() => {
    if (editor.pageIndex !== lastPage) {
      lastPage = editor.pageIndex;
      if (pen.active) stop();
    }
  });

  $effect(() => {
    if (!stageEl) return;
    stageEl.classList.toggle("pen", pen.active);
  });

  /** Le tracé en cours vit dans la page : il suit son défilement et son zoom. */
  function inPage(node: Element) {
    pageEl?.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if pen.active && stroke && pageEl}
  <svg class="ink" use:inPage aria-hidden="true">
    <polyline points={stroke.map((p) => `${mmToPx(p.x, zoom)},${mmToPx(p.y, zoom)}`).join(" ")} />
  </svg>
{/if}
{#if pen.active && hint && hint.x}
  <div class="hint" style:left="{hint.x + 16}px" style:top="{hint.y + 16}px">{fr.penHint}</div>
{/if}

<style>
  .ink {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
    z-index: 3050;
  }
  .ink polyline {
    fill: none;
    stroke: #333;
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .hint {
    position: fixed;
    z-index: 5000;
    pointer-events: none;
    background: var(--panel, #fff);
    color: var(--text, #222);
    border: 1px solid var(--border, #ccc);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 13px;
    box-shadow: 0 2px 8px rgb(0 0 0 / 15%);
  }
  :global(.pen .page),
  :global(.pen .page *) {
    cursor: crosshair !important;
  }
</style>

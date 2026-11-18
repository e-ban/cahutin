<script lang="ts">
  // Prolongement d'un trait clic après clic (lib/extending.svelte.ts). Pendant le prolongement, les clics sur la
  // page ajoutent un point et ne sélectionnent rien ; un clic dans la marge grise ne fait rien ; un bouton, un
  // menu, une autre page, un clic droit, Échap, Entrée ou un clic sur le dernier point posé le terminent.
  // Ctrl+Z retire le dernier point posé. Un trait fin pointillé va du bout à la souris.
  import { editor } from "../lib/docStore.svelte";
  import { extendKey, extending } from "../lib/extending.svelte";
  import { snapLines } from "../lib/geometry";
  import { boundsOf } from "../lib/groups";
  import { fr } from "../lib/i18n/fr";
  import type { Pt } from "../lib/linePath";
  import { PX_PER_MM, mmToPx } from "../lib/units";

  let { pageEl, stageEl, zoom }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined; zoom: number } = $props();

  /** Lignes d'accroche à cette distance, à l'écran (comme les poignées). */
  const SNAP_PX = 6;
  /** Un clic aussi près du dernier point posé termine (le second clic d'un double-clic aussi). */
  const DONE_PX = 6;

  /** La bulle d'aide se montre au premier prolongement de la séance de travail. */
  let hintShown = false;
  let hint = $state<{ x: number; y: number } | null>(null);
  /** « Ce chemin a déjà 200 points », montré trois secondes où l'on a cliqué. */
  let full = $state<{ x: number; y: number } | null>(null);
  let fullSeq = 0;
  /** Après un clic pris par le prolongement, les `click` et `dblclick` qui suivent sont avalés. */
  let swallowUntil = 0;
  /** Dernier clic qui a posé un point (écran, instant) : un second clic aussitôt au même endroit termine. */
  let lastClick: { x: number; y: number; t: number } | null = null;

  const tolMm = () => SNAP_PX / (PX_PER_MM * zoom);
  const toPage = (ev: { clientX: number; clientY: number }): Pt | null => {
    if (!pageEl) return null;
    const r = pageEl.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / (PX_PER_MM * zoom), y: (ev.clientY - r.top) / (PX_PER_MM * zoom) };
  };
  const lines = () => {
    const page = editor.page;
    return snapLines(page.size, page.elements.filter((o) => o.id !== extending.id).map((o) => boundsOf([o.frame])));
  };
  const onPage = (t: EventTarget | null) => !!pageEl && t instanceof Node && pageEl.contains(t);
  const inStage = (t: EventTarget | null) => !!stageEl && t instanceof Node && stageEl.contains(t);
  /** Un bouton, un champ, un menu : le prolongement s'arrête et le clic fait son effet. */
  const control = (t: EventTarget | null) => t instanceof Element && !!t.closest("button, input, select, textarea, a, [role='menu'], [role='dialog'], .pop");

  function finish() {
    extending.finish();
    hint = null;
  }

  $effect(() => {
    if (!extending.active) return;
    lastClick = null;
    if (!hintShown) {
      hintShown = true;
      hint = { x: 0, y: 0 };
    }
    const down = (ev: PointerEvent) => {
      // Clic du milieu (ou autre bouton) : rien, il garde son usage.
      if (ev.button !== 0 && ev.button !== 2) return;
      if (ev.button === 2) {
        ev.preventDefault();
        ev.stopPropagation();
        swallowUntil = performance.now() + 400;
        return finish();
      }
      // Le bouton « Prolonger » lui-même : il termine, sans relancer.
      if (ev.target instanceof Element && ev.target.closest("[data-extend-toggle]")) {
        ev.preventDefault();
        ev.stopPropagation();
        swallowUntil = performance.now() + 400;
        return finish();
      }
      if (control(ev.target) || !inStage(ev.target)) return finish();
      ev.preventDefault();
      ev.stopPropagation();
      swallowUntil = performance.now() + 400;
      if (!onPage(ev.target)) {
        // Une autre page (vis-à-vis) : fin ; la marge grise : rien.
        if (ev.target instanceof Element && ev.target.closest(".page")) finish();
        return;
      }
      const p = toPage(ev);
      const tip = extending.tip;
      if (!p) return;
      // Clic sur l'autre bout, à partir de trois points : le chemin se ferme.
      const other = extending.otherEnd;
      if (extending.canClose && other && Math.hypot(p.x - other.x, p.y - other.y) * PX_PER_MM * zoom < DONE_PX) {
        extending.close();
        hint = null;
        return;
      }
      // Second clic d'un double-clic (même endroit, aussitôt), ou clic sur le dernier point posé : fin.
      const now = performance.now();
      const again = lastClick && now - lastClick.t < 500 && Math.hypot(ev.clientX - lastClick.x, ev.clientY - lastClick.y) < DONE_PX;
      lastClick = { x: ev.clientX, y: ev.clientY, t: now };
      if (again || (tip && Math.hypot(p.x - tip.x, p.y - tip.y) * PX_PER_MM * zoom < DONE_PX)) return finish();
      extending.add(p, ev.shiftKey, lines(), tolMm());
      hint = null;
      if (extending.full) {
        const n = ++fullSeq;
        full = { x: ev.clientX, y: ev.clientY };
        setTimeout(() => n === fullSeq && (full = null), 3000);
      }
    };
    const moveAt = (ev: PointerEvent) => {
      const p = onPage(ev.target) || inStage(ev.target) ? toPage(ev) : null;
      extending.preview = p ? extending.snap(p, ev.shiftKey, lines(), tolMm()) : null;
      if (hint) hint = { x: ev.clientX, y: ev.clientY };
    };
    const key = (ev: KeyboardEvent) => {
      const what = extendKey(ev);
      if (what === "ignore") return;
      if (what === "pass") return finish();
      ev.preventDefault();
      ev.stopPropagation();
      if (what === "finish") finish();
      else extending.undoLast();
    };
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("pointermove", moveAt, true);
    window.addEventListener("keydown", key, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointermove", moveAt, true);
      window.removeEventListener("keydown", key, true);
    };
  });

  // Les clics qui suivent un clic pris par le prolongement sont avalés, même après sa fin : le `dblclick` d'un
  // double-clic arrive une fois le prolongement fini, et retirerait sinon le point qu'il vient de poser.
  $effect(() => {
    const swallow = (ev: Event) => {
      if (performance.now() < swallowUntil) {
        ev.preventDefault();
        ev.stopPropagation();
      }
    };
    for (const name of ["click", "dblclick", "contextmenu"]) window.addEventListener(name, swallow, true);
    return () => {
      for (const name of ["click", "dblclick", "contextmenu"]) window.removeEventListener(name, swallow, true);
    };
  });

  // Une autre sélection ou une autre page termine le prolongement.
  $effect(() => {
    const sel = editor.selection;
    if (extending.active && (sel.length !== 1 || sel[0] !== extending.id)) finish();
  });
  let lastPage = editor.pageIndex;
  $effect(() => {
    if (editor.pageIndex !== lastPage) {
      lastPage = editor.pageIndex;
      if (extending.active) finish();
    }
  });

  // Curseur en croix sur la zone de travail pendant le prolongement.
  $effect(() => {
    if (!stageEl) return;
    stageEl.classList.toggle("extending", extending.active);
  });

  /** Le trait fin vit dans la page : il suit son défilement et son zoom. */
  function inPage(node: Element) {
    pageEl?.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if extending.active && pageEl}
  {@const tip = extending.tip}
  <svg class="next" use:inPage aria-hidden="true">
    {#if tip && extending.preview}
      <line x1={mmToPx(tip.x, zoom)} y1={mmToPx(tip.y, zoom)} x2={mmToPx(extending.preview.x, zoom)} y2={mmToPx(extending.preview.y, zoom)} />
    {/if}
  </svg>
{/if}
{#if full}
  <div class="hint" style:left="{full.x + 16}px" style:top="{full.y + 16}px">{fr.extendFull}</div>
{/if}
{#if extending.active && hint && hint.x}
  <div class="hint" style:left="{hint.x + 16}px" style:top="{hint.y + 16}px">{fr.extendHint}</div>
{/if}

<style>
  .next {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    overflow: visible;
    pointer-events: none;
    z-index: 3050;
  }
  .next line {
    stroke: var(--accent, #2b7de9);
    stroke-width: 1.5;
    stroke-dasharray: 5 4;
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
  :global(.extending .page),
  :global(.extending .page *) {
    cursor: crosshair !important;
  }
</style>

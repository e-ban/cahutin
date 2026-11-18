<script lang="ts">
  // Poignées aux deux bouts d'un trait ou d'une flèche : tirer une poignée déplace ce bout, l'autre reste en place.
  // Pendant le geste, le document est en aperçu ; il change une seule fois au relâcher (un pas d'annulation).
  // Échap, ou un geste interrompu, ne change rien.
  import { editor } from "../lib/docStore.svelte";
  import { snapLines } from "../lib/geometry";
  import { boundsOf } from "../lib/groups";
  import { handleTarget } from "../lib/handleTarget";
  import { dragEnd, isLineTarget, lineEndpoints, withHitHeight, type Pt } from "../lib/lineEnds";
  import { fr } from "../lib/i18n/fr";
  import { settings } from "../lib/settings.svelte";
  import { PX_PER_MM, mmToPx } from "../lib/units";
  import type { Document } from "../lib/types";

  let { pageEl, zoom }: { pageEl: HTMLElement | undefined; zoom: number } = $props();

  /** Lignes d'accroche à cette distance du bout, à l'écran (comme Moveable). */
  const SNAP_PX = 6;

  const target = $derived.by(() => {
    const e = handleTarget();
    return e && isLineTarget(e) ? e : null;
  });
  const ends = $derived(target ? lineEndpoints(target.frame, !!target.flip_h) : null);

  let drag = $state<{ id: string; which: "start" | "end"; base: Document; pointer: number } | null>(null);
  /** Lignes où le bout s'est accroché pendant le geste, montrées sur la page. */
  let guides = $state<{ x: number | null; y: number | null }>({ x: null, y: null });

  function toPage(ev: PointerEvent): Pt | null {
    if (!pageEl) return null;
    const r = pageEl.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / (PX_PER_MM * zoom), y: (ev.clientY - r.top) / (PX_PER_MM * zoom) };
  }

  function start(ev: PointerEvent, which: "start" | "end") {
    if (!target || ev.button !== 0) return;
    ev.preventDefault();
    ev.stopPropagation();
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    drag = { id: target.id, which, base: editor.doc, pointer: ev.pointerId };
    editor.gesture = true;
  }

  function move(ev: PointerEvent) {
    if (!drag || ev.pointerId !== drag.pointer) return;
    const to = toPage(ev);
    const page = drag.base.pages[editor.pageIndex];
    const e = page?.elements.find((o) => o.id === drag!.id);
    if (!to || !e || e.kind.type !== "Shape") return;
    const lines = snapLines(page.size, page.elements.filter((o) => o.id !== e.id).map((o) => boundsOf([o.frame])));
    const tolMm = SNAP_PX / (PX_PER_MM * zoom);
    const frame = withHitHeight(dragEnd({ frame: e.frame, flipH: !!e.flip_h, which: drag.which, to, shift: ev.shiftKey, lines, tolMm }), e.kind);
    const [s, en] = lineEndpoints(frame, !!e.flip_h);
    const tip = drag.which === "start" ? s : en;
    const on = (v: number, ls: number[]) => (ls.some((l) => Math.abs(l - v) < 1e-6) ? v : null);
    guides = ev.shiftKey ? { x: null, y: null } : { x: on(tip.x, lines.xs), y: on(tip.y, lines.ys) };
    const next = { ...e, frame };
    editor.preview({ ...drag.base, pages: drag.base.pages.map((p, i) => (i === editor.pageIndex ? { ...p, elements: p.elements.map((o) => (o.id === e.id ? next : o)) } : p)) });
  }

  function finish(keep: boolean) {
    if (!drag) return;
    if (!keep) editor.preview(drag.base);
    editor.endPreview();
    drag = null;
    guides = { x: null, y: null };
    editor.gesture = false;
  }

  $effect(() => {
    const onKey = (ev: KeyboardEvent) => {
      if (drag && ev.key === "Escape") {
        ev.stopPropagation();
        finish(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });

  /** Les poignées vivent dans la page : elles suivent son défilement et son zoom. */
  function inPage(node: HTMLElement) {
    pageEl?.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if pageEl && target && ends && !editor.proposing}
  <div class="line-handles" use:inPage>
    {#each [["start", ends[0]], ["end", ends[1]]] as const as [which, p] (which)}
      <span
        class="handle"
        class:big={settings.bigHandles}
        class:hidden={editor.gesture && !drag}
        style:left="{mmToPx(p.x, zoom)}px"
        style:top="{mmToPx(p.y, zoom)}px"
        title={fr.lineHandleHint}
        role="presentation"
        onpointerdown={(ev) => start(ev, which)}
        onpointermove={move}
        onpointerup={() => finish(true)}
        onpointercancel={() => finish(false)}
      ></span>
    {/each}
    {#if guides.x !== null}<span class="guide v" style:left="{mmToPx(guides.x, zoom)}px"></span>{/if}
    {#if guides.y !== null}<span class="guide h" style:top="{mmToPx(guides.y, zoom)}px"></span>{/if}
  </div>
{/if}

<style>
  .line-handles {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 3100;
  }
  .handle {
    position: absolute;
    width: 14px;
    height: 14px;
    margin: -7px 0 0 -7px;
    border-radius: 50%;
    background: #fff;
    border: 2px solid #4af;
    box-sizing: border-box;
    cursor: crosshair;
    pointer-events: auto;
    touch-action: none;
  }
  .handle.big {
    width: 26px;
    height: 26px;
    margin: -13px 0 0 -13px;
  }
  .handle.hidden {
    visibility: hidden;
  }
  .guide {
    position: absolute;
    background: #e91e63;
  }
  .guide.v {
    top: 0;
    bottom: 0;
    width: 1px;
  }
  .guide.h {
    left: 0;
    right: 0;
    height: 1px;
  }
</style>

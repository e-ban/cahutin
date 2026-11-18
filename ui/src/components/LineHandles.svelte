<script lang="ts">
  // Poignées d'un trait ou d'un chemin : une ronde par point (les deux bouts d'un trait droit, chaque coude d'un
  // chemin), qui se tire, et une petite au milieu de chaque segment, qui ajoute un coude quand on la tire.
  // Pendant le geste, le document est en aperçu ; il change une seule fois au relâcher (un pas d'annulation).
  // Échap, ou un geste interrompu, ne change rien. Double-clic sur un point d'un chemin : il est retiré.
  import { editor } from "../lib/docStore.svelte";
  import { removePathPoint, throughPoints } from "../lib/elementEdit";
  import { snapLines } from "../lib/geometry";
  import { boundsOf } from "../lib/groups";
  import { handleTarget } from "../lib/handleTarget";
  import { isLineTarget } from "../lib/lineEnds";
  import { MAX_PATH_POINTS, dragPoint, endDirections, insertPoint, isPath, pagePoints, segmentMids, type Pt } from "../lib/linePath";
  import { extending } from "../lib/extending.svelte";
  import { fr } from "../lib/i18n/fr";
  import { settings } from "../lib/settings.svelte";
  import { PX_PER_MM, mmToPx } from "../lib/units";
  import type { Document } from "../lib/types";

  let { pageEl, zoom }: { pageEl: HTMLElement | undefined; zoom: number } = $props();

  /** Lignes d'accroche à cette distance du point, à l'écran (comme Moveable). */
  const SNAP_PX = 6;
  /** Une poignée de milieu n'ajoute un coude qu'au-delà de ce geste (un simple clic ne change rien). */
  const MID_DEAD_PX = 3;

  const target = $derived.by(() => {
    const e = handleTarget();
    return e && isLineTarget(e) ? e : null;
  });
  const path = $derived(target?.kind.type === "Shape" && isPath(target.kind));
  const pts = $derived(target && target.kind.type === "Shape" ? pagePoints(target.frame, target.kind, !!target.flip_h, !!target.flip_v) : []);
  // À 200 points, plus de coude à ajouter : pas de poignée de milieu.
  const mids = $derived(target && target.kind.type === "Shape" && pts.length < MAX_PATH_POINTS ? segmentMids(pts, path && !!target.kind.curve) : []);

  /** Le « + » de prolongement, à 22 px de chaque bout (à côté de sa poignée), dans le prolongement du chemin. */
  const PLUS_PX = 22;
  const pluses = $derived.by(() => {
    if (!target || target.kind.type !== "Shape" || pts.length < 2) return [];
    const d = endDirections(pts, path && !!target.kind.curve);
    const mm = (settings.bigHandles ? PLUS_PX + 8 : PLUS_PX) / (PX_PER_MM * zoom);
    const at = (p: Pt, deg: number): Pt => ({ x: p.x + mm * Math.cos((deg * Math.PI) / 180), y: p.y + mm * Math.sin((deg * Math.PI) / 180) });
    return [
      { which: "start" as const, p: at(pts[0], d.start) },
      { which: "end" as const, p: at(pts[pts.length - 1], d.end) },
    ];
  });

  type Drag = { id: string; mode: "point" | "mid"; i: number; base: Document; pts0: Pt[]; pointer: number; from: { x: number; y: number }; live: boolean };
  let drag = $state<Drag | null>(null);
  /** Lignes où le point s'est accroché pendant le geste, montrées sur la page. */
  let guides = $state<{ x: number | null; y: number | null }>({ x: null, y: null });

  function toPage(ev: PointerEvent): Pt | null {
    if (!pageEl) return null;
    const r = pageEl.getBoundingClientRect();
    return { x: (ev.clientX - r.left) / (PX_PER_MM * zoom), y: (ev.clientY - r.top) / (PX_PER_MM * zoom) };
  }

  function start(ev: PointerEvent, mode: "point" | "mid", i: number) {
    if (!target || ev.button !== 0) return;
    ev.preventDefault();
    ev.stopPropagation();
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    drag = { id: target.id, mode, i, base: editor.doc, pts0: pts, pointer: ev.pointerId, from: { x: ev.clientX, y: ev.clientY }, live: mode === "point" };
    editor.gesture = true;
  }

  function move(ev: PointerEvent) {
    if (!drag || ev.pointerId !== drag.pointer) return;
    if (!drag.live && Math.hypot(ev.clientX - drag.from.x, ev.clientY - drag.from.y) < MID_DEAD_PX) return;
    drag.live = true;
    const to = toPage(ev);
    const page = drag.base.pages[editor.pageIndex];
    const e = page?.elements.find((o) => o.id === drag!.id);
    if (!to || !e || e.kind.type !== "Shape") return;
    const lines = snapLines(page.size, page.elements.filter((o) => o.id !== e.id).map((o) => boundsOf([o.frame])));
    const tolMm = SNAP_PX / (PX_PER_MM * zoom);
    // Milieu : un coude inséré après le point `i`, qui suit la souris.
    const [base, i] = drag.mode === "mid" ? [insertPoint(drag.pts0, drag.i, to), drag.i + 1] : [drag.pts0, drag.i];
    const next = dragPoint({ pts: base, i, to, shift: ev.shiftKey, lines, tolMm });
    const tip = next[i];
    const others = next.filter((_, j) => j !== i);
    const on = (v: number, ls: number[]) => (ls.some((l) => Math.abs(l - v) < 1e-6) ? v : null);
    guides = ev.shiftKey ? { x: null, y: null } : { x: on(tip.x, [...lines.xs, ...others.map((p) => p.x)]), y: on(tip.y, [...lines.ys, ...others.map((p) => p.y)]) };
    const moved = throughPoints(e, e.kind, next);
    editor.preview({ ...drag.base, pages: drag.base.pages.map((p, k) => (k === editor.pageIndex ? { ...p, elements: p.elements.map((o) => (o.id === e.id ? moved : o)) } : p)) });
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

{#if pageEl && target && pts.length && !editor.proposing}
  <div class="line-handles" use:inPage>
    {#each mids as m, i (i)}
      <span
        class="handle mid"
        class:big={settings.bigHandles}
        class:hidden={editor.gesture && !drag}
        style:left="{mmToPx(m.x, zoom)}px"
        style:top="{mmToPx(m.y, zoom)}px"
        title={fr.midHandleHint}
        role="presentation"
        onpointerdown={(ev) => start(ev, "mid", i)}
        onpointermove={move}
        onpointerup={() => finish(true)}
        onpointercancel={() => finish(false)}
      ></span>
    {/each}
    {#each pts as p, i (i)}
      <span
        class="handle"
        class:big={settings.bigHandles}
        class:hidden={editor.gesture && !drag}
        style:left="{mmToPx(p.x, zoom)}px"
        style:top="{mmToPx(p.y, zoom)}px"
        title={path ? fr.pointHandleHint : fr.lineHandleHint}
        role="presentation"
        onpointerdown={(ev) => start(ev, "point", i)}
        onpointermove={move}
        onpointerup={() => finish(true)}
        onpointercancel={() => finish(false)}
        ondblclick={(ev) => {
          ev.stopPropagation();
          if (target && path) removePathPoint(target.id, i);
        }}
      ></span>
    {/each}
    {#if !extending.active}
      {#each pluses as plus (plus.which)}
        <button
          class="plus"
          class:big={settings.bigHandles}
          class:hidden={editor.gesture}
          style:left="{mmToPx(plus.p.x, zoom)}px"
          style:top="{mmToPx(plus.p.y, zoom)}px"
          title={fr.extendPlusHint}
          aria-label={fr.extendPlusHint}
          onpointerdown={(ev) => ev.stopPropagation()}
          onclick={(ev) => {
            ev.stopPropagation();
            if (target) extending.start(target.id, plus.which);
          }}>+</button
        >
      {/each}
    {/if}
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
  /* Milieu d'un segment : plus petite et claire, pour ne pas la confondre avec un point. */
  .handle.mid {
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border: 1.5px dashed #4af;
    cursor: copy;
  }
  .handle.mid.big {
    width: 18px;
    height: 18px;
    margin: -9px 0 0 -9px;
  }
  /* « + » de prolongement : petit rond avec un plus, à côté du bout. */
  .plus {
    position: absolute;
    width: 16px;
    height: 16px;
    margin: -8px 0 0 -8px;
    padding: 0;
    border-radius: 50%;
    background: #4af;
    color: #fff;
    border: none;
    font: bold 14px/16px sans-serif;
    text-align: center;
    cursor: pointer;
    pointer-events: auto;
  }
  .plus.big {
    width: 26px;
    height: 26px;
    margin: -13px 0 0 -13px;
    font-size: 20px;
    line-height: 26px;
  }
  .plus.hidden,
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

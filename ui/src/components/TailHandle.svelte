<script lang="ts">
  // Poignée de la pointe d'une bulle de parole ou de pensée : un losange jaune sur le point visé. La tirer dirige la
  // pointe (un pas d'annulation au relâcher, Échap ou un geste interrompu ne change rien). Une ancienne bulle, à pointe
  // fixe, devient au premier geste une bulle à pointe réglable : son cadre se réduit au corps (lib/tail.ts).
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { handleTarget } from "../lib/handleTarget";
  import { settings } from "../lib/settings.svelte";
  import { LEGACY_TIP, legacyBubbleToTailed, tailOf, tailPagePoint, tipFraction } from "../lib/tail";
  import { PX_PER_MM, mmToPx } from "../lib/units";
  import type { Document } from "../lib/types";

  let { pageEl, zoom }: { pageEl: HTMLElement | undefined; zoom: number } = $props();

  /** La bulle sélectionnée seule : toute bulle de parole, ou un nuage qui a une pointe (bulle de pensée). */
  const target = $derived.by(() => {
    const e = handleTarget();
    if (!e || e.kind.type !== "Shape") return null;
    return e.kind.shape === "Bubble" || (e.kind.shape === "Cloud" && tailOf(e.kind)) ? e : null;
  });
  const tip = $derived.by(() => {
    if (!target || target.kind.type !== "Shape") return null;
    return tailPagePoint(target.frame, tailOf(target.kind) ?? LEGACY_TIP, !!target.flip_h, !!target.flip_v);
  });

  let drag = $state<{ id: string; base: Document; pointer: number } | null>(null);

  function start(ev: PointerEvent) {
    if (!target || ev.button !== 0) return;
    ev.preventDefault();
    ev.stopPropagation();
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    drag = { id: target.id, base: editor.doc, pointer: ev.pointerId };
    editor.gesture = true;
  }

  function move(ev: PointerEvent) {
    if (!drag || ev.pointerId !== drag.pointer || !pageEl) return;
    const r = pageEl.getBoundingClientRect();
    const to = { x: (ev.clientX - r.left) / (PX_PER_MM * zoom), y: (ev.clientY - r.top) / (PX_PER_MM * zoom) };
    const page = drag.base.pages[editor.pageIndex];
    const e0 = page?.elements.find((o) => o.id === drag!.id);
    if (!e0 || e0.kind.type !== "Shape") return;
    const e = tailOf(e0.kind) ? e0 : legacyBubbleToTailed(e0);
    if (e.kind.type !== "Shape") return;
    const moved = { ...e, kind: { ...e.kind, tail: tipFraction(e.frame, to, !!e.flip_h, !!e.flip_v) } };
    editor.preview({ ...drag.base, pages: drag.base.pages.map((p, k) => (k === editor.pageIndex ? { ...p, elements: p.elements.map((o) => (o.id === e.id ? moved : o)) } : p)) });
  }

  function finish(keep: boolean) {
    if (!drag) return;
    if (!keep) editor.preview(drag.base);
    editor.endPreview();
    drag = null;
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

  /** La poignée vit dans la page : elle suit son défilement et son zoom. */
  function inPage(node: HTMLElement) {
    pageEl?.appendChild(node);
    return { destroy: () => node.remove() };
  }
</script>

{#if pageEl && target && tip && !editor.proposing}
  <div class="tail-handle-layer" use:inPage>
    <span
      class="tail-handle"
      class:big={settings.bigHandles}
      class:hidden={editor.gesture && !drag}
      style:left="{mmToPx(tip.x, zoom)}px"
      style:top="{mmToPx(tip.y, zoom)}px"
      title={fr.tailHandleHint}
      role="presentation"
      onpointerdown={start}
      onpointermove={move}
      onpointerup={() => finish(true)}
      onpointercancel={() => finish(false)}
      onclick={(ev) => ev.stopPropagation()}
    ></span>
  </div>
{/if}

<style>
  .tail-handle-layer {
    position: absolute;
    inset: 0;
    pointer-events: none;
    z-index: 3100;
  }
  /* Losange jaune, comme la poignée de réglage d'une forme dans les logiciels de dessin : à part des poignées de
     taille, bleues et carrées. */
  .tail-handle {
    position: absolute;
    width: 14px;
    height: 14px;
    margin: -7px 0 0 -7px;
    transform: rotate(45deg);
    background: #ffd23f;
    border: 2px solid #8a6d00;
    box-sizing: border-box;
    cursor: move;
    pointer-events: auto;
    touch-action: none;
  }
  .tail-handle.big {
    width: 24px;
    height: 24px;
    margin: -12px 0 0 -12px;
  }
  .tail-handle.hidden {
    visibility: hidden;
  }
</style>

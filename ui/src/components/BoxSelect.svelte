<script lang="ts">
  // Sélection au cadre : glisser sur la page vide ou la vue autour sélectionne les éléments entièrement couverts
  // (un élément groupé sélectionne tout son groupe) ; Maj ajoute à la sélection.
  import { editor } from "../lib/docStore.svelte";
  import { elementsInRect, expandSelection } from "../lib/groups";
  import { PX_PER_MM } from "../lib/units";

  let { pageEl, stageEl, zoom }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined; zoom: number } = $props();

  const THRESHOLD_PX = 4;
  let start: { x: number; y: number; add: boolean } | null = null;
  let box = $state<{ left: number; top: number; width: number; height: number } | null>(null);

  function onDown(ev: PointerEvent) {
    if (ev.button !== 0 || (ev.target !== pageEl && ev.target !== stageEl) || editor.cropping || editor.editingText) return;
    start = { x: ev.clientX, y: ev.clientY, add: ev.shiftKey };
  }

  function onMove(ev: PointerEvent) {
    if (!start) return;
    const [x0, x1] = [start.x, ev.clientX].sort((a, b) => a - b);
    const [y0, y1] = [start.y, ev.clientY].sort((a, b) => a - b);
    if (!box && x1 - x0 < THRESHOLD_PX && y1 - y0 < THRESHOLD_PX) return;
    box = { left: x0, top: y0, width: x1 - x0, height: y1 - y0 };
  }

  function onUp() {
    const [s, b] = [start, box];
    start = null;
    box = null;
    if (!s || !b || !pageEl) return;
    const r = pageEl.getBoundingClientRect();
    const k = PX_PER_MM * zoom;
    const ids = elementsInRect(editor.page, { x_mm: (b.left - r.left) / k, y_mm: (b.top - r.top) / k, w_mm: b.width / k, h_mm: b.height / k });
    editor.focusedMember = null;
    editor.selection = expandSelection(editor.page, s.add ? [...editor.selection, ...ids] : ids);
  }

  $effect(() => {
    const els = [pageEl, stageEl].filter((e): e is HTMLElement => !!e);
    for (const el of els) el.addEventListener("pointerdown", onDown);
    return () => els.forEach((el) => el.removeEventListener("pointerdown", onDown));
  });
</script>

<svelte:window onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp} />

{#if box}
  <div class="box" style:left="{box.left}px" style:top="{box.top}px" style:width="{box.width}px" style:height="{box.height}px"></div>
{/if}

<style>
  .box {
    position: fixed;
    border: 1px solid var(--accent);
    background: rgba(46, 134, 222, 0.1);
    pointer-events: none;
    z-index: 3200;
  }
</style>

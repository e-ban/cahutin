<script lang="ts">
  import PhotoEl from "./elements/PhotoEl.svelte";
  import ShapeEl from "./elements/ShapeEl.svelte";
  import TextEl from "./elements/TextEl.svelte";
  import { dragCrop, wheelCrop } from "../lib/crop";
  import { editor } from "../lib/docStore.svelte";
  import { ASSET_MIME } from "../lib/dnd";
  import { PX_PER_MM, mmToPx } from "../lib/units";
  import type { Crop, Document, Element, Page, RichText } from "../lib/types";

  let {
    page,
    doc,
    zoom,
    selection = [],
    cropping = null,
    editingText = null,
    templateMode = false,
    onselect,
    ondropasset,
    oncropstart,
    oncropend,
    onedittext,
    ontextdone,
    oncontext,
    pageEl = $bindable(),
  }: {
    page: Page;
    doc: Document;
    zoom: number;
    selection?: string[];
    cropping?: string | null;
    editingText?: string | null;
    templateMode?: boolean;
    onselect?: (id: string | null, add: boolean) => void;
    ondropasset?: (assetId: string, elementId: string | null, xMm: number, yMm: number) => void;
    oncropstart?: (id: string) => void;
    oncropend?: (id: string, crop: Crop) => void;
    onedittext?: (id: string) => void;
    ontextdone?: (id: string, content: RichText) => void;
    /** Clic droit sur un élément (id) ou sur la page (null), avec la position en mm sur la page. */
    oncontext?: (id: string | null, xMm: number, yMm: number, clientX: number, clientY: number) => void;
    pageEl?: HTMLElement;
  } = $props();

  let liveCrop = $state<Crop | null>(null);
  let dragFrom: { x: number; y: number } | null = null;

  function frameStyle(e: Element): string {
    const f = e.frame;
    return `left:${mmToPx(f.x_mm, zoom)}px;top:${mmToPx(f.y_mm, zoom)}px;width:${mmToPx(f.w_mm, zoom)}px;height:${mmToPx(f.h_mm, zoom)}px;transform:rotate(${f.rotation_deg}deg)`;
  }

  function acceptsAsset(ev: DragEvent): boolean {
    return !!ev.dataTransfer?.types.includes(ASSET_MIME);
  }

  function drop(ev: DragEvent, elementId: string | null) {
    if (!acceptsAsset(ev) || !pageEl) return;
    ev.preventDefault();
    ev.stopPropagation();
    const id = ev.dataTransfer!.getData(ASSET_MIME);
    const r = pageEl.getBoundingClientRect();
    ondropasset?.(id, elementId, (ev.clientX - r.left) / (PX_PER_MM * zoom), (ev.clientY - r.top) / (PX_PER_MM * zoom));
  }

  const croppedElement = $derived(cropping ? page.elements.find((e) => e.id === cropping) : undefined);

  function endCrop() {
    const e = croppedElement;
    if (e && e.kind.type === "Photo" && liveCrop) oncropend?.(e.id, liveCrop);
    else if (e) oncropend?.(e.id, e.kind.type === "Photo" ? e.kind.crop : { scale: 1, center_x: 0.5, center_y: 0.5 });
    liveCrop = null;
  }

  // Enregistrer ou fermer pendant un recadrage : le recadrage en cours est validé d'abord.
  $effect(() => {
    if (!cropping || !oncropend) return;
    return editor.onFlush(endCrop);
  });

  function cropOf(e: Element): Crop | undefined {
    return e.id === cropping && liveCrop ? liveCrop : undefined;
  }
</script>

<svelte:window
  onkeydown={(ev) => {
    if (cropping && (ev.key === "Escape" || ev.key === "Enter")) endCrop();
  }}
/>

<div
  class="page"
  class:template={templateMode}
  bind:this={pageEl}
  role="presentation"
  style:width="{mmToPx(page.size.width_mm, zoom)}px"
  style:height="{mmToPx(page.size.height_mm, zoom)}px"
  onpointerdown={(ev) => {
    if (cropping) endCrop();
    if (ev.target === ev.currentTarget) onselect?.(null, false);
  }}
  ondragover={(ev) => {
    if (acceptsAsset(ev)) ev.preventDefault();
  }}
  ondrop={(ev) => drop(ev, null)}
  oncontextmenu={(ev) => {
    // Pendant l'écriture, le menu du système reste (correcteur orthographique).
    if (!oncontext || !pageEl || (ev.target as HTMLElement).isContentEditable) return;
    ev.preventDefault();
    const r = pageEl.getBoundingClientRect();
    const id = (ev.target as HTMLElement).closest<HTMLElement>(".element")?.dataset.id ?? null;
    oncontext(id, (ev.clientX - r.left) / (PX_PER_MM * zoom), (ev.clientY - r.top) / (PX_PER_MM * zoom), ev.clientX, ev.clientY);
  }}
>
  {#each page.elements as e (e.id)}
    <div
      class="element"
      class:selected={selection.includes(e.id)}
      class:locked={e.locked}
      class:cropping={e.id === cropping}
      class:zone-photo={e.zone && e.kind.type === "Photo"}
      class:zone-text={e.zone && e.kind.type === "Text"}
      class:merge-ok={templateMode && editor.mergePreview?.ok && editor.mergePreview.ids.includes(e.id)}
      class:merge-bad={templateMode && editor.mergePreview && !editor.mergePreview.ok && editor.mergePreview.ids.includes(e.id)}
      data-id={e.id}
      role="presentation"
      style={frameStyle(e)}
      onpointerdown={(ev) => {
        if (e.id === cropping) {
          ev.stopPropagation();
          dragFrom = { x: ev.clientX, y: ev.clientY };
          (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
          return;
        }
        onselect?.(e.id, ev.shiftKey);
      }}
      onpointermove={(ev) => {
        if (e.id !== cropping || !dragFrom || e.kind.type !== "Photo" || !e.kind.asset) return;
        const meta = doc.assets[e.kind.asset] ?? editor.assetMeta(e.kind.asset);
        if (!meta) return;
        liveCrop = dragCrop(liveCrop ?? e.kind.crop, ev.clientX - dragFrom.x, ev.clientY - dragFrom.y, meta, e.frame, zoom);
        dragFrom = { x: ev.clientX, y: ev.clientY };
      }}
      onpointerup={() => (dragFrom = null)}
      onwheel={(ev) => {
        if (e.id !== cropping || e.kind.type !== "Photo") return;
        ev.preventDefault();
        liveCrop = wheelCrop(liveCrop ?? e.kind.crop, ev.deltaY);
      }}
      ondblclick={() => {
        if (e.kind.type === "Photo" && e.kind.asset && !e.locked) {
          liveCrop = null;
          oncropstart?.(e.id);
        } else if (e.kind.type === "Text" && !e.locked) {
          onedittext?.(e.id);
        }
      }}
      ondragover={(ev) => {
        if (acceptsAsset(ev) && e.kind.type === "Photo" && !e.locked) {
          ev.preventDefault();
          ev.stopPropagation();
        }
      }}
      ondrop={(ev) => {
        if (e.kind.type === "Photo" && !e.locked) drop(ev, e.id);
      }}
    >
      {#if e.kind.type === "Photo"}
        <PhotoEl
          element={e}
          props={cropOf(e) ? { ...e.kind, crop: cropOf(e)! } : e.kind}
          meta={e.kind.asset ? (doc.assets[e.kind.asset] ?? editor.assetMeta(e.kind.asset)) : undefined}
          {zoom}
        />
      {:else if e.kind.type === "Text"}
        <TextEl element={e} props={e.kind} {zoom} editing={e.id === editingText} ondone={(content) => ontextdone?.(e.id, content)} />
      {:else}
        <ShapeEl props={e.kind} {zoom} />
      {/if}
    </div>
  {/each}
</div>

<style>
  .page {
    position: relative;
    background: white;
    box-shadow: 0 2px 14px rgba(60, 40, 20, 0.18);
    flex: none;
  }
  .element {
    position: absolute;
    transform-origin: center;
  }
  .element.merge-ok {
    outline: 3px solid var(--accent);
    background: rgba(46, 134, 222, 0.18);
  }
  .element.merge-bad {
    outline: 3px solid var(--accent-strong);
    background: rgba(192, 57, 43, 0.15);
  }
  .element.selected {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .element:has(.editor) {
    z-index: 10;
  }
  .page.template .element.zone-photo {
    outline: 2px dashed var(--zone-photo);
    outline-offset: -1px;
  }
  .page.template .element.zone-text {
    outline: 2px dashed var(--zone-text);
    outline-offset: -1px;
  }
  .page.template .element.selected {
    outline: 3px solid var(--accent);
  }
  .element.cropping {
    outline: 3px solid #f39c12;
    cursor: move;
    z-index: 10;
  }
</style>

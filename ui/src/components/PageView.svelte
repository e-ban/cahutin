<script lang="ts">
  import PhotoEl from "./elements/PhotoEl.svelte";
  import ShapeEl from "./elements/ShapeEl.svelte";
  import TextEl from "./elements/TextEl.svelte";
  import { mmToPx } from "../lib/units";
  import type { Document, Element, Page } from "../lib/types";

  let {
    page,
    doc,
    zoom,
    selection = [],
    onselect,
    pageEl = $bindable(),
  }: {
    page: Page;
    doc: Document;
    zoom: number;
    selection?: string[];
    onselect?: (id: string | null, add: boolean) => void;
    pageEl?: HTMLElement;
  } = $props();

  function frameStyle(e: Element): string {
    const f = e.frame;
    return `left:${mmToPx(f.x_mm, zoom)}px;top:${mmToPx(f.y_mm, zoom)}px;width:${mmToPx(f.w_mm, zoom)}px;height:${mmToPx(f.h_mm, zoom)}px;transform:rotate(${f.rotation_deg}deg)`;
  }
</script>

<div
  class="page"
  bind:this={pageEl}
  role="presentation"
  style:width="{mmToPx(page.size.width_mm, zoom)}px"
  style:height="{mmToPx(page.size.height_mm, zoom)}px"
  onpointerdown={(ev) => {
    if (ev.target === ev.currentTarget) onselect?.(null, false);
  }}
>
  {#each page.elements as e (e.id)}
    <div
      class="element"
      class:selected={selection.includes(e.id)}
      class:locked={e.locked}
      data-id={e.id}
      role="presentation"
      style={frameStyle(e)}
      onpointerdown={(ev) => onselect?.(e.id, ev.shiftKey)}
    >
      {#if e.kind.type === "Photo"}
        <PhotoEl element={e} props={e.kind} meta={e.kind.asset ? doc.assets[e.kind.asset] : undefined} {zoom} />
      {:else if e.kind.type === "Text"}
        <TextEl element={e} props={e.kind} {zoom} />
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
  .element.selected {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
</style>

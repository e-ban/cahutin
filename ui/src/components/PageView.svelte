<script lang="ts">
  import PhotoEl from "./elements/PhotoEl.svelte";
  import ShapeEl from "./elements/ShapeEl.svelte";
  import TextEl from "./elements/TextEl.svelte";
  import { dragCrop, wheelCrop } from "../lib/crop";
  import { editor } from "../lib/docStore.svelte";
  import { ASSET_MIME, PICTO_MIME } from "../lib/dnd";
  import { fr } from "../lib/i18n/fr";
  import { pageHasPictos } from "../lib/pictos.svelte";
  import { TEXT_PADDING_MM } from "../lib/richtext";
  import PictoEl from "./elements/PictoEl.svelte";
  import { marks } from "../lib/background";
  import { CREDIT_SIZE_PT, creditFrame } from "../lib/decor";
  import { PX_PER_MM, mmToPx, ptToPx } from "../lib/units";
  import type { Crop, Document, Element, Frame, Page, RichText } from "../lib/types";

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
    onfocusmember,
    ondroppicto,
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
    /** Double-clic sur un élément groupé : il est modifié seul. */
    onfocusmember?: (id: string) => void;
    /** Image de la banque lâchée sur la page, au point (mm). */
    ondroppicto?: (code: string, xMm: number, yMm: number) => void;
    pageEl?: HTMLElement;
  } = $props();

  let liveCrop = $state<Crop | null>(null);
  let dragFrom: { x: number; y: number } | null = null;

  /** Double-clic : un élément groupé est modifié seul ; une photo se recadre, un texte s'écrit. */
  function activate(e: Element) {
    if (e.group) onfocusmember?.(e.id);
    if (e.kind.type === "Photo" && e.kind.asset && !e.locked) {
      liveCrop = null;
      oncropstart?.(e.id);
    } else if (e.kind.type === "Text" && !e.locked) {
      onedittext?.(e.id);
    }
  }

  /** Élément de cette page sous le pointeur, même sous la zone des poignées d'un groupe. */
  function elementUnder(x: number, y: number): string | null {
    const n = document.elementsFromPoint(x, y).map((el) => el.closest<HTMLElement>(".element")).find((el) => el && pageEl?.contains(el));
    return n?.dataset.id ?? null;
  }

  // Forme vide (sans remplissage) : seul son contour se clique, l'intérieur laisse passer vers la page.
  const isHollow = (e: Element) => e.kind.type === "Shape" && e.kind.fill === null && e.kind.shape !== "Line" && e.kind.shape !== "Arrow";

  function frameStyle(e: Element): string {
    return rectStyle(e.frame);
  }

  function rectStyle(f: Frame): string {
    return `left:${mmToPx(f.x_mm, zoom)}px;top:${mmToPx(f.y_mm, zoom)}px;width:${mmToPx(f.w_mm, zoom)}px;height:${mmToPx(f.h_mm, zoom)}px;transform:rotate(${f.rotation_deg}deg)`;
  }

  function acceptsAsset(ev: DragEvent): boolean {
    return !!ev.dataTransfer?.types.includes(ASSET_MIME) || !!ev.dataTransfer?.types.includes(PICTO_MIME);
  }

  function drop(ev: DragEvent, elementId: string | null) {
    if (!acceptsAsset(ev) || !pageEl) return;
    ev.preventDefault();
    ev.stopPropagation();
    const r = pageEl.getBoundingClientRect();
    const [xMm, yMm] = [(ev.clientX - r.left) / (PX_PER_MM * zoom), (ev.clientY - r.top) / (PX_PER_MM * zoom)];
    const picto = ev.dataTransfer!.getData(PICTO_MIME);
    if (picto) {
      ondroppicto?.(picto, xMm, yMm);
      return;
    }
    ondropasset?.(ev.dataTransfer!.getData(ASSET_MIME), elementId, xMm, yMm);
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
  style:background={page.background?.color ?? "white"}
  onpointerdown={(ev) => {
    if (cropping) endCrop();
    // Maj : la sélection au cadre ajoutera à la sélection actuelle.
    if (ev.target === ev.currentTarget && !ev.shiftKey) onselect?.(null, false);
  }}
  ondblclick={(ev) => {
    // Double-clic sur la zone des poignées d'un groupe (pas sur un élément) : l'élément sous le pointeur.
    if ((ev.target as HTMLElement).closest(".element")) return;
    const id = elementUnder(ev.clientX, ev.clientY);
    const e = id ? page.elements.find((x) => x.id === id) : undefined;
    if (e) activate(e);
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
    // Sous la zone des poignées d'un groupe, l'élément est celui qui se trouve sous le pointeur.
    const id = elementUnder(ev.clientX, ev.clientY);
    oncontext(id, (ev.clientX - r.left) / (PX_PER_MM * zoom), (ev.clientY - r.top) / (PX_PER_MM * zoom), ev.clientX, ev.clientY);
  }}
>
  {#if page.background?.pattern}
    <!-- Motif d'école : même géométrie que l'ODG (lib/background.ts), derrière tout, jamais cliquable. -->
    <svg class="pattern" viewBox="0 0 {page.size.width_mm} {page.size.height_mm}" aria-hidden="true">
      {#each marks(page.size, page.background.pattern) as m, i (i)}
        {#if m.type === "Line"}
          <line x1={m.x1} y1={m.y1} x2={m.x2} y2={m.y2} stroke={m.color} stroke-width={m.width_mm} />
        {:else}
          <circle cx={m.x} cy={m.y} r={m.d_mm / 2} fill={m.color} />
        {/if}
      {/each}
    </svg>
  {/if}
  {#each page.elements as e (e.id)}
    <div
      class="element"
      class:selected={selection.includes(e.id)}
      class:locked={e.locked}
      class:cropping={e.id === cropping}
      class:zone-photo={e.zone && e.kind.type === "Photo"}
      class:zone-text={e.zone && e.kind.type === "Text"}
      class:hollow={isHollow(e)}
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
      ondblclick={() => activate(e)}
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
      {:else if e.kind.type === "Image"}
        <PictoEl code={e.kind.code} />
      {:else if e.kind.type === "Shape"}
        <ShapeEl props={e.kind} frame={e.frame} {zoom} />
      {/if}
    </div>
  {/each}
  {#if pageHasPictos(page)}
    {@const f = creditFrame(page.size)}
    <!-- Mention de licence OpenMoji : même cadre et même police que dans l'ODG, aussi à l'impression. -->
    <div class="credit" aria-hidden="true" style={rectStyle(f)} style:font-size="{ptToPx(CREDIT_SIZE_PT, zoom)}px" style:padding="{mmToPx(TEXT_PADDING_MM, zoom)}px">
      {fr.openmojiCreditPage}
    </div>
  {/if}
</div>

<style>
  .page {
    position: relative;
    background: white;
    box-shadow: 0 2px 14px rgba(60, 40, 20, 0.18);
    flex: none;
  }
  .pattern {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .credit {
    position: absolute;
    box-sizing: border-box;
    font-family: "Andika";
    color: #888888;
    text-align: center;
    line-height: normal;
    pointer-events: none;
    user-select: none;
  }
  .element {
    position: absolute;
    transform-origin: center;
  }
  .element.hollow {
    pointer-events: none;
  }
  .element.hollow.selected,
  .element.hollow.locked {
    pointer-events: none;
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

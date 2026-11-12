<script lang="ts">
  import PhotoEl from "./elements/PhotoEl.svelte";
  import ShapeEl from "./elements/ShapeEl.svelte";
  import GraphismeGhosts from "./GraphismeGhosts.svelte";
  import TableEl from "./elements/TableEl.svelte";
  import TextEl from "./elements/TextEl.svelte";
  import { dragCrop, photoTrims, wheelCrop, withTrims, type Trims } from "../lib/crop";
  import { editor } from "../lib/docStore.svelte";
  import { ASSET_MIME, CLASS_PHOTO_MIME, MY_IMAGE_MIME, PICTO_MIME } from "../lib/dnd";
  import { fr } from "../lib/i18n/fr";
  import { pageHasPictos } from "../lib/pictos.svelte";
  import { TEXT_PADDING_MM } from "../lib/richtext";
  import PictoEl from "./elements/PictoEl.svelte";
  import { marks } from "../lib/background";
  import { isCutMark } from "../lib/cutMarks";
  import { CREDIT_SIZE_PT, creditFrame, isPageFrame } from "../lib/decor";
  import { PX_PER_MM, mmToPx, ptToPx } from "../lib/units";
  import { settings } from "../lib/settings.svelte";
  import { isPlacedCell } from "../lib/table";
  import { IS_LINUX, resizeCursor } from "../lib/cursors";
  import { pixelRatio } from "../lib/pixelRatio.svelte";
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
    ondropmyimage,
    ondropclassphoto,
    oncropstart,
    oncropend,
    oncrop,
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
    /** Une image de la collection glissée : son nom de fichier. */
    ondropmyimage?: (name: string, elementId: string | null, xMm: number, yMm: number) => void;
    /** Une photo de la photothèque de la classe glissée : son identifiant. */
    ondropclassphoto?: (id: string, elementId: string | null, xMm: number, yMm: number) => void;
    oncropstart?: (id: string) => void;
    /** Sortie du recadrage, une fois le dernier geste enregistré par `oncrop`. */
    oncropend?: (id: string) => void;
    /** Fin d'un geste du recadrage (glisser, molette, poignée de rognage) : le cadre et le recadrage. */
    oncrop?: (id: string, frame: Frame, crop: Crop) => void;
    onedittext?: (id: string) => void;
    ontextdone?: (id: string, content: RichText) => void;
    /** Clic droit sur un élément (id) ou sur la page (null), avec la position en mm sur la page. */
    oncontext?: (id: string | null, xMm: number, yMm: number, clientX: number, clientY: number) => void;
    /** Double-clic sur un élément groupé : il est modifié seul. */
    onfocusmember?: (id: string) => boolean;
    /** Image de la banque lâchée sur la page, au point (mm). */
    ondroppicto?: (code: string, xMm: number, yMm: number) => void;
    pageEl?: HTMLElement;
  } = $props();

  let liveCrop = $state<Crop | null>(null);
  // Rognage en cours : le cadre bouge avec la poignée, la photo reste en place.
  let liveFrame = $state<Frame | null>(null);
  let dragFrom: { x: number; y: number } | null = null;
  let wheelTimer: ReturnType<typeof setTimeout> | null = null;
  /** Molette : enregistrée après une courte pause, en un seul pas d'annulation. */
  const WHEEL_COMMIT_MS = 400;

  type Handle = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";
  const HANDLES: Handle[] = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
  let trimFrom: { handle: Handle; x: number; y: number; frame: Frame; crop: Crop; trims: Trims } | null = null;

  /** Double-clic : un élément groupé est modifié seul ; une photo se recadre, un texte s'écrit. */
  function activate(e: Element) {
    // Dans un groupe de groupes, le double-clic entre d'abord d'un niveau, sans recadrer ni écrire.
    if (e.group && onfocusmember?.(e.id) === false) return;
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

  // Forme vide (sans remplissage) : sa boîte laisse passer les clics, ShapeEl dessine la surface qui les reçoit.
  // Un tableau laisse passer tous les clics vers ses cases.
  const isHollow = (e: Element) => e.kind.type === "Table" || (e.kind.type === "Shape" && e.kind.fill === null && e.kind.shape !== "Line" && e.kind.shape !== "Arrow");

  /** Une forme vide se clique dans tout son intérieur, comme une pleine : l'absence de fond n'est qu'une
   *  apparence. Sauf ce qui entoure d'autres éléments (cadre de page, repères de découpe) et les tracés de
   *  graphisme, qui ne se cliquent que sur leur trait. */
  const solidInside = (e: Element) => e.kind.type === "Shape" && e.kind.shape !== "Path" && !isCutMark(e) && !isPageFrame(e, page.size);

  function frameStyle(e: Element): string {
    return rectStyle(e.frame);
  }

  function rectStyle(f: Frame): string {
    return `left:${mmToPx(f.x_mm, zoom)}px;top:${mmToPx(f.y_mm, zoom)}px;width:${mmToPx(f.w_mm, zoom)}px;height:${mmToPx(f.h_mm, zoom)}px;transform:rotate(${f.rotation_deg}deg)`;
  }

  function acceptsAsset(ev: DragEvent): boolean {
    return [ASSET_MIME, PICTO_MIME, MY_IMAGE_MIME, CLASS_PHOTO_MIME].some((t) => ev.dataTransfer?.types.includes(t));
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
    const mine = ev.dataTransfer!.getData(MY_IMAGE_MIME);
    if (mine) {
      ondropmyimage?.(mine, elementId, xMm, yMm);
      return;
    }
    const classPhoto = ev.dataTransfer!.getData(CLASS_PHOTO_MIME);
    if (classPhoto) {
      ondropclassphoto?.(classPhoto, elementId, xMm, yMm);
      return;
    }
    ondropasset?.(ev.dataTransfer!.getData(ASSET_MIME), elementId, xMm, yMm);
  }

  const croppedElement = $derived(cropping ? page.elements.find((e) => e.id === cropping) : undefined);

  function endCrop() {
    if (wheelTimer) clearTimeout(wheelTimer);
    wheelTimer = null;
    trimFrom = null;
    const e = croppedElement;
    if (e) {
      commitLive(e);
      oncropend?.(e.id);
    }
    liveCrop = null;
    liveFrame = null;
  }

  /** Fin d'un geste : le recadrage en cours rejoint le document (un pas d'annulation). */
  function commitLive(e: Element) {
    if (wheelTimer) clearTimeout(wheelTimer);
    wheelTimer = null;
    if (e.kind.type !== "Photo" || !oncrop || (!liveCrop && !liveFrame)) return;
    oncrop(e.id, liveFrame ?? e.frame, liveCrop ?? e.kind.crop);
    liveCrop = null;
    liveFrame = null;
  }

  /** Photo retournée à l'écran : miroir de la photo et retournement de l'élément se combinent. */
  const flips = (e: Element) => [e.kind.type === "Photo" && !!e.kind.mirrored !== !!e.flip_h, !!e.flip_v] as const;

  /** Les bords d'une photo se rognent, sauf dans une case de grille (la grille la place) ou verrouillée. */
  const trimmable = (e: Element) => e.kind.type === "Photo" && !!e.kind.asset && (!e.locked || templateMode) && !isPlacedCell(page, e.id);

  function startTrim(ev: PointerEvent, e: Element, handle: Handle) {
    if (e.kind.type !== "Photo" || !e.kind.asset) return;
    const meta = doc.assets[e.kind.asset] ?? editor.assetMeta(e.kind.asset);
    if (!meta) return;
    ev.stopPropagation();
    (ev.currentTarget as HTMLElement).setPointerCapture(ev.pointerId);
    // Lus avant d'enregistrer la molette en attente : après, `e` peut ne pas l'avoir encore reçue.
    const frame = liveFrame ?? e.frame;
    const crop = liveCrop ?? e.kind.crop;
    if (wheelTimer) commitLive(e);
    trimFrom = { handle, x: ev.clientX, y: ev.clientY, frame, crop, trims: photoTrims(meta, frame, crop, ...flips(e)) };
  }

  function moveTrim(ev: PointerEvent, e: Element) {
    const t = trimFrom;
    if (!t || e.kind.type !== "Photo" || !e.kind.asset) return;
    const meta = doc.assets[e.kind.asset] ?? editor.assetMeta(e.kind.asset);
    if (!meta) return;
    // Le geste à l'écran, ramené dans le cadre tourné, en mm.
    const a = (t.frame.rotation_deg * Math.PI) / 180;
    const dx = ev.clientX - t.x;
    const dy = ev.clientY - t.y;
    const k = PX_PER_MM * zoom;
    const lx = (dx * Math.cos(a) + dy * Math.sin(a)) / k;
    const ly = (-dx * Math.sin(a) + dy * Math.cos(a)) / k;
    const h = t.handle;
    const trims = {
      left: t.trims.left + (h.includes("w") ? lx : 0),
      right: t.trims.right - (h.includes("e") ? lx : 0),
      top: t.trims.top + (h.includes("n") ? ly : 0),
      bottom: t.trims.bottom - (h.includes("s") ? ly : 0),
    };
    const r = withTrims(meta, t.frame, t.crop, trims, ...flips(e));
    liveFrame = r.frame;
    liveCrop = r.crop;
  }

  function endTrim(e: Element) {
    if (!trimFrom) return;
    trimFrom = null;
    commitLive(e);
  }

  // Enregistrer ou fermer pendant un recadrage : le recadrage en cours est validé d'abord.
  $effect(() => {
    if (!cropping || !oncropend) return;
    return editor.onFlush(endCrop);
  });

  function cropOf(e: Element): Crop | undefined {
    return e.id === cropping && liveCrop ? liveCrop : undefined;
  }

  /** L'élément tel qu'on le voit, avec le cadre du rognage en cours. */
  const shown = (e: Element): Element => (e.id === cropping && liveFrame ? { ...e, frame: liveFrame } : e);
</script>

{#snippet content(e: Element)}
  {#if e.kind.type === "Photo"}
    <PhotoEl
      element={shown(e)}
      ghost={e.id === cropping}
      props={cropOf(e) ? { ...e.kind, crop: cropOf(e)! } : e.kind}
      meta={e.kind.asset ? (doc.assets[e.kind.asset] ?? editor.assetMeta(e.kind.asset)) : undefined}
      {zoom}
    />
  {:else if e.kind.type === "Text"}
    <!-- Une page qu'on regarde sans la modifier (aperçus, miniatures) ne montre pas « Texte trop long ». -->
    <TextEl element={e} props={e.kind} {zoom} editing={e.id === editingText} alerts={!!onselect} ondone={(content) => ontextdone?.(e.id, content)} />
  {:else if e.kind.type === "Image"}
    <PictoEl code={e.kind.code} outline={e.kind.outline === true} />
  {:else if e.kind.type === "Shape"}
    <ShapeEl props={e.kind} frame={e.frame} {zoom} solid={solidInside(e)} />
  {:else if e.kind.type === "Table"}
    <TableEl props={e.kind} frame={e.frame} {zoom} />
  {/if}
{/snippet}


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
      style={frameStyle(shown(e))}
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
        liveCrop = dragCrop(liveCrop ?? e.kind.crop, ev.clientX - dragFrom.x, ev.clientY - dragFrom.y, meta, e.frame, zoom, !!e.kind.mirrored, !!e.flip_v);
        dragFrom = { x: ev.clientX, y: ev.clientY };
      }}
      onpointerup={() => {
        if (!dragFrom) return;
        dragFrom = null;
        commitLive(e);
      }}
      onwheel={(ev) => {
        // Ctrl+molette zoome la page, même pendant le recadrage.
        if (e.id !== cropping || e.kind.type !== "Photo" || ev.ctrlKey || ev.metaKey || trimFrom) return;
        ev.preventDefault();
        liveCrop = wheelCrop(liveCrop ?? e.kind.crop, ev.deltaY);
        if (wheelTimer) clearTimeout(wheelTimer);
        wheelTimer = setTimeout(() => commitLive(e), WHEEL_COMMIT_MS);
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
      <!-- En miroir : le dessin est retourné dans son cadre (sous la rotation) ; un texte ne l'est jamais. -->
      {#if (e.flip_h || e.flip_v) && e.kind.type !== "Text" && e.kind.type !== "Table"}
        <div class="flip" style:transform="scale({e.flip_h ? -1 : 1}, {e.flip_v ? -1 : 1})">{@render content(e)}</div>
      {:else}
        {@render content(e)}
      {/if}
      {#if e.id === cropping && trimmable(e)}
        <!-- Poignées de rognage : chacune déplace un bord (ou deux), la photo reste en place. -->
        {#each HANDLES as h (h)}
          <span
            class="trim {h}"
            class:big={settings.bigHandles}
            style:cursor={resizeCursor(h, shown(e).frame.rotation_deg, IS_LINUX, pixelRatio.value)}
            role="presentation"
            onpointerdown={(ev) => startTrim(ev, e, h)}
            onpointermove={(ev) => moveTrim(ev, e)}
            onpointerup={() => endTrim(e)}
            onpointercancel={() => endTrim(e)}
          ></span>
        {/each}
      {/if}
    </div>
  {/each}
  {#if selection.length}<GraphismeGhosts {page} {selection} {zoom} />{/if}
  {#if pageHasPictos(page)}
    {@const f = creditFrame(page.size)}
    <!-- Mention de licence OpenMoji : même cadre et même police que dans l'ODG, aussi à l'impression. -->
    <div class="credit" aria-hidden="true" style={rectStyle(f)} style:font-size="{ptToPx(CREDIT_SIZE_PT, zoom)}px" style:padding="{mmToPx(TEXT_PADDING_MM, zoom)}px">
      {fr.openmojiCreditPage}
    </div>
  {/if}
</div>

<style>
  .flip {
    position: absolute;
    inset: 0;
  }
  /* La page reste blanche, comme sur papier, quel que soit le thème : ses repères gardent les couleurs claires. */
  .page {
    --text: #2d2a26;
    --muted: #7d756b;
    --line: #e2d9cb;
    --panel: #fbf8f3;
    --surface: #ffffff;
    --accent: #2e86de;
    --accent-soft: #e8f1fb;
    color-scheme: light;
    color: var(--text);
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
  /* Repères doublés (Préférences) : la fusion possible ou non se lit aussi à un signe dans le coin, pas seulement
     à la couleur. */
  :global(:root[data-cues]) .element.merge-ok::after,
  :global(:root[data-cues]) .element.merge-bad::after {
    position: absolute;
    top: 4px;
    right: 8px;
    font: 700 28px var(--ui-font);
    pointer-events: none;
    z-index: 5;
  }
  :global(:root[data-cues]) .element.merge-ok::after {
    content: "✓";
    color: var(--accent);
  }
  :global(:root[data-cues]) .element.merge-bad::after {
    content: "✗";
    color: var(--accent-strong);
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
  /* Repères doublés : une zone de texte en pointillés ronds, une zone photo en tirets. */
  :global(:root[data-cues]) .page.template .element.zone-text {
    outline-style: dotted;
  }
  .page.template .element.selected {
    outline: 3px solid var(--accent);
  }
  .element.cropping {
    outline: 3px solid #f39c12;
    cursor: move;
    z-index: 10;
  }
  .trim {
    --size: 12px;
    position: absolute;
    width: var(--size);
    height: var(--size);
    margin: calc(var(--size) / -2);
    background: #f39c12;
    border: 2px solid white;
    border-radius: 2px;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
    z-index: 1;
    touch-action: none;
  }
  .trim.big {
    --size: 22px;
  }
  .trim.n,
  .trim.s {
    left: 50%;
    width: calc(var(--size) * 2);
    margin-left: calc(var(--size) * -1);
  }
  .trim.e,
  .trim.w {
    top: 50%;
    height: calc(var(--size) * 2);
    margin-top: calc(var(--size) * -1);
  }
  .trim.n,
  .trim.nw,
  .trim.ne {
    top: 0;
  }
  .trim.s,
  .trim.sw,
  .trim.se {
    top: 100%;
  }
  .trim.w,
  .trim.nw,
  .trim.sw {
    left: 0;
  }
  .trim.e,
  .trim.ne,
  .trim.se {
    left: 100%;
  }

</style>

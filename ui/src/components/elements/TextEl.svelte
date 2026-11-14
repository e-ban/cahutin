<script lang="ts">
  // Le texte est mis en page à sa taille réelle (mm et pt), puis mis à l'échelle du zoom :
  // les retours à la ligne ne dépendent pas du zoom et correspondent à ceux de l'export.
  import TextEditor from "../TextEditor.svelte";
  import { frameMarks } from "../../lib/background";
  import { assetUrl } from "../../lib/api";
  import { fieldLabel, fieldShown, splitFields } from "../../lib/classFields";
  import { alignToCss, textPadding } from "../../lib/richtext";
  import { dashArray } from "../../lib/strokes";
  import { onFontsLoaded } from "../../lib/fontsLoaded";
  import { editor } from "../../lib/docStore.svelte";
  import { handlePadding } from "../../lib/geometry";
  import { fr } from "../../lib/i18n/fr";
  import { settings } from "../../lib/settings.svelte";
  import { actionsPlace, contentHeightPx, fitActions, grownHeight, neededHeightMm, shrunkText } from "../../lib/textFit";
  import { mmToPx } from "../../lib/units";
  import type { Element, RichText, Run, TextProps } from "../../lib/types";

  let {
    element,
    props,
    zoom,
    editing = false,
    alerts = true,
    ondone,
  }: { element: Element; props: TextProps; zoom: number; editing?: boolean; alerts?: boolean; ondone?: (content: RichText) => void } = $props();

  let box: HTMLDivElement | undefined = $state();
  let overflow = $state(false);

  const blank = $derived(props.content.paragraphs.every((p) => p.runs.every((r) => r.text.trim() === "")));

  function runStyle(r: Run): string {
    return [
      r.font ? `font-family: "${r.font}", "Andika"` : "",
      r.size_pt ? `font-size: ${r.size_pt}pt` : "",
      r.color ? `color: ${r.color}` : "",
      r.bold ? "font-weight: 700" : "",
      r.italic ? "font-style: italic" : "",
      r.underline ? "text-decoration: underline" : "",
    ]
      .filter(Boolean)
      .join("; ");
  }

  $effect(() => {
    void props;
    void editing;
    void element.frame;
    if (!box) return;
    const el = box;
    const measure = () => requestAnimationFrame(() => (overflow = el.scrollHeight > el.clientHeight + 1));
    measure();
    return onFontsLoaded(document.fonts, measure);
  });

  // Pendant l'écriture, le texte change sans que `props` change : la zone d'écriture qui grandit ou rapetisse
  // remet le signal à jour.
  $effect(() => {
    const flow = box?.firstElementChild;
    if (!box || !flow || typeof ResizeObserver === "undefined") return;
    const el = box;
    const ro = new ResizeObserver(() => (overflow = el.scrollHeight > el.clientHeight + 1));
    ro.observe(flow);
    return () => ro.disconnect();
  });

  // Corriger d'un clic : seulement sur la page qu'on modifie, pour le bloc choisi seul.
  const fix = $derived(
    alerts && !editor.proposing && editor.selection.length === 1 && editor.selection[0] === element.id && editor.page.elements.includes(element)
      ? fitActions(editor.page, element, editor.inside)
      : { grow: false, shrink: false },
  );
  // Sous le bloc, au-delà des poignées (plus grandes, et écartées d'un petit bloc) ; dans le bloc s'il touche le bas
  // de la page. Les boutons gardent leur taille à tous les zooms.
  const handleHalf = $derived(settings.bigHandles ? 13 : 7);
  const fixGap = $derived(handleHalf + 6 + handlePadding(mmToPx(element.frame.h_mm, zoom), settings.bigHandles));
  const FIX_HEIGHT_PX = 30;
  let fixWidth = $state(270);
  const fixPlace = $derived(actionsPlace(element.frame, editor.page.size, zoom, fixGap + FIX_HEIGHT_PX, fixWidth));

  /** Un seul pas d'annulation, et aucun si rien ne change. */
  function growFrame() {
    if (!box) return;
    const need = Math.max(neededHeightMm(contentHeightPx(box) + 1, textPadding(props.style)), neededHeightMm(box.scrollHeight + 1, 0));
    const g = grownHeight(need, element.frame, editor.page.size.height_mm);
    if (g) editor.updateElement(element.id, (e) => ({ ...e, frame: { ...e.frame, h_mm: g.h_mm } }));
  }

  /** Le texte essayé à d'autres tailles dans une copie cachée du bloc, à côté de lui (mêmes styles, même largeur). */
  function fitsAt(p: TextProps): boolean {
    if (!box) return true;
    const clone = box.cloneNode(true) as HTMLDivElement;
    clone.style.visibility = "hidden";
    clone.style.fontSize = `${p.style.size_pt}pt`;
    clone.setAttribute("aria-hidden", "true");
    const paras = clone.querySelectorAll<HTMLElement>(":scope > .flow > p");
    p.content.paragraphs.forEach((q, i) =>
      q.runs.forEach((r, j) => {
        const span = paras[i]?.children[j] as HTMLElement | undefined;
        if (span && r.size_pt) span.style.fontSize = `${r.size_pt}pt`;
      }),
    );
    box.after(clone);
    const fits = clone.scrollHeight <= clone.clientHeight + 1;
    clone.remove();
    return fits;
  }

  function shrinkText() {
    const next = shrunkText(props, fitsAt);
    if (next) editor.updateElement(element.id, (e) => (e.kind.type === "Text" ? { ...e, kind: { ...e.kind, style: next.style, content: next.content } } : e));
  }

  /** Un appui sur les boutons ne prend ni ne déplace le bloc, et deux clics de suite ne l'ouvrent pas à l'écriture.
   *  Écouteurs posés directement : les poignées (Moveable) écoutent le bloc sans passer par Svelte. */
  function shield(node: HTMLElement) {
    const stop = (ev: Event) => ev.stopPropagation();
    const types = ["pointerdown", "mousedown", "touchstart", "dblclick"];
    for (const t of types) node.addEventListener(t, stop);
    return { destroy: () => types.forEach((t) => node.removeEventListener(t, stop)) };
  }
</script>

{#if props.frame}
  <!-- Cadre derrière le texte, en mm comme lui ; trait centré sur le bord, comme dans LibreOffice. -->
  {@const f = props.frame}
  {@const w = element.frame.w_mm}
  {@const h = element.frame.h_mm}
  {#if f.image}
    <!-- Image de fond : couvre le cadre, recadrée au centre (comme l'export), aux coins du cadre. Un fond CSS
         plutôt qu'un clipPath SVG : l'élément est dessiné plusieurs fois (page, miniatures, impression). -->
    <div
      class="deco"
      style:width="{w}mm"
      style:height="{h}mm"
      style:border-radius="{f.corner_radius_mm}mm"
      style:background-image="url({JSON.stringify(assetUrl(f.image))})"
      style:transform="scale({zoom})"
      aria-hidden="true"
    ></div>
  {/if}
  <svg class="deco" viewBox="0 0 {w} {h}" style:width="{w}mm" style:height="{h}mm" style:transform="scale({zoom})" aria-hidden="true">
    <rect
      width={w}
      height={h}
      rx={f.corner_radius_mm}
      fill={f.image ? "none" : (f.fill ?? "none")}
      stroke={f.stroke?.color ?? "none"}
      stroke-width={f.stroke?.width_mm ?? 0}
      stroke-dasharray={dashArray(f.stroke, f.stroke?.width_mm ?? 0)}
    />
    {#if f.pattern}
      <!-- Lignes d'écriture, entre le fond et le texte, comme à l'export. -->
      {#each frameMarks({ width_mm: w, height_mm: h }, f.pattern) as m, i (i)}
        {#if m.type === "Line"}
          <line x1={m.x1} y1={m.y1} x2={m.x2} y2={m.y2} stroke={m.color} stroke-width={m.width_mm} />
        {:else}
          <circle cx={m.x} cy={m.y} r={m.d_mm / 2} fill={m.color} />
        {/if}
      {/each}
    {/if}
  </svg>
{/if}
<div
  class="text"
  class:overflow
  bind:this={box}
  style:width="{element.frame.w_mm}mm"
  style:height="{element.frame.h_mm}mm"
  style:padding="{textPadding(props.style)}mm"
  style:transform="scale({zoom})"
  style:font-family={`"${props.style.font}", "Andika"`}
  style:font-size="{props.style.size_pt}pt"
  style:color={props.style.color}
  style:text-align={alignToCss(props.style.align)}
  class:outline={props.style.outline}
  class:editing
>
  <!-- Alignement vertical par marges automatiques : un texte trop long part du haut, sans déborder vers le haut. -->
  <div class="flow" class:middle={props.style.valign === "Middle"} class:bottom={props.style.valign === "Bottom"}>
    {#if editing && ondone}
      <TextEditor {props} {ondone} />
    {:else if blank}
      {#if props.prompt}
        <!-- Une case de lettre mobile est petite : seul son numéro s'affiche. -->
        <p class="prompt zone-empty">{props.prompt.match(/^lettre (\d+)$/i)?.[1] ?? props.prompt}</p>
      {/if}
    {:else}
      {#each props.content.paragraphs as p}
        <p style:text-align={p.align ? alignToCss(p.align) : null}>
          {#each p.runs as r}<span style={runStyle(r)}
              >{#each splitFields(r.text) as part}{#if "field" in part}<span class="class-field" title={fieldLabel(part.field)}>{fieldShown(part.field)}</span>{:else}{part.text}{/if}{/each}</span
            >{/each}{#if p.runs.every((r) => r.text === "")}<br />{/if}
        </p>
      {/each}
    {/if}
  </div>
</div>
<!-- Une invite trop longue n'est qu'une indication : pas d'alerte. -->
{#if alerts && overflow && !editing && !blank}
  {#if fix.grow || fix.shrink}
    <div
      class="badge fix"
      class:inside={fixPlace.inside}
      class:left={fixPlace.left}
      style:--gap="{fixGap}px"
      style:--edge="{handleHalf + 4}px"
      role="presentation"
      use:shield
      bind:offsetWidth={fixWidth}
    >
      {#if fix.grow}<button type="button" title={fr.textGrowFrameHint} onclick={(ev) => (ev.currentTarget.blur(), growFrame())}>{fr.textGrowFrame}</button>{/if}
      {#if fix.shrink}<button type="button" title={fr.textShrinkHint} onclick={(ev) => (ev.currentTarget.blur(), shrinkText())}>{fr.textShrink}</button>{/if}
    </div>
  {:else}
    <span class="badge" style:transform="scale({Math.max(zoom, 0.6)})">{fr.textTooLong}</span>
  {/if}
{/if}

<style>
  .text {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
    overflow: hidden;
    line-height: normal;
    white-space: pre-wrap;
    overflow-wrap: break-word;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
  }
  /* En haut, la zone d'écriture occupe tout le cadre (un clic sous le texte y place le curseur). */
  .flow {
    flex: 1 0 auto;
  }
  .flow.middle {
    flex: none;
    margin-block: auto;
  }
  .flow.bottom {
    flex: none;
    margin-top: auto;
  }
  .deco {
    position: absolute;
    left: 0;
    top: 0;
    transform-origin: 0 0;
    overflow: visible;
    pointer-events: none;
    background-size: cover;
    background-position: center;
  }
  .text :global(p) {
    margin: 0;
  }
  /* Lettres creuses : contour dans la couleur du texte (celle de chaque passage), sans remplissage. */
  .text.outline {
    -webkit-text-fill-color: transparent;
    -webkit-text-stroke-width: 0.035em;
    -webkit-text-stroke-color: currentColor;
  }
  .text.overflow {
    outline: 2px solid #e74c3c;
    outline-offset: -2px;
  }
  /* Pendant l'écriture, le surplus reste visible (et le curseur avec lui) : rien ne bouge, il dépasse du cadre. */
  .text.editing {
    overflow: visible;
  }
  .prompt {
    color: #9a9a9a;
    font-style: italic;
  }
  .badge {
    position: absolute;
    right: 0;
    bottom: -22px;
    transform-origin: right top;
    background: #e74c3c;
    color: white;
    font: 600 11px var(--ui-font);
    padding: 2px 6px;
    border-radius: 4px;
    white-space: nowrap;
  }
  /* Le bloc qui montre les boutons passe devant ses voisins : un élément posé dessous ne les couvre pas. */
  :global(.element:has(> .badge.fix)) {
    z-index: 1;
  }
  .badge.fix {
    bottom: auto;
    top: calc(100% + var(--gap));
    display: flex;
    gap: 4px;
    padding: 3px;
    border-radius: 6px;
  }
  .badge.fix.inside {
    top: auto;
    bottom: var(--edge);
    right: var(--edge);
  }
  .badge.fix.left {
    right: auto;
    left: 0;
  }
  .badge.fix.inside.left {
    left: var(--edge);
  }
  .fix button {
    font: 600 13px var(--ui-font);
    padding: 4px 10px;
    border: none;
    border-radius: 4px;
    background: white;
    color: #b03a2e;
    cursor: pointer;
  }
  .fix button:hover {
    text-decoration: underline;
  }
  .fix button:focus-visible {
    outline: 2px solid white;
    outline-offset: 1px;
  }
  /* Un champ de la classe : une pastille qui dit son nom ; à l'impression, blanc à la même place (print.css).
     Aussi dans l'éditeur, d'où le sélecteur global. */
  /* En ligne, comme le texte : un nom long (« Anniversaires de septembre ») passe à la ligne, chaque morceau
     gardant sa pastille. */
  :global(.class-field) {
    padding: 0 0.3em;
    border: 1px solid #2b6cb0;
    border-radius: 0.6em;
    background: #ebf4ff;
    color: #1a4a80;
    -webkit-box-decoration-break: clone;
    box-decoration-break: clone;
  }
  :global(:root[data-contrast] .class-field) {
    border-width: 2px;
    border-color: #000;
    background: #fff;
    color: #000;
  }
</style>

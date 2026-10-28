<script lang="ts">
  // Le texte est mis en page à sa taille réelle (mm et pt), puis mis à l'échelle du zoom :
  // les retours à la ligne ne dépendent pas du zoom et correspondent à ceux de l'export.
  import TextEditor from "../TextEditor.svelte";
  import { assetUrl } from "../../lib/api";
  import { alignToCss, textPadding } from "../../lib/richtext";
  import { dashArray } from "../../lib/strokes";
  import type { Element, RichText, Run, TextProps } from "../../lib/types";

  let {
    element,
    props,
    zoom,
    editing = false,
    ondone,
  }: { element: Element; props: TextProps; zoom: number; editing?: boolean; ondone?: (content: RichText) => void } = $props();

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
    requestAnimationFrame(() => (overflow = el.scrollHeight > el.clientHeight + 1));
  });
</script>

{#if props.frame}
  <!-- Cadre derrière le texte, en mm comme lui ; trait centré sur le bord, comme dans LibreOffice. -->
  {@const f = props.frame}
  {@const w = element.frame.w_mm}
  {@const h = element.frame.h_mm}
  <svg class="deco" viewBox="0 0 {w} {h}" style:width="{w}mm" style:height="{h}mm" style:transform="scale({zoom})" aria-hidden="true">
    {#if f.image}
      <!-- Image de fond : couvre le cadre, recadrée au centre (comme l'export), aux coins du cadre. -->
      <clipPath id="frame-clip-{element.id}"><rect width={w} height={h} rx={f.corner_radius_mm} /></clipPath>
      <image href={assetUrl(f.image)} width={w} height={h} preserveAspectRatio="xMidYMid slice" clip-path="url(#frame-clip-{element.id})" />
    {/if}
    <rect
      width={w}
      height={h}
      rx={f.corner_radius_mm}
      fill={f.image ? "none" : (f.fill ?? "none")}
      stroke={f.stroke?.color ?? "none"}
      stroke-width={f.stroke?.width_mm ?? 0}
      stroke-dasharray={dashArray(f.stroke, f.stroke?.width_mm ?? 0)}
    />
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
>
  <!-- Alignement vertical par marges automatiques : un texte trop long part du haut, sans déborder vers le haut. -->
  <div class="flow" class:middle={props.style.valign === "Middle"} class:bottom={props.style.valign === "Bottom"}>
    {#if editing && ondone}
      <TextEditor {props} {ondone} />
    {:else if blank}
      {#if props.prompt}
        <p class="prompt zone-empty">{props.prompt}</p>
      {/if}
    {:else}
      {#each props.content.paragraphs as p}
        <p style:text-align={p.align ? alignToCss(p.align) : null}>
          {#each p.runs as r}<span style={runStyle(r)}>{r.text}</span>{/each}{#if p.runs.every((r) => r.text === "")}<br />{/if}
        </p>
      {/each}
    {/if}
  </div>
</div>
{#if overflow && !editing}
  <span class="badge" style:transform="scale({Math.max(zoom, 0.6)})">Texte trop long</span>
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
  }
  .text :global(p) {
    margin: 0;
  }
  .text.overflow {
    outline: 2px solid #e74c3c;
    outline-offset: -2px;
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
</style>

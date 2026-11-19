<script lang="ts">
  // Rectangle et ellipse en CSS ; les formes dessinées, les tracés libres et les traits en SVG, avec le tracé de
  // l'export.
  import { SHAPE_VIEWBOX } from "../../lib/shapePaths";
  import { shapePath, tailBox } from "../../lib/tail";
  import { endsOf, headWidthMm } from "../../lib/lineEnds";
  import { lineSvg, pathSvg } from "../../lib/lineSvg";
  import { hitMm, isPath, pathPointsMm } from "../../lib/linePath";
  import { dashArray, isSolid } from "../../lib/strokes";
  import { mmToPx } from "../../lib/units";
  import type { Frame, ShapeProps } from "../../lib/types";

  // `solid` : l'intérieur d'une forme vide se clique comme celui d'une forme pleine (sinon, le contour seul).
  let { props, frame, zoom, solid = false }: { props: ShapeProps; frame: Frame; zoom: number; solid?: boolean } = $props();

  // Tracé libre (graphisme, chemin) : le sien ; une bulle à pointe : calculé pour son cadre ; sinon celui de la sorte
  // de forme.
  const path = $derived(props.shape === "Path" ? (props.path ?? undefined) : shapePath(frame, props));
  // Une bulle à pointe : le dessin s'étend jusqu'à la pointe (sinon le clic sur la pointe, hors du cadre, se perd).
  const box = $derived(tailBox(props));
  const strokePx = $derived(props.stroke ? mmToPx(props.stroke.width_mm, zoom) : 0);
  const w = $derived(mmToPx(frame.w_mm, zoom));
  const h = $derived(mmToPx(frame.h_mm, zoom));
  // Pointe de flèche : même proportion que le marqueur de l'export (base 20, longueur 30).
  const headW = $derived(mmToPx(headWidthMm(props), zoom));
  const dash = $derived(dashArray(props.stroke, strokePx));
  /** Épaisseur cliquable autour du contour d'une forme vide. */
  const HIT_PX = 8;
</script>

{#if (props.shape === "Line" || props.shape === "Arrow") && isPath(props)}
  <!-- Chemin à coudes ou courbe : dessiné en mm de son cadre (rien ne s'étire), cliquable sur son tracé seul. -->
  <svg class="shape hollow" width={w} height={h} viewBox="0 0 {frame.w_mm} {frame.h_mm}" aria-hidden="true">
    {#if props.stroke}
      {@const p = pathSvg(pathPointsMm(frame, props), !!props.curve, endsOf(props), headWidthMm(props), !!props.closed)}
      {@const c = props.stroke.color}
      <!-- Fermé : tout l'intérieur se clique, comme une forme vide ; ouvert, le tracé seul. -->
      {@const filled = !!props.closed && !!props.fill}
      <path class="hit" class:solid={!!props.closed && (filled || solid)} d={p.d} stroke-width={hitMm(props)} />
      <path d={p.d} fill={filled ? props.fill : "none"} stroke={c} stroke-width={props.stroke.width_mm} stroke-dasharray={dashArray(props.stroke, props.stroke.width_mm)} stroke-linejoin="round" />
      {#each p.parts as part, i (i)}
        {#if part.kind === "polygon"}<polygon class="hit-all" {...part.attrs} fill={c} />
        {:else if part.kind === "polyline"}<polyline class="hit-all" {...part.attrs} fill="none" stroke={c} stroke-linejoin="round" stroke-linecap="round" />
        {:else if part.kind === "circle"}<circle class="hit-all" {...part.attrs} fill={c} />
        {:else}<rect class="hit-all" {...part.attrs} fill={c} />{/if}
      {/each}
    {/if}
  </svg>
{:else if props.shape === "Line" || props.shape === "Arrow"}
  <svg class="shape" width={w} height={h} aria-hidden="true">
    {#if props.stroke}
      {@const line = lineSvg(w, h, endsOf(props), headW)}
      {@const c = props.stroke.color}
      <line x1={line.x1} y1={h / 2} x2={line.x2} y2={h / 2} stroke={c} stroke-width={strokePx} stroke-dasharray={dash} />
      <!-- Terminaisons pleines de la couleur du trait, jamais en tirets ; le chevron, au trait. -->
      {#each line.parts as part, i (i)}
        {#if part.kind === "polygon"}<polygon {...part.attrs} fill={c} />
        {:else if part.kind === "polyline"}<polyline {...part.attrs} fill="none" stroke={c} stroke-linejoin="round" stroke-linecap="round" />
        {:else if part.kind === "circle"}<circle {...part.attrs} fill={c} />
        {:else}<rect {...part.attrs} fill={c} />{/if}
      {/each}
    {/if}
  </svg>
{:else if path}
  <svg
    class="shape"
    class:hollow={!props.fill}
    class:tailed={box.x0 < 0 || box.y0 < 0 || box.x1 > 1 || box.y1 > 1}
    viewBox="{box.x0 * SHAPE_VIEWBOX} {box.y0 * SHAPE_VIEWBOX} {(box.x1 - box.x0) * SHAPE_VIEWBOX} {(box.y1 - box.y0) * SHAPE_VIEWBOX}"
    preserveAspectRatio="none"
    style:left="{box.x0 * 100}%"
    style:top="{box.y0 * 100}%"
    style:width="{(box.x1 - box.x0) * 100}%"
    style:height="{(box.y1 - box.y0) * 100}%"
    aria-hidden="true"
  >
    {#if !props.fill}<path class="hit" class:solid d={path} stroke-width={Math.max(strokePx, HIT_PX)} vector-effect="non-scaling-stroke" />{/if}
    <path
      d={path}
      fill={props.fill ?? "none"}
      stroke={props.stroke?.color ?? "none"}
      stroke-width={strokePx}
      stroke-dasharray={dash}
      stroke-linejoin="round"
      vector-effect="non-scaling-stroke"
    />
  </svg>
{:else if !props.fill || !isSolid(props.stroke)}
  <!-- Forme vide (contour cliquable, intérieur aussi si `solid`) ou bordure en tirets
       (dessinée en SVG : les tirets CSS dépendent du moteur du navigateur). -->
  {@const sw = strokePx}
  {@const r = props.shape === "Ellipse" ? null : Math.max(0, mmToPx(props.corner_radius_mm, zoom) - sw / 2)}
  {@const hollow = !props.fill}
  <svg class="shape" class:hollow width={w} height={h} aria-hidden="true">
    {#if r === null}
      {#if hollow}<ellipse class="hit" class:solid cx={w / 2} cy={h / 2} rx={Math.max(0, w / 2 - sw / 2)} ry={Math.max(0, h / 2 - sw / 2)} stroke-width={Math.max(sw, HIT_PX)} />{/if}
      <ellipse cx={w / 2} cy={h / 2} rx={Math.max(0, w / 2 - sw / 2)} ry={Math.max(0, h / 2 - sw / 2)} fill={props.fill ?? "none"} stroke={props.stroke?.color ?? "none"} stroke-width={sw} stroke-dasharray={dash} />
    {:else}
      {#if hollow}<rect class="hit" class:solid x={sw / 2} y={sw / 2} width={Math.max(0, w - sw)} height={Math.max(0, h - sw)} rx={r} stroke-width={Math.max(sw, HIT_PX)} />{/if}
      <rect x={sw / 2} y={sw / 2} width={Math.max(0, w - sw)} height={Math.max(0, h - sw)} rx={r} fill={props.fill ?? "none"} stroke={props.stroke?.color ?? "none"} stroke-width={sw} stroke-dasharray={dash} />
    {/if}
  </svg>
{:else}
  <div
    class="shape"
    style:background={props.fill ?? "transparent"}
    style:border={props.stroke ? `${strokePx}px solid ${props.stroke.color}` : "none"}
    style:border-radius={props.shape === "Ellipse" ? "50%" : `${mmToPx(props.corner_radius_mm, zoom)}px`}
  ></div>
{/if}

<style>
  .shape {
    position: absolute;
    inset: 0;
    box-sizing: border-box;
    overflow: visible;
  }
  svg.shape {
    width: 100%;
    height: 100%;
  }
  /* Forme vide : ni le dessin ni sa boîte ne captent les clics, seul le tracé .hit les reçoit : son contour, ou
     toute sa surface quand l'absence de fond n'est qu'un choix d'apparence (.solid). */
  svg.hollow {
    pointer-events: none;
  }
  .hit {
    fill: none;
    stroke: transparent;
    pointer-events: stroke;
  }
  /* Bulle à pointe : son dessin déborde du cadre jusqu'à la pointe ; seul ce qui est peint reçoit les clics (le vide
     autour de la pointe laisse cliquer ce qui est dessous : le personnage qui parle). */
  svg.tailed {
    pointer-events: none;
  }
  svg.tailed path:not(.hit) {
    pointer-events: visiblePainted;
  }
  .hit.solid {
    pointer-events: all;
  }
  /* Terminaisons d'un chemin : cliquables comme son tracé. */
  .hit-all {
    pointer-events: all;
  }
</style>

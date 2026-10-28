<script lang="ts">
  // Rectangle et ellipse en CSS ; les formes dessinées et les traits en SVG, avec le tracé de l'export.
  import { SHAPE_PATHS, SHAPE_VIEWBOX, arrowHeadMm } from "../../lib/shapePaths";
  import { dashArray, isSolid } from "../../lib/strokes";
  import { mmToPx } from "../../lib/units";
  import type { Frame, ShapeProps } from "../../lib/types";

  let { props, frame, zoom }: { props: ShapeProps; frame: Frame; zoom: number } = $props();

  const path = $derived(SHAPE_PATHS[props.shape]);
  const strokePx = $derived(props.stroke ? mmToPx(props.stroke.width_mm, zoom) : 0);
  const w = $derived(mmToPx(frame.w_mm, zoom));
  const h = $derived(mmToPx(frame.h_mm, zoom));
  // Pointe de flèche : même proportion que celle de LibreOffice (base 20, longueur 30).
  const headW = $derived(mmToPx(arrowHeadMm(props.stroke?.width_mm ?? 1), zoom));
  const headL = $derived(headW * 1.5);
  const dash = $derived(dashArray(props.stroke, strokePx));
  /** Épaisseur cliquable autour du contour d'une forme vide. */
  const HIT_PX = 8;
</script>

{#if props.shape === "Line" || props.shape === "Arrow"}
  <svg class="shape" width={w} height={h} aria-hidden="true">
    {#if props.stroke}
      <line x1="0" y1={h / 2} x2={props.shape === "Arrow" ? w - headL / 2 : w} y2={h / 2} stroke={props.stroke.color} stroke-width={strokePx} stroke-dasharray={dash} />
      {#if props.shape === "Arrow"}
        <polygon points="{w},{h / 2} {w - headL},{h / 2 - headW / 2} {w - headL},{h / 2 + headW / 2}" fill={props.stroke.color} />
      {/if}
    {/if}
  </svg>
{:else if path}
  <svg class="shape" class:hollow={!props.fill} viewBox="0 0 {SHAPE_VIEWBOX} {SHAPE_VIEWBOX}" preserveAspectRatio="none" aria-hidden="true">
    {#if !props.fill}<path class="hit" d={path} stroke-width={Math.max(strokePx, HIT_PX)} vector-effect="non-scaling-stroke" />{/if}
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
  <!-- Forme vide (le contour seul se clique, intérieur transparent aux clics) ou bordure en tirets
       (dessinée en SVG : les tirets CSS dépendent du moteur du navigateur). -->
  {@const sw = strokePx}
  {@const r = props.shape === "Ellipse" ? null : Math.max(0, mmToPx(props.corner_radius_mm, zoom) - sw / 2)}
  {@const hollow = !props.fill}
  <svg class="shape" class:hollow width={w} height={h} aria-hidden="true">
    {#if r === null}
      {#if hollow}<ellipse class="hit" cx={w / 2} cy={h / 2} rx={Math.max(0, w / 2 - sw / 2)} ry={Math.max(0, h / 2 - sw / 2)} stroke-width={Math.max(sw, HIT_PX)} />{/if}
      <ellipse cx={w / 2} cy={h / 2} rx={Math.max(0, w / 2 - sw / 2)} ry={Math.max(0, h / 2 - sw / 2)} fill={props.fill ?? "none"} stroke={props.stroke?.color ?? "none"} stroke-width={sw} stroke-dasharray={dash} />
    {:else}
      {#if hollow}<rect class="hit" x={sw / 2} y={sw / 2} width={Math.max(0, w - sw)} height={Math.max(0, h - sw)} rx={r} stroke-width={Math.max(sw, HIT_PX)} />{/if}
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
  /* Forme vide : ni le dessin ni sa boîte ne captent les clics, seul le contour (.hit) les reçoit. */
  svg.hollow {
    pointer-events: none;
  }
  .hit {
    fill: none;
    stroke: transparent;
    pointer-events: stroke;
  }
</style>

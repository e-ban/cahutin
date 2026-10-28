<script lang="ts">
  // Rectangle et ellipse en CSS ; les formes dessinées et les traits en SVG, avec le tracé de l'export.
  import { SHAPE_PATHS, SHAPE_VIEWBOX, arrowHeadMm } from "../../lib/shapePaths";
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
</script>

{#if props.shape === "Line" || props.shape === "Arrow"}
  <svg class="shape" width={w} height={h} aria-hidden="true">
    {#if props.stroke}
      <line x1="0" y1={h / 2} x2={props.shape === "Arrow" ? w - headL / 2 : w} y2={h / 2} stroke={props.stroke.color} stroke-width={strokePx} />
      {#if props.shape === "Arrow"}
        <polygon points="{w},{h / 2} {w - headL},{h / 2 - headW / 2} {w - headL},{h / 2 + headW / 2}" fill={props.stroke.color} />
      {/if}
    {/if}
  </svg>
{:else if path}
  <svg class="shape" viewBox="0 0 {SHAPE_VIEWBOX} {SHAPE_VIEWBOX}" preserveAspectRatio="none" aria-hidden="true">
    <path
      d={path}
      fill={props.fill ?? "none"}
      stroke={props.stroke?.color ?? "none"}
      stroke-width={strokePx}
      stroke-linejoin="round"
      vector-effect="non-scaling-stroke"
    />
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
</style>

<script lang="ts">
  import { mmToPx } from "../../lib/units";
  import type { ShapeProps } from "../../lib/types";

  let { props, zoom }: { props: ShapeProps; zoom: number } = $props();
</script>

<div
  class="shape"
  style:background={props.fill ?? "transparent"}
  style:border={props.stroke ? `${mmToPx(props.stroke.width_mm, zoom)}px solid ${props.stroke.color}` : "none"}
  style:border-radius={props.shape === "Ellipse" ? "50%" : `${mmToPx(props.corner_radius_mm, zoom)}px`}
></div>

<style>
  .shape {
    position: absolute;
    inset: 0;
    box-sizing: border-box;
  }
</style>

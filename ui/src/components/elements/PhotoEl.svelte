<script lang="ts">
  import { assetUrl } from "../../lib/api";
  import { cropToCss } from "../../lib/crop";
  import { fr } from "../../lib/i18n/fr";
  import { mmToPx } from "../../lib/units";
  import type { AssetMeta, Element, PhotoProps } from "../../lib/types";

  let { element, props, meta, zoom }: { element: Element; props: PhotoProps; meta: AssetMeta | undefined; zoom: number } = $props();

  const css = $derived(meta ? cropToCss(meta, element.frame, props.crop) : null);
</script>

{#if props.asset && meta && css}
  <div
    class="photo"
    style:border-radius="{mmToPx(props.corner_radius_mm, zoom)}px"
    style:outline={props.border ? `${mmToPx(props.border.width_mm, zoom)}px solid ${props.border.color}` : "none"}
    style:outline-offset={props.border ? `-${mmToPx(props.border.width_mm, zoom) / 2}px` : "0"}
    style:box-shadow={props.shadow ? `${mmToPx(1, zoom)}px ${mmToPx(1, zoom)}px 0 rgba(128,128,128,0.5)` : "none"}
  >
    <img
      src={assetUrl(props.asset)}
      alt=""
      draggable="false"
      style:width="{mmToPx(css.widthMm, zoom)}px"
      style:height="{mmToPx(css.heightMm, zoom)}px"
      style:left="{mmToPx(css.leftMm, zoom)}px"
      style:top="{mmToPx(css.topMm, zoom)}px"
    />
  </div>
{:else}
  <div class="zone-empty photo-zone" class:zone={element.zone}>
    <span>{element.zone ? fr.dropPhotoHere : fr.photo}</span>
  </div>
{/if}

<style>
  .photo {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }
  img {
    position: absolute;
    max-width: none;
    user-select: none;
  }
  .zone-empty {
    position: absolute;
    inset: 0;
    border: 2px dashed var(--zone-photo);
    background: color-mix(in srgb, var(--zone-photo) 8%, transparent);
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--zone-photo);
    font: 600 13px var(--ui-font);
    text-align: center;
    padding: 4px;
  }
</style>

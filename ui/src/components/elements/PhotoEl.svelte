<script lang="ts">
  import { assetUrl } from "../../lib/api";
  import { cropToCss } from "../../lib/crop";
  import { fr } from "../../lib/i18n/fr";
  import { dashArray, isSolid } from "../../lib/strokes";
  import { mmToPx } from "../../lib/units";
  import type { AssetMeta, Element, PhotoProps } from "../../lib/types";

  let { element, props, meta, zoom }: { element: Element; props: PhotoProps; meta: AssetMeta | undefined; zoom: number } = $props();

  const css = $derived(meta ? cropToCss(meta, element.frame, props.crop) : null);
  const borderPx = $derived(props.border ? mmToPx(props.border.width_mm, zoom) : 0);
  // Bordure pleine en contour CSS ; en tirets, en SVG avec les longueurs de l'export.
  const solidBorder = $derived(!!props.border && isSolid(props.border));
</script>

{#if props.asset && meta && css}
  <div
    class="photo"
    style:border-radius="{mmToPx(props.corner_radius_mm, zoom)}px"
    style:outline={solidBorder ? `${borderPx}px solid ${props.border!.color}` : "none"}
    style:outline-offset={solidBorder ? `-${borderPx / 2}px` : "0"}
    style:box-shadow={props.shadow ? `${mmToPx(1, zoom)}px ${mmToPx(1, zoom)}px 0 rgba(128,128,128,0.5)` : "none"}
  >
    <!-- Miroir : la partie visible, retournée d'un bloc (comme l'export). -->
    <div class="flip" class:mirrored={props.mirrored}>
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
  </div>
  {#if props.border && !solidBorder}
    <!-- Hors du cadre rogné : le trait est centré sur le bord, comme le contour plein et dans LibreOffice. -->
    {@const w = mmToPx(element.frame.w_mm, zoom)}
    {@const h = mmToPx(element.frame.h_mm, zoom)}
    <svg class="border" width={w} height={h} aria-hidden="true">
      <rect width={w} height={h} rx={mmToPx(props.corner_radius_mm, zoom)} fill="none" stroke={props.border.color} stroke-width={borderPx} stroke-dasharray={dashArray(props.border, borderPx)} />
    </svg>
  {/if}
{:else}
  <div class="zone-empty photo-zone" class:zone={element.zone} style:border-width="{zoom < 0.5 ? 1 : 2}px">
    <!-- Dans les miniatures, l'invite serait illisible : seule la zone en pointillés reste. -->
    {#if zoom >= 0.5}<span>{element.zone ? fr.dropPhotoHere : fr.photo}</span>{/if}
  </div>
{/if}

<style>
  .flip {
    position: absolute;
    inset: 0;
  }
  .flip.mirrored {
    transform: scaleX(-1);
  }
  .photo {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }
  .border {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
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

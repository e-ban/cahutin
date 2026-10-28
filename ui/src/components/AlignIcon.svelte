<script lang="ts">
  // Icône d'alignement du texte, à la manière des traitements de texte : quatre lignes alignées.
  import type { Align } from "../lib/types";

  let { align }: { align: Align } = $props();

  // Longueurs des lignes (sur 14) ; la dernière est plus courte, sauf en justifié.
  const lengths = [14, 9, 12, 7];
  const x = (len: number) => (align === "Center" ? (14 - len) / 2 : align === "Right" ? 14 - len : 0);
</script>

<svg class="icon" width="14" height="12" viewBox="0 0 14 12" aria-hidden="true">
  {#each lengths as len, i (i)}
    {@const l = align === "Justify" && i < 3 ? 14 : len}
    <rect x={x(l)} y={i * 3 + 0.5} width={l} height="1.6" rx="0.5" fill="currentColor" />
  {/each}
</svg>

<style>
  .icon {
    display: block;
  }
</style>

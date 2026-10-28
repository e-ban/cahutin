<script lang="ts">
  // Icône d'alignement vertical : deux lignes de texte en haut, au milieu ou en bas d'un cadre.
  import type { VAlign } from "../lib/types";

  let { valign }: { valign: VAlign } = $props();

  const y = $derived(valign === "Top" ? 3 : valign === "Middle" ? 5.5 : 8);
</script>

<svg class="icon" width="14" height="14" viewBox="0 0 14 14" aria-hidden="true">
  <rect x="0.5" y="0.5" width="13" height="13" rx="1.5" fill="none" stroke="currentColor" stroke-width="1" opacity="0.6" />
  <rect x="3" y={y} width="8" height="1.4" rx="0.5" fill="currentColor" />
  <rect x="3" y={y + 2.4} width="6" height="1.4" rx="0.5" fill="currentColor" />
</svg>

<style>
  .icon {
    display: block;
  }
</style>

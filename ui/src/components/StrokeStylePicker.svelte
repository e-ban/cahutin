<script lang="ts">
  // Style d'un trait ou d'une bordure : plein, tirets, pointillés (aperçus dessinés comme sur la page).
  import { fr } from "../lib/i18n/fr";
  import { dashArray } from "../lib/strokes";
  import type { StrokeStyle } from "../lib/types";

  let { value, onpick }: { value: StrokeStyle | null | undefined; onpick: (s: StrokeStyle) => void } = $props();

  const STYLES: StrokeStyle[] = ["Solid", "Dashed", "Dotted"];
</script>

<div class="styles" role="group" aria-label={fr.strokeStyle}>
  {#each STYLES as s (s)}
    <button class:on={(value ?? "Solid") === s} title={fr.strokeStyles[s]} aria-label={fr.strokeStyles[s]} onclick={() => onpick(s)}>
      <svg width="28" height="8" aria-hidden="true">
        <line x1="1" y1="4" x2="27" y2="4" stroke="currentColor" stroke-width="2" stroke-dasharray={dashArray({ width_mm: 0, color: "", style: s }, 2)} />
      </svg>
    </button>
  {/each}
</div>

<style>
  .styles {
    display: flex;
    gap: 2px;
  }
  button {
    padding: 4px 6px;
    display: flex;
  }
  .on {
    background: #e8f1fb;
    border-color: var(--accent);
    color: var(--accent);
  }
</style>

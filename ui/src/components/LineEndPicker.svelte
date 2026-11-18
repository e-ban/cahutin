<script lang="ts">
  // Terminaison d'un bout de trait (cinq dessins, nommés au survol) et taille des terminaisons du trait.
  import { lineSvg } from "../lib/lineSvg";
  import { fr } from "../lib/i18n/fr";
  import type { HeadSize, LineEnd } from "../lib/types";

  let { which, value, size, onpick, onsize }: { which: "start" | "end"; value: LineEnd; size: HeadSize; onpick: (end: LineEnd) => void; onsize: (size: HeadSize) => void } = $props();

  const ENDS: LineEnd[] = ["None", "Arrow", "OpenArrow", "Circle", "Square"];
  const SIZES: HeadSize[] = ["Small", "Medium", "Large"];
  const W = 36;
  const H = 18;
  // L'aperçu montre le bout réglé : à gauche pour le début, à droite pour la fin.
  const preview = (end: LineEnd) => lineSvg(W, H, which === "start" ? { start: end, end: "None", size: "Medium" } : { start: "None", end, size: "Medium" }, 9);
</script>

<div class="ends" role="radiogroup" aria-label={which === "start" ? fr.lineStart : fr.lineEnd}>
  {#each ENDS as end (end)}
    {@const p = preview(end)}
    <button role="radio" aria-checked={value === end} class:on={value === end} title={fr.lineEnds[end]} aria-label={fr.lineEnds[end]} onclick={() => onpick(end)}>
      <svg width={W} height={H} aria-hidden="true">
        <line x1={p.x1 + 3} y1={H / 2} x2={p.x2 - 3} y2={H / 2} stroke="currentColor" stroke-width="2" />
        <g transform="translate({which === 'start' ? 3 : -3} 0)">
          {#each p.parts as part, i (i)}
            {#if part.kind === "polygon"}<polygon {...part.attrs} fill="currentColor" />
            {:else if part.kind === "polyline"}<polyline {...part.attrs} fill="none" stroke="currentColor" stroke-linejoin="round" stroke-linecap="round" />
            {:else if part.kind === "circle"}<circle {...part.attrs} fill="currentColor" />
            {:else}<rect {...part.attrs} fill="currentColor" />{/if}
          {/each}
        </g>
      </svg>
    </button>
  {/each}
</div>
<div class="sizes" role="radiogroup" aria-label={fr.headSizeLabel}>
  <span>{fr.headSizeLabel}</span>
  {#each SIZES as s (s)}
    <button role="radio" aria-checked={size === s} class:on={size === s} onclick={() => onsize(s)}>{fr.headSizes[s]}</button>
  {/each}
</div>

<style>
  .ends,
  .sizes {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  /* Cinq cases égales qui tiennent dans la largeur du panneau Réglages comme dans le menu de la barre. */
  .ends {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
  }
  .ends button {
    min-width: 0;
    padding-left: 2px;
    padding-right: 2px;
  }

  .sizes {
    margin-top: 6px;
  }
  button.on {
    outline: 2px solid var(--accent, #2b7de9);
  }
</style>

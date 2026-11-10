<script lang="ts">
  // Pastilles de couleur et sélecteur libre. Préférences : couleurs pour élèves daltoniens, nom de chaque couleur
  // au survol et nom de la couleur choisie écrit à côté.
  import Icon from "./Icon.svelte";
  import { fr } from "../lib/i18n/fr";
  import { CVD_PALETTE, PALETTE, colorName } from "../lib/palette";
  import { settings } from "../lib/settings.svelte";

  let { value, onchange, allowNone = false }: { value: string | null; onchange: (c: string | null) => void; allowNone?: boolean } = $props();
</script>

<div class="colors">
  {#if allowNone}
    <button class="swatch none" class:active={value === null} title="Aucune" aria-label="Aucune" onclick={() => onchange(null)}><Icon name="ban" size={14} /></button>
  {/if}
  {#each settings.cvdPalette ? CVD_PALETTE : PALETTE as s (s.color)}
    {@const label = settings.colorNames ? s.name : s.color}
    <button class="swatch" class:active={value === s.color} style:background={s.color} title={label} aria-label={s.name} onclick={() => onchange(s.color)}></button>
  {/each}
  <input type="color" title={fr.otherColor} aria-label={fr.otherColor} value={value ?? "#ffffff"} onchange={(e) => onchange(e.currentTarget.value)} />
  {#if settings.colorNames && value}<span class="name">{colorName(value) ?? fr.otherColor}</span>{/if}
</div>

<style>
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    align-items: center;
  }
  .swatch {
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: 50%;
  }
  .swatch.active {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .none {
    display: grid;
    place-items: center;
  }
  .name {
    font-size: 12px;
    color: var(--muted);
  }
  input[type="color"] {
    width: 28px;
    height: 24px;
    padding: 0;
  }
</style>

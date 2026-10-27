<script lang="ts">
  import { PALETTE } from "../lib/palette";

  let { value, onchange, allowNone = false }: { value: string | null; onchange: (c: string | null) => void; allowNone?: boolean } = $props();
</script>

<div class="colors">
  {#if allowNone}
    <button class="swatch none" class:active={value === null} title="Aucune" onclick={() => onchange(null)}>∅</button>
  {/if}
  {#each PALETTE as c}
    <button class="swatch" class:active={value === c} style:background={c} title={c} aria-label={c} onclick={() => onchange(c)}></button>
  {/each}
  <input type="color" value={value ?? "#ffffff"} onchange={(e) => onchange(e.currentTarget.value)} />
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
    font-size: 12px;
    line-height: 1;
  }
  input[type="color"] {
    width: 28px;
    height: 24px;
    padding: 0;
  }
</style>

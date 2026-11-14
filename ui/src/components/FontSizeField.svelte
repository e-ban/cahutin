<script lang="ts">
  // Taille du texte, la même dans la barre et dans le panneau : tapée (« 13,5 ») ou choisie dans la liste.
  // Une saisie qui n'est pas une taille est oubliée : le champ reprend la taille actuelle.
  import Icon from "./Icon.svelte";
  import { FONT_SIZES, formatFontSize, MAX_FONT_PT, MIN_FONT_PT, parseFontSize } from "../lib/fontSizes";
  import { fr } from "../lib/i18n/fr";
  import { fitInWindow } from "../lib/popover";

  let { value, onpick, unit = false }: { value: number; onpick: (pt: number) => void; unit?: boolean } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();

  function commit(input: HTMLInputElement) {
    const pt = parseFontSize(input.value);
    input.value = formatFontSize(pt ?? value);
    if (pt !== null && pt !== value) onpick(pt);
  }

  function pick(pt: number) {
    open = false;
    onpick(pt);
  }

  function onKey(e: KeyboardEvent & { currentTarget: HTMLInputElement }) {
    if (e.key === "Escape") {
      e.stopPropagation();
      e.currentTarget.value = formatFontSize(value);
      open = false;
    } else if (e.key === "Enter") {
      e.preventDefault();
      commit(e.currentTarget);
    }
  }
</script>

<svelte:window
  onkeydowncapture={(e) => {
    // Échap, liste ouverte : seule la liste se ferme (l'écriture en cours continue, le bloc reste choisi).
    if (!open || e.key !== "Escape") return;
    e.preventDefault();
    e.stopPropagation();
    open = false;
  }}
  onpointerdown={(e) => {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }}
/>

<div class="size" bind:this={root}>
  <input
    value={formatFontSize(value)}
    inputmode="decimal"
    title={fr.sizeHint(MIN_FONT_PT, MAX_FONT_PT)}
    aria-label={fr.size}
    onchange={(e) => commit(e.currentTarget)}
    onkeydown={onKey}
  />
  {#if unit}<span class="unit">pt</span>{/if}
  <button class="more" title={fr.sizeList} aria-label={fr.sizeList} aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}><Icon name="chevronDown" size={14} /></button>
  {#if open}
    <div use:fitInWindow class="pop" role="listbox" aria-label={fr.size}>
      {#each FONT_SIZES as s (s)}
        <button role="option" aria-selected={s === value} class:on={s === value} onclick={() => pick(s)}>{s}</button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .size {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  input {
    width: 4em;
    text-align: right;
  }
  .unit {
    color: var(--muted);
  }
  .more {
    padding: 4px;
  }
  .pop {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 72px;
    max-height: 320px;
    overflow-y: auto;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px var(--shadow);
    padding: 4px;
    z-index: 4000;
    display: flex;
    flex-direction: column;
  }
  .pop button {
    border: 0;
    background: transparent;
    text-align: right;
    padding: 4px 10px;
  }
  .pop button:hover,
  .pop button.on {
    background: var(--accent-soft);
  }
</style>

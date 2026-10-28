<script lang="ts">
  // Choix d'une police avec recherche : fournies, ajoutées par vous, puis celles de l'ordinateur.
  import { tick } from "svelte";
  import { fonts } from "../lib/fonts.svelte";
  import { groupFamilies, searchFamilies } from "../lib/fontSearch";
  import { fr } from "../lib/i18n/fr";
  import type { FontFamilyInfo } from "../lib/types";

  let { value, onpick, compact = false }: { value: string; onpick: (name: string) => void; compact?: boolean } = $props();

  let open = $state(false);
  let query = $state("");
  let root = $state<HTMLElement>();
  let input = $state<HTMLInputElement>();

  const found = $derived(searchFamilies(fonts.families, query));
  const groups = $derived(groupFamilies(found));
  const sections = $derived(
    [
      { title: fr.builtinFonts, items: groups.builtin },
      { title: fr.userFonts, items: groups.user },
      { title: fr.systemFonts, items: groups.system },
    ].filter((s) => s.items.length > 0),
  );

  async function toggle() {
    open = !open;
    query = "";
    if (open) {
      await tick();
      input?.focus();
    }
  }

  function pick(f: FontFamilyInfo) {
    open = false;
    onpick(f.name);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      open = false;
    } else if (e.key === "Enter" && found.length > 0) {
      e.preventDefault();
      pick(found[0]);
    }
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }}
/>

<div class="picker" bind:this={root}>
  <button class="trigger" class:compact title={fr.font} aria-haspopup="listbox" aria-expanded={open} onclick={toggle}>
    <span style:font-family={`"${value}", "Andika"`}>{value}</span> ▾
  </button>
  {#if open}
    <div class="pop">
      <input bind:this={input} bind:value={query} placeholder={fr.searchFont} aria-label={fr.searchFont} onkeydown={onKey} />
      <div class="list" role="listbox" aria-label={fr.fonts}>
        {#each sections as s (s.title)}
          <div class="group">{s.title}</div>
          {#each s.items as f (f.name)}
            <button role="option" aria-selected={f.name === value} class:on={f.name === value} onclick={() => pick(f)}>
              <span class="name" style:font-family={`"${f.name}", "Andika"`}>{f.name}</span>
              {#if !f.embeddable}<small class="warn">{fr.exportWarning}</small>{/if}
            </button>
          {/each}
        {:else}
          <p class="none">{fr.noFontFound}</p>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    text-align: left;
    padding: 4px 8px;
  }
  .trigger span {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trigger.compact {
    max-width: 160px;
  }
  .pop {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    width: 280px;
    background: white;
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
    padding: 6px;
    z-index: 4000;
    display: flex;
    flex-direction: column;
    gap: 6px;
    white-space: normal;
  }
  .list {
    max-height: 320px;
    overflow-y: auto;
  }
  .group {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    margin: 6px 4px 2px;
  }
  .list button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 4px 8px;
    /* Les lignes hors de vue ne chargent pas leur police. */
    content-visibility: auto;
    contain-intrinsic-size: auto 30px;
  }
  .list button:hover {
    background: #eef5fc;
  }
  .list button.on {
    background: #e8f1fb;
  }
  .name {
    flex: 1;
    font-size: 16px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .warn {
    font-size: 10px;
    border-radius: 8px;
    padding: 1px 6px;
    background: #fff1e0;
    color: #b9770e;
    white-space: nowrap;
  }
  .none {
    color: var(--muted);
    margin: 6px;
  }
</style>

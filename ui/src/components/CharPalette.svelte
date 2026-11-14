<script lang="ts">
  // Les lettres et signes difficiles à taper (É, À, Ç, guillemets « »...), pendant l'écriture d'un texte : un clic
  // les écrit au curseur, sans quitter l'écriture. Se ferme avec Échap, un clic ailleurs ou le même bouton.
  import { tick } from "svelte";
  import type { EditorState, Transaction } from "@tiptap/pm/state";
  import Icon from "./Icon.svelte";
  import { PALETTE_CHARS, insertChar, insertGuillemets } from "../lib/frenchTypo";
  import { fr } from "../lib/i18n/fr";
  import { fitInWindow } from "../lib/popover";
  import { textEditing } from "../lib/textEditing.svelte";

  let open = $state(false);
  let root = $state<HTMLElement>();
  let list = $state<HTMLElement>();

  // L'écriture finie (Échap, Tab dans un tableau...) : la palette se referme, elle ne revient pas ouverte.
  $effect(() => {
    if (!textEditing.current) open = false;
  });

  const items = () => [...(list?.querySelectorAll<HTMLButtonElement>("button") ?? [])];

  async function toggle(e: MouseEvent) {
    open = !open;
    // Ouvert au clavier : le premier signe prend le focus. À la souris, le curseur reste dans le texte.
    if (open && e.detail === 0) {
      await tick();
      items()[0]?.focus();
    }
  }

  /** Écrit dans le texte en cours, puis lui rend le focus : on continue de taper. */
  function write(make: (state: EditorState) => Transaction) {
    const tip = textEditing.current;
    if (!tip) return;
    tip.view.dispatch(make(tip.state));
    tip.view.focus();
  }

  function onKey(e: KeyboardEvent) {
    const all = items();
    const i = all.indexOf(document.activeElement as HTMLButtonElement);
    const step = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
    if (e.key === "Escape") {
      // Échap ferme la palette et revient au texte, sans fermer l'écriture.
      e.preventDefault();
      e.stopPropagation();
      open = false;
      textEditing.current?.view.focus();
    } else if (step && i >= 0) {
      e.preventDefault();
      all[(i + step + all.length) % all.length].focus();
    } else if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      all[e.key === "Home" ? 0 : all.length - 1]?.focus();
    }
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }}
/>

{#if textEditing.current}
  <div class="chars" bind:this={root}>
    <button class="trigger" class:on={open} title={fr.charPaletteHint} aria-label={fr.charPalette} aria-haspopup="menu" aria-expanded={open} onclick={toggle}>
      É « » <Icon name="chevronDown" size={14} />
    </button>
    {#if open}
      <div use:fitInWindow class="pop" role="menu" tabindex="-1" aria-label={fr.charPalette} bind:this={list} onkeydown={onKey}>
        <div class="grid">
          {#each PALETTE_CHARS as c (c)}
            <button role="menuitem" title={fr.charInsert(c)} aria-label={fr.charInsert(c)} onclick={() => write((s) => insertChar(s, c))}>{c}</button>
          {/each}
        </div>
        <button class="quotes" role="menuitem" title={fr.charGuillemetsHint} onclick={() => write(insertGuillemets)}>« » <span>{fr.charGuillemets}</span></button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .chars {
    position: relative;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border-color: transparent;
    background: transparent;
    padding: 4px 8px;
    white-space: nowrap;
  }
  .trigger:hover {
    background: var(--bg);
  }
  .trigger.on {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .pop {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px var(--shadow);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 10;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(7, 2.2em);
    gap: 4px;
  }
  .grid button {
    font-size: 18px;
    padding: 4px 0;
    text-align: center;
  }
  .quotes {
    font-size: 18px;
    padding: 4px 8px;
  }
  .quotes span {
    font-size: 14px;
  }
  .grid button:hover,
  .quotes:hover {
    background: var(--accent-soft);
  }
</style>

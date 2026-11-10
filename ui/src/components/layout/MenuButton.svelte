<script lang="ts" module>
  import type { IconName } from "../../lib/icons";

  export interface MenuItem {
    label: string;
    /** Icône dans la colonne de gauche. */
    icon?: IconName;
    /** Réglage qu'on coche : une coche dans la colonne des icônes quand il est vrai. */
    checked?: boolean;
    /** Petit titre de section, qu'on ne clique pas. */
    heading?: boolean;
    shortcut?: string;
    title?: string;
    disabled?: boolean;
    /** Trait de séparation avant l'entrée. */
    separator?: boolean;
    /** Sous-menu : il s'ouvre au survol ou au clic, à côté de l'entrée. */
    children?: MenuItem[];
    action?: () => unknown;
  }
</script>

<script lang="ts">
  // Bouton qui ouvre une liste de commandes ; se ferme avec Échap, un clic ailleurs ou après un choix.
  import Icon from "../Icon.svelte";
  import { fitInWindow } from "../../lib/popover";

  let {
    label,
    items,
    primary = false,
    icon,
    title,
  }: { label: string; items: MenuItem[]; primary?: boolean; icon?: IconName; title?: string } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();
  /** Entrée dont le sous-menu est ouvert. */
  let sub = $state<number | null>(null);

  $effect(() => {
    if (!open) sub = null;
  });

  function onWindowPointer(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }

  function choose(item: MenuItem) {
    open = false;
    item.action?.();
  }
</script>

<svelte:window onpointerdown={onWindowPointer} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<!-- Colonne des icônes de 16 px, vide s'il n'y en a pas : les libellés restent alignés. -->
{#snippet ico(item: MenuItem)}
  <span class="ico">
    {#if item.checked}<Icon name="check" />{:else if item.icon}<Icon name={item.icon} />{/if}
  </span>
{/snippet}

{#snippet entry(item: MenuItem, onenter?: () => void)}
  {#if item.heading}
    <div class="heading" role="presentation" onpointerenter={onenter}>{item.label}</div>
  {:else}
    <button
      role={item.checked === undefined ? "menuitem" : "menuitemcheckbox"}
      aria-checked={item.checked}
      class="item"
      title={item.title}
      disabled={item.disabled}
      onpointerenter={onenter}
      onclick={() => choose(item)}
    >
      {@render ico(item)}
      <span class="label">{item.label}</span>
      {#if item.shortcut}<kbd>{item.shortcut}</kbd>{/if}
    </button>
  {/if}
{/snippet}

<div class="menu" bind:this={root}>
  <button class="trigger" class:primary class:ghost={!primary} {title} aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    {#if icon}<Icon name={icon} />{/if}{label}<Icon name="chevronDown" size={14} />
  </button>
  {#if open}
    <div use:fitInWindow={4} class="list" class:right={primary} role="menu">
      {#each items as item, i}
        {#if item.separator}<hr />{/if}
        {#if item.children}
          <div class="sub-wrap" role="presentation" onpointerenter={() => (sub = i)}>
            <button role="menuitem" class="item" class:active={sub === i} aria-haspopup="menu" aria-expanded={sub === i} title={item.title} disabled={item.disabled} onclick={() => (sub = i)}>
              {@render ico(item)}
              <span class="label">{item.label}</span>
              <span class="arrow" aria-hidden="true"><Icon name="chevronRight" size={14} /></span>
            </button>
            {#if sub === i}
              <div class="list sub" class:left={primary} role="menu">
                {#each item.children as child}
                  {#if child.separator}<hr />{/if}
                  {@render entry(child)}
                {/each}
              </div>
            {/if}
          </div>
        {:else}
          {@render entry(item, () => (sub = null))}
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu {
    position: relative;
  }
  .trigger {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .ghost {
    border-color: transparent;
    background: transparent;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
  .primary:not(:disabled):hover {
    filter: brightness(1.08);
  }
  .list {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    min-width: 230px;
    background: white;
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.15);
    padding: 4px;
    z-index: 4000; /* au-dessus des poignées Moveable (3000) */
  }
  .list.right {
    left: auto;
    right: 0;
  }
  .sub-wrap {
    position: relative;
  }
  /* Sous-menu à droite de son entrée (à gauche pour un menu calé à droite), et défilant s'il est long. */
  .list.sub {
    top: -5px;
    left: calc(100% + 4px);
    max-height: 70vh;
    overflow-y: auto;
  }
  .list.sub.left {
    left: auto;
    right: calc(100% + 4px);
  }
  .arrow {
    display: flex;
    color: var(--muted);
  }
  .item.active {
    background: #eef5fc;
  }
  /* Icône, libellé, puis le raccourci calé à droite. */
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 6px 10px;
  }
  .ico {
    flex: none;
    display: flex;
    width: 16px;
    height: 16px;
  }
  .label {
    flex: 1;
  }
  .heading {
    padding: 6px 10px 2px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .item:not(:disabled):hover {
    background: #eef5fc;
  }
  kbd {
    font: inherit;
    color: var(--muted);
    margin-left: 16px;
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 3px 0;
  }
</style>

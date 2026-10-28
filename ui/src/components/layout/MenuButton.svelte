<script lang="ts" module>
  export interface MenuItem {
    label: string;
    shortcut?: string;
    title?: string;
    disabled?: boolean;
    /** Trait de séparation avant l'entrée. */
    separator?: boolean;
    action: () => unknown;
  }
</script>

<script lang="ts">
  // Bouton qui ouvre une liste de commandes ; se ferme avec Échap, un clic ailleurs ou après un choix.
  let { label, items, primary = false }: { label: string; items: MenuItem[]; primary?: boolean } = $props();

  let open = $state(false);
  let root = $state<HTMLElement>();

  function onWindowPointer(e: PointerEvent) {
    if (open && root && !root.contains(e.target as Node)) open = false;
  }

  function choose(item: MenuItem) {
    open = false;
    item.action();
  }
</script>

<svelte:window onpointerdown={onWindowPointer} onkeydown={(e) => e.key === "Escape" && (open = false)} />

<div class="menu" bind:this={root}>
  <button class="trigger" class:primary class:ghost={!primary} aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    {label} ▾
  </button>
  {#if open}
    <div class="list" class:right={primary} role="menu">
      {#each items as item}
        {#if item.separator}<hr />{/if}
        <button role="menuitem" class="item" title={item.title} disabled={item.disabled} onclick={() => choose(item)}>
          <span>{item.label}</span>
          {#if item.shortcut}<kbd>{item.shortcut}</kbd>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu {
    position: relative;
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
  .item {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 6px 10px;
  }
  .item:not(:disabled):hover {
    background: #eef5fc;
  }
  kbd {
    font: inherit;
    color: var(--muted);
  }
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 3px 0;
  }
</style>

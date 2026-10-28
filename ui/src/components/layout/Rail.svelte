<script lang="ts">
  import { editor, type DrawerId } from "../../lib/docStore.svelte";
  import { pickLogo } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";

  const normal: { id: DrawerId; icon: string; label: string }[] = [
    { id: "templates", icon: "▦", label: fr.templates },
    { id: "photos", icon: "🖼", label: fr.photos },
    { id: "text", icon: "T", label: fr.text },
    { id: "shapes", icon: "◯", label: fr.shapes },
    { id: "fonts", icon: "Aa", label: fr.fonts },
    { id: "decor", icon: "✿", label: fr.decor },
  ];
  const template: { id: DrawerId; icon: string; label: string }[] = [
    { id: "grid", icon: "▦", label: fr.grid },
    { id: "text", icon: "T", label: fr.text },
    { id: "shapes", icon: "◯", label: fr.shapes },
    { id: "decor", icon: "✿", label: fr.decor },
  ];
  const items = $derived(editor.templateMode ? template : normal);
</script>

<nav class="rail" aria-label="Outils">
  {#each items as item (item.id)}
    <button class="item" class:on={editor.drawer === item.id} aria-pressed={editor.drawer === item.id} onclick={() => editor.toggleDrawer(item.id)}>
      <b>{item.icon}</b>{item.label}
    </button>
  {/each}
  {#if editor.templateMode}
    <button class="item" onclick={pickLogo}><b>⭐</b>{fr.logo}</button>
  {/if}
</nav>

<style>
  .rail {
    grid-area: rail;
    width: 68px;
    background: white;
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 4px;
  }
  .item {
    border: 0;
    background: transparent;
    border-radius: 8px;
    padding: 7px 0;
    font-size: 11px;
    color: #555;
    text-align: center;
  }
  .item b {
    display: block;
    font-size: 19px;
    font-weight: 400;
  }
  .item:not(:disabled):hover {
    background: #f3efe8;
  }
  .item.on {
    background: #e8f1fb;
    color: var(--accent);
  }
</style>

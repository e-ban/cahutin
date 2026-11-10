<script lang="ts">
  import { editor, type DrawerId } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import type { IconName } from "../../lib/icons";
  import Icon from "../Icon.svelte";

  const normal: { id: DrawerId; icon: IconName; label: string }[] = [
    { id: "templates", icon: "template", label: fr.templates },
    { id: "images", icon: "images", label: fr.images },
    { id: "text", icon: "text", label: fr.text },
    { id: "fields", icon: "braces", label: fr.fieldsTab },
    { id: "shapes", icon: "shapes", label: fr.shapes },
    { id: "fonts", icon: "fonts", label: fr.fonts },
    { id: "decor", icon: "decor", label: fr.decor },
    { id: "class", icon: "class", label: fr.classDrawer },
  ];
  const template: { id: DrawerId; icon: IconName; label: string }[] = [
    { id: "grid", icon: "grid", label: fr.grid },
    { id: "text", icon: "text", label: fr.text },
    { id: "fields", icon: "braces", label: fr.fieldsTab },
    { id: "shapes", icon: "shapes", label: fr.shapes },
    { id: "images", icon: "images", label: fr.images },
    { id: "decor", icon: "decor", label: fr.decor },
  ];
  const items = $derived(editor.templateMode ? template : normal);
</script>

<nav class="rail" aria-label="Outils">
  {#each items as item (item.id)}
    <button class="item" data-tour={item.id} class:on={editor.drawer === item.id} aria-pressed={editor.drawer === item.id} onclick={() => editor.toggleDrawer(item.id)}>
      <span class="pill"><Icon name={item.icon} size={20} /></span>{item.label}
    </button>
  {/each}
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
  /* Chaque entrée : l'icône dans une pastille arrondie, le nom dessous. */
  .item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    border: 0;
    background: transparent;
    border-radius: 8px;
    padding: 4px 0;
    font-size: 11px;
    color: #555;
    text-align: center;
  }
  .pill {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 38px;
    height: 30px;
    border-radius: 10px;
  }
  .item:not(:disabled):hover .pill {
    background: #f3efe8;
  }
  /* Le tiroir ouvert : pastille teintée et nom en gras. */
  .item.on {
    color: var(--accent);
    font-weight: 700;
  }
  .item.on .pill,
  .item.on:not(:disabled):hover .pill {
    background: #e8f1fb;
  }
  :global(:root[data-contrast]) .item.on .pill {
    outline: 2px solid var(--accent);
  }
</style>

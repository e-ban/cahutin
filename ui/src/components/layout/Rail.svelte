<script lang="ts">
  import { editor, type DrawerId } from "../../lib/docStore.svelte";
  import { pickLogo } from "../../lib/fileActions";
  import { centeredFrame, photoZoneElement, textElement } from "../../lib/factory";
  import type { Element } from "../../lib/types";

  /** Mode modèle : une zone libre de plus, sélectionnée, à placer où l'on veut. */
  function addZone(el: Element) {
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }
  import { fr } from "../../lib/i18n/fr";

  const normal: { id: DrawerId; icon: string; label: string }[] = [
    { id: "templates", icon: "▦", label: fr.templates },
    { id: "images", icon: "🖼", label: fr.images },
    { id: "text", icon: "T", label: fr.text },
    { id: "shapes", icon: "◯", label: fr.shapes },
    { id: "fonts", icon: "Aa", label: fr.fonts },
    { id: "decor", icon: "✿", label: fr.decor },
    { id: "class", icon: "🧒", label: fr.classDrawer },
  ];
  const template: { id: DrawerId; icon: string; label: string }[] = [
    { id: "grid", icon: "▦", label: fr.grid },
    { id: "text", icon: "T", label: fr.text },
    { id: "shapes", icon: "◯", label: fr.shapes },
    { id: "images", icon: "🖼", label: fr.images },
    { id: "decor", icon: "✿", label: fr.decor },
  ];
  const items = $derived(editor.templateMode ? template : normal);
</script>

<nav class="rail" aria-label="Outils">
  {#each items as item (item.id)}
    <button class="item" data-tour={item.id} class:on={editor.drawer === item.id} aria-pressed={editor.drawer === item.id} onclick={() => editor.toggleDrawer(item.id)}>
      <b>{item.icon}</b>{item.label}
    </button>
  {/each}
  {#if editor.templateMode}
    <button class="item" title={fr.photoZoneHint} onclick={() => addZone(photoZoneElement(editor.page))}><b>🖼</b>{fr.photoZone}</button>
    <button class="item" title={fr.textZoneHint} onclick={() => addZone(textElement(centeredFrame(editor.page, 120, 20), true))}><b>✎</b>{fr.textZone}</button>
    <button class="item" title={fr.logoHint} onclick={pickLogo}><b>⭐</b>{fr.logo}</button>
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

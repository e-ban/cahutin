<script lang="ts" module>
  export interface ContextTarget {
    /** Élément visé, ou null pour la page vide. */
    id: string | null;
    xMm: number;
    yMm: number;
    clientX: number;
    clientY: number;
  }
</script>

<script lang="ts">
  // Menu du clic droit : sur un élément, ou sur la page vide.
  import { editor } from "../../lib/docStore.svelte";
  import { bringSelection, copySelection, cutSelection, deleteSelection, duplicateSelection, groupSelection, hasClipboard, paste, toggleLockSelection, ungroupSelection } from "../../lib/editActions";
  import { shapeElement, textElement } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";
  import Icon from "../Icon.svelte";
  import type { IconName } from "../../lib/icons";

  let { target, onclose }: { target: ContextTarget; onclose: () => void } = $props();

  let menuEl = $state<HTMLElement>();
  const element = $derived(target.id ? editor.page.elements.find((e) => e.id === target.id) : undefined);

  // Reste dans la fenêtre même près des bords.
  const left = $derived(Math.min(target.clientX, window.innerWidth - (menuEl?.offsetWidth ?? 220) - 8));
  const top = $derived(Math.min(target.clientY, window.innerHeight - (menuEl?.offsetHeight ?? 300) - 8));

  // L'action passe avant la fermeture : fermer détruit le menu, et `target` n'est plus lisible ensuite.
  function run(action: () => void) {
    try {
      action();
    } finally {
      onclose();
    }
  }

  // Fixer une case (modèle) : une punaise ; sinon le cadenas, ouvert ou fermé selon ce qu'on va faire.
  const lockIcon = $derived<IconName>(editor.templateMode ? "pin" : element?.locked ? "unlock" : "lock");

  function addHere(kind: "text" | "shape") {
    const at = { x_mm: target.xMm, y_mm: target.yMm };
    const el = kind === "text" ? textElement({ x_mm: Math.round(at.x_mm - 40), y_mm: Math.round(at.y_mm - 10), w_mm: 80, h_mm: 20, rotation_deg: 0 }) : shapeElement(editor.page, "Rect", 0, at);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
    if (kind === "text") editor.editingText = el.id;
  }
</script>

<svelte:window
  onpointerdown={(e) => {
    if (menuEl && !menuEl.contains(e.target as Node)) onclose();
  }}
  onkeydown={(e) => e.key === "Escape" && onclose()}
  onblur={onclose}
/>

<div class="menu" role="menu" bind:this={menuEl} style:left="{left}px" style:top="{top}px">
  {#if element}
    <button role="menuitem" disabled={element.locked && !editor.templateMode} onclick={() => run(cutSelection)}><span class="ico"><Icon name="cut" /></span><span>{fr.cut}</span><kbd>Ctrl+X</kbd></button>
    <button role="menuitem" onclick={() => run(copySelection)}><span class="ico"><Icon name="copy" /></span><span>{fr.copy}</span><kbd>Ctrl+C</kbd></button>
    <button role="menuitem" disabled={!hasClipboard()} onclick={() => run(paste)}><span class="ico"><Icon name="clipboard" /></span><span>{fr.paste}</span><kbd>Ctrl+V</kbd></button>
    <button role="menuitem" onclick={() => run(duplicateSelection)}><span class="ico"><Icon name="duplicate" /></span><span>{fr.duplicate}</span><kbd>Ctrl+D</kbd></button>
    {#if editor.selection.length > 1 || element.group}
      <hr />
      {#if element.group}
        <button role="menuitem" onclick={() => run(ungroupSelection)}><span class="ico"><Icon name="ungroup" /></span><span>{fr.ungroup}</span><kbd>Ctrl+Maj+G</kbd></button>
      {:else}
        <button role="menuitem" onclick={() => run(groupSelection)}><span class="ico"><Icon name="group" /></span><span>{fr.group}</span><kbd>Ctrl+G</kbd></button>
      {/if}
    {/if}
    <hr />
    <button role="menuitem" onclick={() => run(() => bringSelection("front"))}><span class="ico"><Icon name="bringFront" /></span><span>{fr.bringFront}</span></button>
    <button role="menuitem" onclick={() => run(() => bringSelection("back"))}><span class="ico"><Icon name="sendBack" /></span><span>{fr.sendBack}</span></button>
    <hr />
    <button role="menuitem" onclick={() => run(toggleLockSelection)}><span class="ico"><Icon name={lockIcon} /></span><span>{editor.templateMode ? (element.locked ? fr.unfixCell : fr.fixCell) : element.locked ? fr.unlock : fr.lock}</span></button>
    <button role="menuitem" disabled={element.locked && !editor.templateMode} onclick={() => run(deleteSelection)}><span class="ico"><Icon name="trash" /></span><span>{fr.delete}</span><kbd>Suppr</kbd></button>
  {:else}
    <button role="menuitem" disabled={!hasClipboard()} onclick={() => run(paste)}><span class="ico"><Icon name="clipboard" /></span><span>{fr.paste}</span><kbd>Ctrl+V</kbd></button>
    <button role="menuitem" onclick={() => run(() => addHere("text"))}><span class="ico"><Icon name="text" /></span><span>{fr.addTextHere}</span></button>
    <button role="menuitem" onclick={() => run(() => addHere("shape"))}><span class="ico"><Icon name="shapes" /></span><span>{fr.addShapeHere}</span></button>
    <hr />
    <button role="menuitem" onclick={() => run(() => (editor.showBackground = true))}><span class="ico"><Icon name="background" /></span><span>{fr.backgroundMenu}</span></button>
  {/if}
</div>

<style>
  .menu {
    position: fixed;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px var(--shadow);
    padding: 4px;
    min-width: 210px;
    z-index: 4000; /* au-dessus des poignées Moveable (3000) */
  }
  /* Icône, libellé, puis le raccourci calé à droite. */
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    border: 0;
    background: transparent;
    text-align: left;
    padding: 5px 10px;
  }
  button:not(:disabled):hover {
    background: var(--accent-soft);
  }
  .ico {
    flex: none;
    display: flex;
    width: 16px;
    height: 16px;
  }
  .ico + span {
    flex: 1;
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

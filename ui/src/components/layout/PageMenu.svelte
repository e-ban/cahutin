<script lang="ts">
  // Menu du clic droit sur une miniature de la bande des pages : commencer un chapitre à cette page (ou renommer celui
  // qu'elle ouvre), la dupliquer, la supprimer.
  import Icon from "../Icon.svelte";
  import { chapterSpans, startChapter } from "../../lib/chapters";
  import { editor } from "../../lib/docStore.svelte";
  import { deletePage } from "../../lib/fileActions";
  import { fr } from "../../lib/i18n/fr";
  import { duplicatePage } from "../../lib/pageOrder";

  let { index, x, y, onclose }: { index: number; x: number; y: number; onclose: () => void } = $props();

  let menuEl = $state<HTMLElement>();
  const page = $derived(editor.doc.pages[index]);
  const left = $derived(Math.min(x, window.innerWidth - (menuEl?.offsetWidth ?? 240) - 8));
  const top = $derived(Math.min(y, window.innerHeight - (menuEl?.offsetHeight ?? 160) - 8));

  function run(action: () => void) {
    try {
      action();
    } finally {
      onclose();
    }
  }

  /** Ouvre Réglages sur cette page, le curseur dans le nom de son chapitre. */
  function nameChapter() {
    editor.pageIndex = index;
    editor.selection = [];
    editor.inspectorOpen = true;
    editor.focusChapterName = true;
  }

  function newChapter() {
    editor.commit(startChapter(editor.doc, index, fr.chapterDefault(chapterSpans(editor.doc.pages).length + 1)));
    nameChapter();
  }

  function duplicate() {
    const pages = [...editor.doc.pages];
    pages.splice(index + 1, 0, duplicatePage(pages[index]));
    editor.commit({ ...editor.doc, pages });
    editor.pageIndex = index + 1;
    editor.selection = [];
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
  {#if page?.chapter}
    <button role="menuitem" onclick={() => run(nameChapter)}><span class="ico"><Icon name="text" /></span><span>{fr.pageMenuRenameChapter}</span></button>
  {:else}
    <button role="menuitem" onclick={() => run(newChapter)}><span class="ico"><Icon name="add" /></span><span>{fr.chapterStart}</span></button>
  {/if}
  <hr />
  <button role="menuitem" onclick={() => run(duplicate)}><span class="ico"><Icon name="duplicate" /></span><span>{fr.pageMenuDuplicate}</span></button>
  <button role="menuitem" disabled={editor.doc.pages.length === 1} onclick={() => run(() => deletePage(index))}><span class="ico"><Icon name="trash" /></span><span>{fr.deletePage}</span></button>
</div>

<style>
  .menu {
    position: fixed;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 6px 20px var(--shadow);
    padding: 4px;
    min-width: 220px;
    z-index: 4000;
  }
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
  hr {
    border: 0;
    border-top: 1px solid var(--line);
    margin: 3px 0;
  }
</style>

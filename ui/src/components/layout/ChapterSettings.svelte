<script lang="ts">
  // Réglages de la page : le chapitre qu'elle ouvre (nom, couleur, place, retrait), ou de quoi en commencer un.
  import { tick } from "svelte";
  import ColorPicker from "../ColorPicker.svelte";
  import Icon from "../Icon.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { chapterOf, chapterSpans, moveChapter, removeChapter, startChapter, updateChapter } from "../../lib/chapters";

  const i = $derived(editor.pageIndex);
  const own = $derived(editor.page.chapter ?? null);
  const inside = $derived(own ? null : chapterOf(editor.doc.pages, i));
  const spans = $derived(chapterSpans(editor.doc.pages));
  const k = $derived(spans.findIndex((c) => c.start === i));
  let name = $state<HTMLInputElement>();

  async function start() {
    editor.commit(startChapter(editor.doc, i, fr.chapterDefault(spans.length + 1)));
    await tick();
    name?.focus();
    name?.select();
  }

  function rename(value: string) {
    const title = value.trim();
    if (title && title !== own?.title) editor.commit(updateChapter(editor.doc, i, { title }));
    else if (name && own) name.value = own.title;
  }

  function move(dir: -1 | 1) {
    const moved = moveChapter(editor.doc, i, dir);
    if (!moved) return;
    editor.commit(moved.doc);
    editor.pageIndex = moved.start;
    editor.selection = [];
  }
</script>

<div class="field">
  <span>{fr.chapter}</span>
  {#if own}
    <input bind:this={name} value={own.title} aria-label={fr.chapterName} placeholder={fr.chapterName} onchange={(e) => rename(e.currentTarget.value)} onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()} />
    <span class="sub">{fr.chapterColor}</span>
    <ColorPicker value={own.color} onchange={(c) => c && editor.commit(updateChapter(editor.doc, i, { color: c }))} />
    <div class="row" title={fr.chapterMoveHint}>
      <button disabled={k <= 0} onclick={() => move(-1)}><Icon name="arrowLeft" /> {fr.chapterMoveBefore}</button>
      <button disabled={k < 0 || k >= spans.length - 1} onclick={() => move(1)}>{fr.chapterMoveAfter} <Icon name="arrowRight" /></button>
    </div>
    <button title={fr.chapterRemoveHint} onclick={() => editor.commit(removeChapter(editor.doc, i))}><Icon name="trash" /> {fr.chapterRemove}</button>
  {:else}
    <p class="hint">{inside ? fr.chapterIn(inside.title) : fr.chapterStartHint}</p>
    <button onclick={start}><Icon name="add" /> {fr.chapterStart}</button>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
  }
  .hint,
  .sub {
    color: var(--muted);
    margin: 0;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .row button {
    flex: 1;
    white-space: nowrap;
  }
</style>

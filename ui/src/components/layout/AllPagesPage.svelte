<script lang="ts">
  // Vue « Toutes les pages » : le carnet en grande grille, rangé par chapitre. On choisit des pages d'un clic (comme
  // dans « Ma collection »), puis on les range dans un chapitre, on les duplique ou on les supprime, en un seul pas
  // d'annulation. « Ouvrir » (ou un double-clic) mène à la page.
  import PageView from "../PageView.svelte";
  import Icon from "../Icon.svelte";
  import { ask } from "../../lib/ask";
  import { chapterSpans } from "../../lib/chapters";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { deletePages, duplicatePages, followPage, keepsPages, movePagesTo } from "../../lib/manyPages";
  import { fitZoom } from "../../lib/pageFormat";
  import type { Document } from "../../lib/types";

  // Chaque miniature tient dans la place d'une page A4 en portrait : une page en paysage ou en A3 est réduite.
  const ZOOM = 0.16;

  let selected = $state<string[]>([]);
  /** Le chapitre où ranger les pages : l'identifiant de la page qui l'ouvre, ou « -1 » pour le début du carnet. Il reste
   *  le même quand les chapitres changent de place. */
  let target = $state("");

  const pages = $derived(editor.doc.pages);
  const spans = $derived(chapterSpans(pages));
  /** Les parties de la grille : les pages d'avant le premier chapitre (s'il y en a), puis chaque chapitre. */
  const sections = $derived([
    ...(spans.length === 0 || spans[0].start > 0 ? [{ title: spans.length ? fr.allPagesNoChapter : "", color: "", start: 0, end: spans[0]?.start ?? pages.length }] : []),
    ...spans,
  ]);
  /** Les chapitres où ranger les pages choisies : ceux qui gardent au moins une page à eux. */
  const targets = $derived(spans.filter((c) => keepsPages(pages, c.start, selected)));

  // La liste du choix suit les chapitres : une valeur qui n'existe plus revient au premier.
  $effect(() => {
    const values = [...(spans.length && spans[0].start > 0 ? ["-1"] : []), ...targets.map((c) => pages[c.start].id)];
    if (!values.includes(target)) target = values[0] ?? "";
  });

  function close() {
    editor.showAllPages = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape" && !(e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement)) close();
  }

  function openPage(i: number) {
    editor.pageIndex = i;
    editor.selection = [];
    close();
  }

  const toggle = (id: string) => (selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id]);

  function chooseSection(start: number, end: number) {
    const ids = pages.slice(start, end).map((p) => p.id);
    selected = [...selected.filter((id) => !ids.includes(id)), ...ids];
  }

  /** Remplace le document en un pas ; la page affichée reste la même (ou sa voisine si elle a disparu). */
  function apply(next: Document) {
    if (next === editor.doc) return false;
    const before = editor.doc.pages;
    const index = editor.pageIndex;
    editor.commit(next);
    editor.pageIndex = followPage(before, next.pages, index);
    editor.selection = [];
    return true;
  }

  function move() {
    const start = target === "-1" ? -1 : pages.findIndex((p) => p.id === target);
    if (start < 0 && target !== "-1") return;
    const where = start >= 0 ? pages[start].chapter?.title : fr.allPagesNoChapter;
    if (!apply(movePagesTo(editor.doc, selected, start))) return;
    editor.notice = fr.allPagesMoved(selected.length, where ?? "");
    selected = [];
  }

  async function remove() {
    if (selected.length >= pages.length) return void (editor.notice = fr.allPagesNotAll);
    if (!(await ask(fr.allPagesDeleteConfirm(selected.length), fr.delete))) return;
    if (apply(deletePages(editor.doc, selected))) selected = [];
  }

  function scrollTo(start: number) {
    document.getElementById(`all-pages-${start}`)?.scrollIntoView({ block: "start", behavior: "smooth" });
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="all-pages">
  <div class="top">
    <button class="back" onclick={close}><Icon name="back" /> {fr.classPageBack}</button>
    <h1><Icon name="grid" size={24} /> {fr.allPages} <span class="n">{fr.allPagesCount(pages.length)}</span></h1>
  </div>
  <p class="intro">{fr.allPagesIntro}</p>
  <div class="layout">
    {#if spans.length}
      <aside class="chapters" aria-label={fr.chapter}>
        {#each sections as c (c.start)}
          <button style:--chapter={c.color || "var(--line)"} onclick={() => scrollTo(c.start)}>{c.title} <span class="n">{c.end - c.start}</span></button>
        {/each}
      </aside>
    {/if}
    <section aria-label={fr.allPages}>
      <div class="toolbar" class:active={selected.length > 0}>
        {#if selected.length}
          <b>{fr.allPagesChosen(selected.length)}</b>
          {#if spans.length}
            <label>
              {fr.allPagesMoveTo}
              <select bind:value={target}>
                {#if spans[0].start > 0}<option value="-1">{fr.allPagesNoChapter}</option>{/if}
                {#each targets as c (c.start)}<option value={pages[c.start].id}>{c.title}</option>{/each}
              </select>
            </label>
            <button class="primary" disabled={target === ""} onclick={move}>{fr.allPagesMove}</button>
          {/if}
          <button onclick={() => apply(duplicatePages(editor.doc, selected))}><Icon name="duplicate" /> {fr.duplicatePage}</button>
          <button onclick={remove}><Icon name="trash" /> {fr.delete}</button>
          <button class="plain" onclick={() => (selected = [])}>{fr.collectionUnselect}</button>
        {:else}
          <span class="hint">{fr.allPagesChooseHint}</span>
          <button class="plain" onclick={() => (selected = pages.map((p) => p.id))}>{fr.librarySelectAll}</button>
        {/if}
      </div>
      {#each sections as c (c.start)}
        <div class="section" id={`all-pages-${c.start}`} style:--chapter={c.color || "var(--line)"}>
          {#if c.title}
            <h2>
              <span class="title">{c.title}</span>
              <span class="n">{fr.allPagesCount(c.end - c.start)}</span>
              <button class="plain" onclick={() => chooseSection(c.start, c.end)}>{fr.allPagesChooseChapter}</button>
            </h2>
          {/if}
          <div class="grid">
            {#each pages.slice(c.start, c.end) as p, k (p.id)}
              {@const i = c.start + k}
              {@const chosen = selected.includes(p.id)}
              <div class="card" class:chosen class:current={i === editor.pageIndex}>
                <button class="pic" aria-pressed={chosen} aria-label={fr.pageN(i + 1)} title={fr.allPagesChooseHint} onclick={() => toggle(p.id)} ondblclick={() => openPage(i)}>
                  <PageView page={p} doc={editor.doc} zoom={fitZoom(p.size, ZOOM, 250, 297)} />
                  {#if chosen}<span class="tick" aria-hidden="true"><Icon name="check" size={14} /></span>{/if}
                </button>
                <div class="foot">
                  <span>{fr.pageN(i + 1)}</span>
                  <button onclick={() => openPage(i)}>{fr.allPagesOpen}</button>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </section>
  </div>
</div>

<style>
  .all-pages {
    position: fixed;
    inset: 0;
    background: var(--bg);
    padding: 24px clamp(16px, 3vw, 40px);
    overflow: auto;
    z-index: 5100;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  h1 {
    font: 600 24px var(--ui-font);
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .intro {
    margin: 6px 0 14px;
    color: var(--muted);
  }
  .back {
    border-color: transparent;
    background: transparent;
  }
  .layout {
    display: flex;
    gap: 16px;
    align-items: start;
  }
  section {
    flex: 1;
    min-width: 0;
  }
  .chapters {
    flex: none;
    width: 190px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: sticky;
    top: 0;
  }
  .chapters > button {
    display: flex;
    justify-content: space-between;
    text-align: left;
    border-color: transparent;
    border-left: 4px solid var(--chapter);
    background: transparent;
  }
  .n {
    color: var(--muted);
    font-weight: normal;
    font-size: 14px;
  }
  .toolbar {
    position: sticky;
    top: -24px;
    z-index: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    min-height: 40px;
    padding: 4px 8px;
    margin-bottom: 10px;
    border-radius: 8px;
    background: var(--bg);
  }
  .toolbar.active {
    background: var(--accent-soft);
  }
  .toolbar label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .hint {
    color: var(--muted);
  }
  .primary {
    background: var(--accent);
    color: var(--on-accent);
    border-color: var(--accent);
  }
  .plain {
    border-color: transparent;
    background: transparent;
    text-decoration: underline;
  }
  .section {
    margin-bottom: 20px;
    /* Amené par la liste des chapitres, son titre reste visible sous la barre des actions. */
    scroll-margin-top: 64px;
  }
  h2 {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin: 0 0 8px;
    padding-left: 10px;
    border-left: 6px solid var(--chapter);
    font: 600 18px var(--ui-font);
  }
  h2 .plain {
    font: 14px var(--ui-font);
    padding: 2px 6px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 12px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--surface);
    border: 2px solid var(--line);
    border-radius: 10px;
  }
  .card.current {
    border-color: var(--chapter);
  }
  .card.chosen {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .pic {
    position: relative;
    padding: 4px;
    border: 0;
    background: transparent;
    display: flex;
    justify-content: center;
    align-items: center;
    /* La hauteur d'une page A4 en portrait : les cartes d'une rangée restent alignées. */
    min-height: 196px;
  }
  .pic :global(.page) {
    pointer-events: none;
  }
  .tick {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--on-accent);
    display: grid;
    place-items: center;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 13px;
  }
  .foot button {
    padding: 2px 8px;
  }
</style>

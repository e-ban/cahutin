<script lang="ts">
  // Vue agrandie du tiroir Images : les photos du document, la photothèque de la classe et la collection, avec la
  // même disposition (filtres à gauche, images groupées au centre, barre d'ajout en bas). On en choisit plusieurs
  // pour les ajouter à la page, ou on en glisse une : la vue se replie pendant le glisser pour découvrir la page.
  import Icon from "../Icon.svelte";
  import { classes } from "../../lib/classStore.svelte";
  import { CLASS_PHOTO_MIME, ASSET_MIME, MY_IMAGE_MIME } from "../../lib/dnd";
  import { editor } from "../../lib/docStore.svelte";
  import { DRAWN_STEP, drawMoreNearEnd } from "../../lib/drawMore";
  import { askRemoveUnusedPhotos, unusedPhotoIds, useImages } from "../../lib/editActions";
  import { fr } from "../../lib/i18n/fr";
  import { libraryView, mineView, photosView, type Source, type WideFilter } from "../../lib/imageSources";
  import { myImages } from "../../lib/myImages.svelte";
  import { startFilter, type PhotoFilter } from "../../lib/photoLibrary";
  import { firstDays } from "../../lib/phototheque";
  import { classLibrary } from "../../lib/phototheque.svelte";
  import { readPref, writePref } from "../../lib/prefs";
  import { pick, toggleAll } from "../../lib/selection";
  import type { AssetMeta } from "../../lib/types";

  let { ondrag }: { ondrag: (dragging: boolean) => void } = $props();

  const SIZES = [
    { px: 72, label: fr.thumbSmall, iconSize: 10 },
    { px: 96, label: fr.thumbMedium, iconSize: 13 },
    { px: 140, label: fr.thumbLarge, iconSize: 16 },
  ];
  const SOURCES: { id: Source; label: string }[] = [
    { id: "photos", label: fr.photos },
    { id: "library", label: fr.libraryTitle },
    { id: "mine", label: fr.myImages },
  ];
  const MIME: Record<Source, string> = { photos: ASSET_MIME, library: CLASS_PHOTO_MIME, mine: MY_IMAGE_MIME };
  const today = new Date();

  let source = $state<Source>(editor.imagesTab === "mine" ? "mine" : "photos");
  let filters = $state<Record<Source, string>>({
    photos: startFilter(readPref<PhotoFilter | null>("photoFilter", null), editor.photos.length).kind,
    library: "year|all",
    mine: "all",
  });
  let selected = $state<string[]>([]);
  let last = $state<string | null>(null);
  let thumbSize = $state(readPref("thumbSize", 96));
  let busy = $state(false);

  myImages.load();
  // Un glisser annulé avant l'image suivante ne doit pas laisser le tiroir réduit à la prochaine ouverture.
  ondrag(false);
  let held = false;
  // La photothèque de la classe courante, chargée si besoin (comme la page Photothèque) ; déjà chargée, un
  // parcours montre les photos ajoutées depuis dans les dossiers suivis.
  $effect(() => {
    const path = classes.current?.path;
    if (source !== "library" || !path) return;
    if (path !== classLibrary.classPath) classLibrary.open(path).catch((e) => (editor.notice = String(e)));
    else classLibrary.refresh();
  });

  const students = $derived(classes.current?.class.students ?? []);
  const unusedIds = $derived(unusedPhotoIds());
  const view = $derived(
    source === "photos"
      ? photosView(editor.photos, editor.doc, editor.page, filters.photos)
      : source === "library"
        ? libraryView(classLibrary.photos, students, filters.library, today, !!classes.current)
        : mineView(myImages.images, filters.mine),
  );
  const order = $derived(view.groups.flatMap((g) => g.items.map((i) => i.key)));
  // Le retrait des non placées suit le filtre : la vue d'un dossier ne retire que ses photos.
  const shownKeys = $derived(view.groups.flatMap((g) => g.items.map((it) => it.key)));
  const removeCount = $derived(source === "photos" ? shownKeys.filter((k) => unusedIds.has(k)).length : 0);
  const removeOnly = $derived(filters.photos.startsWith("folder:") ? new Set(shownKeys) : undefined);
  // Comme la page Photothèque : les premières vignettes seulement, puis davantage quand le bas approche. Le
  // choix (Maj, le jour entier) porte toujours sur toutes les images, dessinées ou non.
  let drawnCount = $state(DRAWN_STEP);
  const drawnGroups = $derived(firstDays(view.groups.map((g) => ({ ...g, photos: g.items })), drawnCount));
  let more = $state<HTMLElement>();
  // Une autre source ou un autre filtre : la grille repart des premières images.
  $effect(() => {
    void [source, filters[source]];
    drawnCount = DRAWN_STEP;
  });
  $effect(() => {
    void drawnCount;
    if (more) return drawMoreNearEnd(more, () => (drawnCount += DRAWN_STEP));
  });
  // Les filtres de la colonne, par rubrique (« Enfants », « Dossiers »…).
  const sections = $derived.by(() => {
    const out: { heading: string | null; filters: WideFilter[] }[] = [];
    for (const f of view.filters) {
      const heading = f.heading ?? null;
      if (out.at(-1)?.heading === heading) out.at(-1)!.filters.push(f);
      else out.push({ heading, filters: [f] });
    }
    return out;
  });

  function setSource(s: Source) {
    source = s;
    selected = [];
    last = null;
  }

  function setFilter(id: string) {
    filters[source] = id;
    selected = [];
    last = null;
    if (source === "photos" && (id === "all" || id === "unused" || id === "page")) writePref("photoFilter", { kind: id });
  }

  function choose(key: string, ev: MouseEvent) {
    selected = pick(selected, order, key, { ctrl: ev.ctrlKey || ev.metaKey, shift: ev.shiftKey }, last);
    last = key;
  }

  function setSize(px: number) {
    thumbSize = px;
    writePref("thumbSize", px);
  }

  /** Les images choisies, prêtes à poser (la photothèque et la collection passent d'abord dans le document). */
  async function metasOf(keys: string[]): Promise<AssetMeta[]> {
    if (source === "photos") return keys.flatMap((k) => editor.photos.find((p) => p.meta.id === k)?.meta ?? []);
    if (source === "library") return classLibrary.useInDocument(keys);
    const out: AssetMeta[] = [];
    for (const name of keys) out.push(await myImages.use(name));
    return out;
  }

  async function add() {
    if (busy) return;
    busy = true;
    try {
      useImages(await metasOf(selected));
      selected = [];
      editor.closeImagesWide();
    } catch (e) {
      editor.notice = String(e);
    } finally {
      busy = false;
    }
  }

  function onkey(e: KeyboardEvent) {
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement;
    if (e.key === "Escape" && !typing) {
      e.stopPropagation();
      editor.closeImagesWide();
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="wide">
  <header>
    <div class="sources" role="tablist">
      {#each SOURCES as s (s.id)}
        <button role="tab" aria-selected={source === s.id} class:on={source === s.id} onclick={() => setSource(s.id)}>{s.label}</button>
      {/each}
    </div>
    <div class="sizes" role="group" aria-label={fr.thumbSize}>
      {#each SIZES as s (s.px)}
        <button class:on={thumbSize === s.px} title={s.label} aria-label={s.label} onclick={() => setSize(s.px)}><Icon name="border" size={s.iconSize} /></button>
      {/each}
    </div>
  </header>

  <div class="main">
    <nav class="filters" aria-label={fr.images}>
      <!-- En haut de la colonne : visibles même avec beaucoup d'enfants ou de dossiers à faire défiler. -->
      {#if source === "library" && classes.current}
        <button class="link top" onclick={() => (editor.showPhototheque = true)}>{fr.librarySortLink}</button>
      {/if}
      <!-- Comme dans le tiroir : dès que la vue montre une photo non placée, et pas pendant un import (les
           photos encore en route reviendraient après le retrait). -->
      {#if source === "photos" && removeCount > 0 && !editor.importProgress}
        <button class="link top" onclick={async () => (await askRemoveUnusedPhotos(removeOnly)) && (selected = [])}>{fr.photosRemoveUnused(removeCount)}</button>
      {/if}
      {#each sections as sec, i (i)}
        {#if sec.heading}<h4>{sec.heading}</h4>{/if}
        {#each sec.filters as f (f.id)}
          <button class:on={f.on} onclick={() => setFilter(f.id)}>{f.label}</button>
        {/each}
      {/each}
    </nav>

    <section class="groups">
      {#if view.empty}<p class="hint">{view.empty}</p>{/if}
      {#each drawnGroups as g (g.title)}
        {@const keys = g.items.map((i) => i.key)}
        <button class="group" title={fr.imagesChooseDay} onclick={() => (selected = toggleAll(selected, keys))}>{g.title} <span>· {g.items.length}</span></button>
        <div class="thumbs" style:--thumb="{thumbSize}px">
          {#each g.drawn as it (it.key)}
            <div class="cell">
              <button
                class="thumb"
                class:on={selected.includes(it.key)}
                class:contain={source === "mine"}
                aria-pressed={selected.includes(it.key)}
                title={it.title || undefined}
                draggable="true"
                onclick={(e) => choose(it.key, e)}
                ondragstart={(e) => {
                  e.dataTransfer?.setData(MIME[source], it.key);
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
                  // Le tiroir se replie à l'image suivante, sans retirer l'image tenue de la page.
                  held = true;
                  requestAnimationFrame(() => held && ondrag(true));
                }}
                ondragend={() => {
                  held = false;
                  ondrag(false);
                  editor.closeImagesWide();
                }}
              >
                <img src={it.url} alt="" loading="lazy" decoding="async" draggable="false" />
                {#if source === "mine"}<span class="name">{it.title}</span>{/if}
              </button>
              {#if it.badge}<span class="badge">{it.badge}</span>{/if}
              {#if it.warn}<span class="warn" title={it.warn}><Icon name="ban" size={12} /></span>{/if}
            </div>
          {/each}
        </div>
      {/each}
      {#if order.length > drawnCount}<div class="more" bind:this={more}></div>{/if}
    </section>
  </div>

  {#if selected.length}
    <footer>
      <span>{fr.imagesChosen(selected.length)}</span>
      <button class="link" onclick={() => (selected = [])}>{fr.imagesClear}</button>
      <button class="primary" disabled={busy} onclick={add}>{fr.imagesAdd(selected.length)}</button>
    </footer>
  {/if}
</div>

<style>
  .wide {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 100%;
  }
  header {
    position: sticky;
    top: -8px;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: -8px -12px 0;
    padding: 8px 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .sources {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .sources button {
    border: 0;
    border-radius: 0;
    padding: 6px 14px;
    background: var(--surface);
  }
  .sources button + button {
    border-left: 1px solid var(--line);
  }
  .sizes {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  .sizes button {
    padding: 0 7px;
  }
  .on {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }
  .main {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 16px;
    align-items: start;
    flex: 1;
  }
  /* La colonne défile seule (beaucoup d'enfants ou de dossiers) ; ses liens restent en haut. */
  .filters {
    position: sticky;
    top: 48px;
    max-height: calc(100vh - 250px);
    overflow-y: auto;
    overscroll-behavior: contain;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .filters button {
    text-align: left;
    border-color: transparent;
    background: transparent;
    padding: 3px 8px;
    font-size: 13px;
  }
  .filters .on {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  h4 {
    margin: 10px 0 2px;
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
  }
  .link {
    margin-top: 10px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-decoration: underline;
    text-align: left;
  }
  .link.top {
    position: sticky;
    top: 0;
    z-index: 1;
    margin: 0 0 6px;
    padding: 2px 0 6px;
    background: var(--panel);
  }
  .groups {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .group {
    align-self: flex-start;
    border: 0;
    background: none;
    padding: 6px 0 2px;
    font-weight: 600;
  }
  .group span {
    color: var(--muted);
    font-weight: 400;
  }
  .group:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--thumb), 1fr));
    gap: 8px;
  }
  .cell {
    position: relative;
  }
  .more {
    height: 1px;
  }
  .thumb {
    width: 100%;
    display: flex;
    flex-direction: column;
    padding: 0;
    border: 3px solid transparent;
    border-radius: 6px;
    overflow: hidden;
    aspect-ratio: 1;
    background: var(--bg);
    cursor: grab;
    content-visibility: auto;
    contain-intrinsic-size: var(--thumb) var(--thumb);
  }
  .thumb.on {
    border-color: var(--accent);
    font-weight: 400;
  }
  .thumb img {
    width: 100%;
    flex: 1;
    min-height: 0;
    object-fit: cover;
    display: block;
  }
  /* Les dessins de la collection restent entiers, sur du blanc, avec leur nom. */
  .thumb.contain img {
    object-fit: contain;
    background: white;
  }
  .name {
    font-size: 11px;
    padding: 2px 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    position: absolute;
    right: 6px;
    bottom: 6px;
    padding: 0 6px;
    border-radius: 9px;
    background: var(--ok);
    color: var(--on-accent);
    font-size: 11px;
    font-weight: 600;
    line-height: 18px;
    pointer-events: none;
  }
  .warn {
    position: absolute;
    top: 6px;
    right: 6px;
    padding: 0 4px;
    border-radius: 9px;
    background: var(--danger-soft);
    color: var(--danger);
    line-height: 18px;
  }
  footer {
    position: sticky;
    bottom: -12px;
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0 -12px -12px;
    padding: 10px 12px;
    background: var(--panel);
    border-top: 1px solid var(--line);
  }
  footer .link {
    margin: 0;
  }
  .primary {
    margin-left: auto;
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
  .hint {
    color: var(--muted);
    margin: 0;
  }
</style>

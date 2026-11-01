<script lang="ts">
  // Photothèque de la classe : toutes les photos de l'année, à trier (garder, supprimer), à étiqueter
  // (élèves présents, activités), à retrouver (par enfant, par activité, par période) et à poser dans le
  // document. Un bandeau signale les enfants peu photographiés et une pastille les photos d'un enfant
  // sans autorisation (à flouter avant de servir).
  import { open } from "@tauri-apps/plugin-dialog";
  import { classPhotoUrl, listImportableIn } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { classes } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { byDay, countsByStudent, displayNames, filterLibrary, presence, studentsBehind, tagsOf, withoutConsent, type LibraryFilter, type Period } from "../../lib/phototheque";
  import { classLibrary } from "../../lib/phototheque.svelte";
  import PhotoViewer from "./PhotoViewer.svelte";

  const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "JPG", "JPEG", "PNG", "WEBP"];
  const PERIODS: Period[] = ["month", "trimester", "year", "all"];
  /** Enfants nommés dans le bandeau d'équité avant « et N autres ». */
  const BEHIND_SHOWN = 6;

  let filter = $state<LibraryFilter>({ period: "year" });
  let selected = $state<string[]>([]);
  let lastClicked = $state<string | null>(null);
  let newTag = $state("");
  let error = $state<string | null>(null);
  // Photo vue en grand : la liste parcourue et la position de départ.
  let viewing = $state<{ ids: string[]; start: number } | null>(null);
  // Mode « un enfant, puis ses photos » : chaque clic sur une photo l'y ajoute ou l'en retire.
  let marking = $state<string | null>(null);
  let allBehind = $state(false);

  const today = new Date();
  // Par ordre alphabétique, pour retrouver vite un prénom parmi 40.
  const students = $derived([...(classes.current?.class.students ?? [])].sort((a, b) => a.first_name.localeCompare(b.first_name, "fr", { sensitivity: "base" }) || a.last_name.localeCompare(b.last_name, "fr", { sensitivity: "base" })));
  // Deux « Anna » se distinguent par l'initiale de leur nom, partout sur la page.
  const names = $derived(displayNames(students));
  const nameOf = (id: string) => names.get(id) ?? "?";
  const shown = $derived(filterLibrary(classLibrary.photos, filter, today));
  const days = $derived(byDay(shown));
  const counts = $derived(countsByStudent(classLibrary.photos, students, filter.period, today));
  const behind = $derived(studentsBehind(counts));
  const tags = $derived(tagsOf(classLibrary.photos));
  const toSort = $derived(filterLibrary(classLibrary.photos, { period: "all", untagged: true }, today).length);
  const chosen = $derived(classLibrary.photos.filter((p) => selected.includes(p.id)));

  // Ouverte pour la classe courante ; rechargée si l'on change de classe.
  $effect(() => {
    const path = classes.current?.path;
    if (path && path !== classLibrary.classPath) {
      selected = [];
      classLibrary.open(path).catch((e) => (error = String(e)));
    }
  });

  function close() {
    editor.showPhototheque = false;
  }

  async function addFiles() {
    const picked = await open({ multiple: true, filters: [{ name: "Images", extensions: EXTENSIONS }] });
    if (picked) await addPaths(Array.isArray(picked) ? picked : [picked]);
  }
  async function addFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === "string") await addPaths(await listImportableIn(dir));
  }
  /** Un lot arrive dans « À trier », photo après photo ; à la fin il est choisi (si l'on n'a rien choisi
   *  entre-temps) : on indique tout de suite qui est dessus. */
  async function addPaths(paths: string[]) {
    error = null;
    if (!paths.length) return;
    filter = { period: "all", untagged: true };
    try {
      const added = await classLibrary.add(paths);
      if (!selected.length && !marking) selected = added;
    } catch (e) {
      error = String(e);
    }
  }

  /** Clic : une photo ; Ctrl : en ajouter ou en retirer une ; Maj : toutes celles entre deux. En mode
   *  « marquer un enfant », le clic l'ajoute à la photo ou l'en retire. */
  function pick(id: string, ev: MouseEvent) {
    if (marking) {
      const p = classLibrary.photos.find((x) => x.id === id);
      const on = !!p?.students.includes(marking);
      void run(() => classLibrary.tag([id], on ? { remove_students: [marking!] } : { add_students: [marking!] }));
      return;
    }
    if (ev.shiftKey && lastClicked) {
      const ids = shown.map((p) => p.id);
      const [a, b] = [ids.indexOf(lastClicked), ids.indexOf(id)].sort((x, y) => x - y);
      selected = [...new Set([...selected, ...ids.slice(a, b + 1)])];
    } else if (ev.ctrlKey || ev.metaKey) {
      selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id];
    } else {
      selected = selected.length === 1 && selected[0] === id ? [] : [id];
    }
    lastClicked = id;
  }

  /** Tous les jours affichés, ou un seul : ajoutés au choix, retirés s'ils y sont déjà tous. */
  function pickAll(ids: string[]) {
    selected = ids.every((id) => selected.includes(id)) ? selected.filter((id) => !ids.includes(id)) : [...new Set([...selected, ...ids])];
  }

  function startMarking(id: string) {
    marking = id || null;
    selected = [];
  }

  function onkey(e: KeyboardEvent) {
    if (viewing) return;
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement;
    if (e.key === "Escape") {
      if (marking) marking = null;
      else if (selected.length) selected = [];
      else close();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a" && !typing && !marking) {
      e.preventDefault();
      selected = shown.map((p) => p.id);
    }
  }

  /** Un enfant sur toutes les photos choisies : on l'en retire ; sinon on l'y ajoute. */
  async function toggleStudent(id: string) {
    const all = presence(chosen, id) === "all";
    await run(() => classLibrary.tag(selected, all ? { remove_students: [id] } : { add_students: [id] }));
  }
  async function toggleTag(t: string) {
    const all = chosen.every((p) => p.tags.includes(t));
    await run(() => classLibrary.tag(selected, all ? { remove_tags: [t] } : { add_tags: [t] }));
  }
  async function addTag() {
    const t = newTag.trim();
    if (!t) return;
    await run(() => classLibrary.tag(selected, { add_tags: [t] }));
    newTag = "";
  }
  async function removeChosen() {
    if (!(await ask(fr.libraryDeleteConfirm(selected.length), fr.delete))) return;
    await run(() => classLibrary.remove(selected));
    selected = [];
  }
  async function useChosen() {
    await run(async () => {
      const n = await classLibrary.useInDocument(selected);
      editor.notice = fr.libraryUsed(n);
      editor.openImages("photos");
      close();
    });
  }
  async function run(action: () => Promise<unknown>) {
    error = null;
    try {
      await action();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="library-page">
  <div class="top">
    <button class="back" onclick={close}>← {fr.classPageBack}</button>
    <h1>📷 {fr.libraryTitle}{#if classes.current}{` · ${classes.current.class.name}`}{/if}</h1>
    {#if classes.current}
      <button onclick={addFiles}>{fr.addPhotosButton}</button>
      <button onclick={addFolder}>{fr.addFolderButton}</button>
    {/if}
  </div>

  {#if !classes.current}
    <p class="hint">{fr.libraryNoClass}</p>
  {:else}
    {#if classLibrary.progress}
      {@const p = classLibrary.progress}
      <div class="progress" role="status"><span>{fr.importProgress(p.done, p.total)}</span><progress max={p.total} value={p.done}></progress></div>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if behind.length}
      <p class="equity" role="note">
        {fr.libraryBehind(filter.period)}
        {#each allBehind ? behind : behind.slice(0, BEHIND_SHOWN) as id (id)}<button class="kid" onclick={() => (filter = { ...filter, student: id, tag: null, untagged: false })}>{nameOf(id)} ({counts.get(id)})</button>{/each}
        {#if behind.length > BEHIND_SHOWN}<button class="link" onclick={() => (allBehind = !allBehind)}>{allBehind ? fr.libraryBehindLess : fr.libraryBehindMore(behind.length - BEHIND_SHOWN)}</button>{/if}
      </p>
    {/if}

    <div class="layout">
      <aside class="filters">
        <label class="period">
          {fr.libraryPeriod}
          <select bind:value={filter.period}>
            {#each PERIODS as p (p)}<option value={p}>{fr.libraryPeriods[p]}</option>{/each}
          </select>
        </label>
        <button class:on={!filter.student && !filter.tag && !filter.untagged} onclick={() => (filter = { period: filter.period })}>{fr.libraryAll} ({classLibrary.photos.length})</button>
        <button class:on={!!filter.untagged} onclick={() => (filter = { period: "all", untagged: true })}>{fr.libraryToSort} ({toSort})</button>
        <h3>{fr.libraryChildren}</h3>
        {#each students as s (s.id)}
          <button class="child" class:on={filter.student === s.id} onclick={() => (filter = { period: filter.period, student: s.id })}>
            <span>{nameOf(s.id)}{#if !s.photo_allowed}<span class="no-consent" title={fr.libraryNoConsentHint}> 🚫</span>{/if}</span>
            <span class="n">{counts.get(s.id) ?? 0}</span>
          </button>
        {/each}
        {#if tags.length}
          <h3>{fr.libraryActivities}</h3>
          {#each tags as t (t)}<button class:on={filter.tag === t} onclick={() => (filter = { period: filter.period, tag: t })}>{t}</button>{/each}
        {/if}
      </aside>

      <section class="photos" aria-label={fr.libraryTitle}>
        {#if classLibrary.photos.length}
          <div class="toolbar">
            <label>
              {fr.libraryMark}
              <select value={marking ?? ""} onchange={(e) => startMarking(e.currentTarget.value)}>
                <option value="">{fr.libraryMarkNone}</option>
                {#each students as s (s.id)}<option value={s.id}>{nameOf(s.id)}</option>{/each}
              </select>
            </label>
            {#if !marking && shown.length}<button onclick={() => pickAll(shown.map((p) => p.id))} title="Ctrl+A">{fr.librarySelectAll}</button>{/if}
            <span class="hint">{marking ? fr.libraryMarkHint(nameOf(marking)) : fr.libraryZoomHint}</span>
            {#if marking}<button onclick={() => (marking = null)}>{fr.libraryMarkDone}</button>{/if}
          </div>
        {/if}
        {#if shown.length === 0}
          <p class="hint">{classLibrary.photos.length ? fr.libraryNoneHere : fr.libraryEmpty}</p>
        {/if}
        <div class="days">
        {#each days as d (d.day)}
          <div class="day">
            <button class="day-head" disabled={!!marking} title={fr.librarySelectDay} onclick={() => pickAll(d.photos.map((p) => p.id))}>
              {fr.libraryDayShort(d.day)} <span class="n">· {d.photos.length}</span>
            </button>
            <div class="grid">
            {#each d.photos as p (p.id)}
              {@const missing = withoutConsent(p, students)}
              {@const on = marking ? p.students.includes(marking) : selected.includes(p.id)}
              <button
                class="thumb"
                class:on
                class:marking
                aria-pressed={on}
                onclick={(e) => pick(p.id, e)}
                ondblclick={() => (viewing = { ids: shown.map((x) => x.id), start: shown.indexOf(p) })}
                title={p.students.map(nameOf).join(", ")}
              >
                <img src={classPhotoUrl(p.id)} alt="" loading="lazy" draggable="false" />
                {#if marking && on}<span class="badge mark">✓ {nameOf(marking)}</span>{:else if p.students.length}<span class="badge">🧒 {p.students.length}</span>{/if}
                {#if missing.length}<span class="badge warn" title={fr.libraryNoConsent(missing.map((s) => nameOf(s.id)))}>🚫 {missing.length}</span>{/if}
              </button>
            {/each}
            </div>
          </div>
        {/each}
        </div>
      </section>

      {#if chosen.length && !marking}
        <aside class="panel">
          <h3>{fr.libraryChosen(chosen.length)}</h3>
          <p class="hint">{fr.libraryWhoHint}</p>
          <div class="kids">
            {#each students as s (s.id)}
              {@const here = presence(chosen, s.id)}
              <button class="kid {here}" aria-pressed={here === "all"} onclick={() => toggleStudent(s.id)}>{nameOf(s.id)}</button>
            {/each}
          </div>
          <h3>{fr.libraryActivities}</h3>
          <div class="kids">
            {#each tags as t (t)}
              <button class="kid {chosen.every((p) => p.tags.includes(t)) ? 'all' : chosen.some((p) => p.tags.includes(t)) ? 'some' : 'none'}" onclick={() => toggleTag(t)}>{t}</button>
            {/each}
          </div>
          <div class="new-tag">
            <input placeholder={fr.libraryNewTag} bind:value={newTag} onkeydown={(e) => e.key === "Enter" && addTag()} />
            <button disabled={!newTag.trim()} onclick={addTag}>＋</button>
          </div>
          {#if chosen.some((p) => withoutConsent(p, students).length)}<p class="warn-text">{fr.libraryBlurFirst}</p>{/if}
          <div class="actions">
            <button class="primary" onclick={useChosen}>{fr.libraryUse(chosen.length)}</button>
            <button class="danger" onclick={removeChosen}>🗑 {fr.delete}</button>
          </div>
        </aside>
      {/if}
    </div>
    <p class="hint">{fr.libraryPrivacy}</p>
  {/if}
</div>

{#if viewing}
  <PhotoViewer ids={viewing.ids} start={viewing.start} {students} {names} onclose={() => (viewing = null)} onerror={(e) => (error = String(e))} />
{/if}

<style>
  .library-page {
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
    margin-bottom: 14px;
  }
  h1 {
    font: 600 24px var(--ui-font);
    margin: 0;
    flex: 1;
  }
  h3 {
    margin: 14px 0 6px;
    font-size: 13px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .back {
    border-color: transparent;
    background: transparent;
  }
  .layout {
    display: grid;
    grid-template-columns: 200px 1fr auto;
    gap: 16px;
    align-items: start;
  }
  .filters {
    display: flex;
    flex-direction: column;
    gap: 4px;
    position: sticky;
    top: 0;
  }
  .filters > button,
  .child {
    text-align: left;
    border-color: transparent;
    background: transparent;
  }
  .child {
    display: flex;
    justify-content: space-between;
  }
  .filters .on {
    background: #e8f1fb;
    border-color: var(--accent);
  }
  .n {
    color: var(--muted);
  }
  .period {
    display: flex;
    flex-direction: column;
    font-size: 12px;
    gap: 2px;
    margin-bottom: 8px;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 4px;
  }
  .toolbar label {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  /* Les jours s'enchaînent côte à côte : un jour d'une photo ne prend pas toute une ligne. */
  .days {
    display: flex;
    flex-wrap: wrap;
    gap: 14px 18px;
    align-items: flex-start;
  }
  .day {
    max-width: 100%;
  }
  .day-head {
    border: none;
    background: none;
    padding: 0 0 4px;
    font-weight: 600;
    font-size: 13px;
  }
  .day-head:not(:disabled):hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .link {
    border: none;
    background: none;
    color: var(--accent);
    padding: 0;
    text-decoration: underline;
    cursor: pointer;
  }
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .thumb {
    position: relative;
    width: 150px;
    padding: 0;
    aspect-ratio: 4 / 3;
    overflow: hidden;
    border: 3px solid transparent;
    border-radius: 8px;
    background: #ddd;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .thumb.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px #fff inset;
  }
  .badge {
    position: absolute;
    bottom: 4px;
    left: 4px;
    background: rgba(255, 255, 255, 0.9);
    border-radius: 10px;
    padding: 0 6px;
    font-size: 12px;
  }
  .thumb.marking.on {
    border-color: #2e8b57;
  }
  .badge.mark {
    background: #2e8b57;
    color: white;
  }
  .badge.warn {
    left: auto;
    right: 4px;
    background: #fdecea;
    color: #b03a2e;
  }
  .panel {
    width: 260px;
    position: sticky;
    top: 0;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 12px 14px;
  }
  .kids {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .kid {
    border-radius: 14px;
    padding: 2px 10px;
    font-size: 13px;
  }
  .kid.all {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .kid.some {
    background: #e8f1fb;
    border-color: var(--accent);
    border-style: dashed;
  }
  .new-tag {
    display: flex;
    gap: 4px;
    margin-top: 6px;
  }
  .new-tag input {
    flex: 1;
    min-width: 0;
  }
  .actions {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 14px;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
  .danger {
    color: #b03a2e;
  }
  .equity {
    background: #fff6e6;
    border: 1px solid #f0c98a;
    border-radius: 10px;
    padding: 8px 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .warn-text {
    color: #b03a2e;
    font-size: 13px;
  }
  .progress {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 10px;
  }
  .hint {
    color: var(--muted);
    font-size: 13px;
  }
  .error {
    color: #b03a2e;
  }
</style>

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
  import { countsByStudent, filterLibrary, presence, studentsBehind, tagsOf, withoutConsent, type LibraryFilter, type Period } from "../../lib/phototheque";
  import { classLibrary } from "../../lib/phototheque.svelte";

  const EXTENSIONS = ["jpg", "jpeg", "png", "webp", "JPG", "JPEG", "PNG", "WEBP"];
  const PERIODS: Period[] = ["month", "trimester", "year", "all"];

  let filter = $state<LibraryFilter>({ period: "year" });
  let selected = $state<string[]>([]);
  let lastClicked = $state<string | null>(null);
  let newTag = $state("");
  let error = $state<string | null>(null);

  const today = new Date();
  const students = $derived(classes.current?.class.students ?? []);
  const nameOf = (id: string) => students.find((s) => s.id === id)?.first_name ?? "?";
  const shown = $derived(filterLibrary(classLibrary.photos, filter, today));
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
  /** Un lot ajouté arrive choisi, dans « À trier » : on indique tout de suite qui est dessus. */
  async function addPaths(paths: string[]) {
    error = null;
    try {
      const added = await classLibrary.add(paths);
      filter = { period: "all", untagged: true };
      selected = added;
    } catch (e) {
      error = String(e);
    }
  }

  /** Clic : une photo ; Ctrl : en ajouter ou en retirer une ; Maj : toutes celles entre deux. */
  function pick(id: string, ev: MouseEvent) {
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
      editor.drawer = "photos";
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

<svelte:window onkeydown={(e) => e.key === "Escape" && (selected.length ? (selected = []) : close())} />

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
        {#each behind as id (id)}<button class="kid" onclick={() => (filter = { ...filter, student: id, tag: null, untagged: false })}>{nameOf(id)} ({counts.get(id)})</button>{/each}
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
            <span>{s.first_name}{#if !s.photo_allowed}<span class="no-consent" title={fr.libraryNoConsentHint}> 🚫</span>{/if}</span>
            <span class="n">{counts.get(s.id) ?? 0}</span>
          </button>
        {/each}
        {#if tags.length}
          <h3>{fr.libraryActivities}</h3>
          {#each tags as t (t)}<button class:on={filter.tag === t} onclick={() => (filter = { period: filter.period, tag: t })}>{t}</button>{/each}
        {/if}
      </aside>

      <section class="grid" aria-label={fr.libraryTitle}>
        {#if shown.length === 0}
          <p class="hint">{classLibrary.photos.length ? fr.libraryNoneHere : fr.libraryEmpty}</p>
        {/if}
        {#each shown as p (p.id)}
          {@const missing = withoutConsent(p, students)}
          <button class="thumb" class:on={selected.includes(p.id)} aria-pressed={selected.includes(p.id)} onclick={(e) => pick(p.id, e)} title={p.students.map(nameOf).join(", ")}>
            <img src={classPhotoUrl(p.id)} alt="" loading="lazy" draggable="false" />
            {#if missing.length}<span class="badge warn" title={fr.libraryNoConsent(missing.map((s) => s.first_name))}>🚫 {missing.length}</span>{/if}
            {#if p.students.length}<span class="badge">🧒 {p.students.length}</span>{/if}
          </button>
        {/each}
      </section>

      {#if chosen.length}
        <aside class="panel">
          <h3>{fr.libraryChosen(chosen.length)}</h3>
          <p class="hint">{fr.libraryWhoHint}</p>
          <div class="kids">
            {#each students as s (s.id)}
              {@const here = presence(chosen, s.id)}
              <button class="kid {here}" aria-pressed={here === "all"} onclick={() => toggleStudent(s.id)}>{s.first_name}</button>
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
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px;
    align-content: start;
  }
  .thumb {
    position: relative;
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

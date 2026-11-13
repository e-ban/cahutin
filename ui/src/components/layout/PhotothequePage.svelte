<script lang="ts">
  import GuideLink from "../GuideLink.svelte";
  import Icon from "../Icon.svelte";
  // Photothèque de la classe : les photos des dossiers suivis (rien n'est copié), à trier (garder, retirer), à
  // étiqueter (élèves présents, activités), à retrouver (par enfant, par activité, par dossier, par période) et
  // à poser dans le document. Un bandeau signale les enfants peu photographiés et une pastille les photos d'un
  // enfant sans autorisation (à flouter avant de servir). Un autre bandeau propose de retrouver les originaux
  // des photos copiées par une version précédente.
  import { open } from "@tauri-apps/plugin-dialog";
  import { classPhotoUrl } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { classes } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import {
    baseName,
    byDay,
    countsByStudent,
    dayBox,
    displayNames,
    filterLibrary,
    firstDays,
    folderCounts,
    foldersWithOldCopies,
    oldCopies,
    photoVersion,
    presence,
    studentsBehind,
    tagsOf,
    withoutConsent,
    type LibraryFilter,
    type Period,
  } from "../../lib/phototheque";
  import { classLibrary } from "../../lib/phototheque.svelte";
  import PhotoViewer from "./PhotoViewer.svelte";

  const PERIODS: Period[] = ["month", "trimester", "year", "all"];
  /** Enfants nommés dans le bandeau d'équité avant « et N autres ». */
  const BEHIND_SHOWN = 6;
  /** Vignettes dessinées d'abord, puis à chaque fois que le bas de la grille approche. */
  const DRAWN_STEP = 300;

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
  // « Plus tard » : le bandeau des anciennes copies ne revient qu'à la prochaine ouverture de la page.
  let migrationLater = $state(false);
  // Recherche des originaux en cours (le bouton attend).
  let searching = $state(false);

  const today = new Date();
  // Par ordre alphabétique, pour retrouver vite un prénom parmi 40.
  const students = $derived([...(classes.current?.class.students ?? [])].sort((a, b) => a.first_name.localeCompare(b.first_name, "fr", { sensitivity: "base" }) || a.last_name.localeCompare(b.last_name, "fr", { sensitivity: "base" })));
  // Deux « Anna » se distinguent par l'initiale de leur nom, partout sur la page.
  const names = $derived(displayNames(students));
  const nameOf = (id: string) => names.get(id) ?? "?";
  // Seules les photos ni retirées ni absentes sont montrées et comptées.
  const photos = $derived(classLibrary.visible);
  const shown = $derived(filterLibrary(photos, filter, today));
  const days = $derived(byDay(shown));
  let drawnCount = $state(DRAWN_STEP);
  const drawnDays = $derived(firstDays(days, drawnCount));
  // Repère sous la dernière vignette dessinée : quand il approche de l'écran, on en dessine davantage.
  let more = $state<HTMLElement>();
  let daysWidth = $state(0);
  const counts = $derived(countsByStudent(photos, students, filter.period, today));
  const behind = $derived(studentsBehind(counts));
  const tags = $derived(tagsOf(photos));
  const folders = $derived(folderCounts(filterLibrary(photos, { period: filter.period }, today)));
  const toSort = $derived(filterLibrary(photos, { period: "all", untagged: true }, today).length);
  // Une photo choisie puis retirée, ou dont le dossier n'est plus suivi, quitte le choix d'elle-même.
  const chosen = $derived(photos.filter((p) => selected.includes(p.id)));
  const chosenIds = () => chosen.map((p) => p.id);
  const copies = $derived(oldCopies(classLibrary.photos).length);
  // Le dossier des anciennes copies reste suivi tant que la migration n'est pas faite (sinon leurs enfants et
  // activités seraient perdus) : pas de « Ne plus suivre » pour lui d'ici là.
  const keep = $derived(
    copies
      ? foldersWithOldCopies(
          classLibrary.photos,
          classLibrary.folders.map((f) => f.path),
        )
      : [],
  );
  const busy = $derived(searching || !!classLibrary.scanning);

  // Ouverte pour la classe courante ; rechargée si l'on change de classe. Déjà chargée, la page rouverte
  // parcourt les dossiers suivis : les photos ajoutées depuis (celles de la semaine) apparaissent.
  $effect(() => {
    const path = classes.current?.path;
    if (path && path !== classLibrary.classPath) {
      selected = [];
      classLibrary.open(path).catch((e) => (error = String(e)));
    } else if (path) classLibrary.refresh();
  });

  // Un autre filtre : la grille repart des premières photos.
  $effect(() => {
    void [filter.period, filter.student, filter.tag, filter.untagged, filter.folder];
    drawnCount = DRAWN_STEP;
  });
  // Le repère à moins de 1500 px du bas de l'écran, ou déjà passé au-dessus (un saut tout en bas ; la colonne
  // des filtres, plus longue que la grille, fait descendre la page plus bas que lui) : on en dessine davantage.
  // Vu à chaque défilement, et une image après chaque ajout, au cas où il serait encore proche.
  $effect(() => {
    void drawnCount;
    const mark = more;
    const page = mark?.closest(".library-page");
    if (!mark || !page) return;
    const check = () => {
      if (mark.getBoundingClientRect().top < page.getBoundingClientRect().bottom + 1500) drawnCount += DRAWN_STEP;
    };
    page.addEventListener("scroll", check, { passive: true });
    const frame = requestAnimationFrame(check);
    return () => {
      page.removeEventListener("scroll", check);
      cancelAnimationFrame(frame);
    };
  });

  function close() {
    editor.showPhototheque = false;
  }

  /** Un dossier suivi : ses photos arrivent au fil du parcours, quelle que soit leur date. */
  async function follow() {
    const dir = await open({ directory: true });
    if (typeof dir !== "string") return;
    // Des photos de l'an dernier doivent se voir aussi : toutes les périodes, pendant que la grille se remplit.
    filter = { period: "all" };
    await run(() => classLibrary.follow(dir));
  }
  async function unfollow(folder: string) {
    await run(() => classLibrary.unfollow(folder));
  }

  /** Les originaux des anciennes copies : on dit combien sont retrouvés, puis on demande avant la corbeille. */
  async function findOriginals() {
    error = null;
    searching = true;
    try {
      const report = await classLibrary.findOriginals();
      const n = report.matches.length;
      if (n === 0) {
        editor.notice = fr.libraryOriginalsNone;
        return;
      }
      if (!(await ask(fr.libraryOriginalsFound(n, report.unmatched.length), fr.yes, fr.no))) return;
      const left = await classLibrary.applyOriginals(report.matches);
      editor.notice = left ? fr.libraryOriginalsNotTrashed(n - left, left) : fr.libraryOriginalsDone(n);
    } catch (e) {
      error = String(e);
    } finally {
      searching = false;
    }
  }

  /** « Non merci » : la question ne revient plus ; le lien « Retrouver les originaux » reste dans la colonne. */
  async function declineMigration() {
    await run(async () => {
      await classLibrary.settleMigration();
      editor.notice = fr.libraryMigrationDeclined;
    });
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
    await run(() => classLibrary.tag(chosenIds(), all ? { remove_students: [id] } : { add_students: [id] }));
  }
  async function toggleTag(t: string) {
    const all = chosen.every((p) => p.tags.includes(t));
    await run(() => classLibrary.tag(chosenIds(), all ? { remove_tags: [t] } : { add_tags: [t] }));
  }
  async function addTag() {
    const t = newTag.trim();
    if (!t) return;
    await run(() => classLibrary.tag(chosenIds(), { add_tags: [t] }));
    newTag = "";
  }
  /** « Retirer » : plus montrées ici, aucun fichier touché ; l'avis propose « Annuler ». */
  async function hideChosen() {
    const ids = chosenIds();
    await run(() => classLibrary.hide(ids));
    selected = [];
  }
  async function useChosen() {
    await run(async () => {
      const n = (await classLibrary.useInDocument(chosenIds())).length;
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
    <button class="back" onclick={close}><Icon name="back" /> {fr.classPageBack}</button>
    <h1><Icon name="images" size={24} /> {fr.libraryTitle}{#if classes.current}{` · ${classes.current.class.name}`}{/if}</h1>
    {#if classes.current}
      <button data-guide="library-add" onclick={follow}><Icon name="folder" /> {fr.addFolder}</button>
    {/if}
    <GuideLink id="phototheque" />
  </div>

  {#if !classes.current}
    <p class="hint">{fr.libraryNoClass}</p>
  {:else}
    {#if classLibrary.scanning}
      <div class="progress" role="status"><span>{fr.libraryScanning(classLibrary.scanning.found)}</span><progress></progress></div>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#each classLibrary.missingFolders as path (path)}
      <p class="note" role="note" title={path}><Icon name="warning" /> {fr.libraryFolderMissing(baseName(path))}</p>
    {/each}
    {#if copies && !classLibrary.migrationSettled && !migrationLater}
      <div class="note migration" role="note">
        <p>{fr.libraryMigration}</p>
        <div class="row">
          <button class="primary" disabled={busy} onclick={findOriginals}>{searching ? fr.librarySearching : fr.libraryFindOriginals}</button>
          <button onclick={() => (migrationLater = true)}>{fr.libraryLater}</button>
          <button disabled={busy} onclick={declineMigration}>{fr.libraryNoThanks}</button>
        </div>
      </div>
    {/if}
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
        <button class:on={!filter.student && !filter.tag && !filter.untagged && !filter.folder} onclick={() => (filter = { period: filter.period })}>{fr.libraryAll} ({photos.length})</button>
        <button class:on={!!filter.untagged} onclick={() => (filter = { period: "all", untagged: true })}>{fr.libraryToSort} ({toSort})</button>
        <h3>{fr.libraryChildren}</h3>
        {#each students as s (s.id)}
          <button class="child" class:on={filter.student === s.id} onclick={() => (filter = { period: filter.period, student: s.id })}>
            <span>{nameOf(s.id)}{#if !s.photo_allowed}<span class="no-consent" title={fr.libraryNoConsentHint}> <Icon name="ban" size={14} /></span>{/if}</span>
            <span class="n">{counts.get(s.id) ?? 0}</span>
          </button>
        {/each}
        {#if tags.length}
          <h3>{fr.libraryActivities}</h3>
          {#each tags as t (t)}<button class:on={filter.tag === t} onclick={() => (filter = { period: filter.period, tag: t })}>{t}</button>{/each}
        {/if}
        {#if folders.length > 1}
          <h3>{fr.imagesFolders}</h3>
          {#each folders as f (f.folder)}
            <button class="child" class:on={filter.folder === f.folder} onclick={() => (filter = { period: filter.period, folder: f.folder })}>
              <span>{f.folder}</span><span class="n">{f.count}</span>
            </button>
          {/each}
        {/if}
        {#if classLibrary.folders.length}
          <h3>{fr.libraryFollowed}</h3>
          {#each classLibrary.folders as f (f.path)}
            <div class="followed" title={f.path}>
              <span class:gone={f.missing}><Icon name={f.missing ? "warning" : "folder"} size={14} /> {baseName(f.path)}</span>
              {#if !keep.includes(f.path)}<button class="link" onclick={() => unfollow(f.path)}>{fr.libraryUnfollow}</button>{/if}
            </div>
          {/each}
        {/if}
        {#if copies}<button class="link" disabled={busy} onclick={findOriginals}>{searching ? fr.librarySearching : fr.libraryFindOriginals}</button>{/if}
      </aside>

      <section class="photos" aria-label={fr.libraryTitle}>
        {#if photos.length}
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
          <p class="hint">{photos.length ? fr.libraryNoneHere : classLibrary.scanning ? "" : fr.libraryEmpty}</p>
        {/if}
        <div class="days" bind:clientWidth={daysWidth}>
        {#each drawnDays as d (d.day)}
          {@const box = dayBox(d.drawn.length, daysWidth)}
          <div class="day" style:contain-intrinsic-size="auto {box.width}px auto {box.height}px">
            <button class="day-head" disabled={!!marking} title={fr.librarySelectDay} onclick={() => pickAll(d.photos.map((p) => p.id))}>
              {fr.libraryDayShort(d.day)} <span class="n">· {d.photos.length}</span>
            </button>
            <div class="grid">
            {#each d.drawn as p (p.id)}
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
                <img src={classPhotoUrl(p.id, true, photoVersion(p))} alt="" loading="lazy" draggable="false" />
                {#if marking && on}<span class="badge mark"><Icon name="check" size={12} /> {nameOf(marking)}</span>{:else if p.students.length}<span class="badge"><Icon name="users" size={12} /> {p.students.length}</span>{/if}
                {#if missing.length}<span class="badge warn" title={fr.libraryNoConsent(missing.map((s) => nameOf(s.id)))}><Icon name="ban" size={12} /> {missing.length}</span>{/if}
              </button>
            {/each}
            </div>
          </div>
        {/each}
        </div>
        {#if shown.length > drawnCount}<div class="more" bind:this={more}></div>{/if}
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
            <button disabled={!newTag.trim()} title={fr.libraryNewTag} aria-label={fr.libraryNewTag} onclick={addTag}><Icon name="add" /></button>
          </div>
          {#if chosen.some((p) => withoutConsent(p, students).length)}<p class="warn-text">{fr.libraryBlurFirst}</p>{/if}
          <div class="actions">
            <button class="primary" onclick={useChosen}>{fr.libraryUse(chosen.length)}</button>
            <button onclick={hideChosen} title={fr.libraryHideHint}><Icon name="eyeOff" /> {fr.libraryHide}</button>
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
    background: var(--accent-soft);
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
  /* Un jour loin de l'écran n'est ni mis en page ni dessiné : avec des milliers de vignettes dessinées, chaque
     ajout en bas de la grille remettait toute la page en forme (750 ms de gel à 10 000 photos, 70 ms ainsi). Sa
     place est réservée d'après son nombre de photos (`dayBox`). */
  .day {
    max-width: 100%;
    content-visibility: auto;
  }
  .more {
    height: 1px;
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
    background: var(--line);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .thumb.on {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--surface) inset;
  }
  .badge {
    position: absolute;
    bottom: 4px;
    left: 4px;
    background: color-mix(in srgb, var(--surface) 90%, transparent);
    border-radius: 10px;
    padding: 0 6px;
    font-size: 12px;
  }
  .thumb.marking.on {
    border-color: var(--ok);
  }
  .badge.mark {
    background: var(--ok);
    color: var(--on-accent);
  }
  .badge.warn {
    left: auto;
    right: 4px;
    background: var(--danger-soft);
    color: var(--danger);
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
    color: var(--on-accent);
  }
  .kid.some {
    background: var(--accent-soft);
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
    color: var(--on-accent);
    font-weight: 600;
  }
  .followed {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 6px;
    font-size: 13px;
    padding: 2px 0;
  }
  .followed span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gone {
    color: var(--muted);
  }
  .link:disabled {
    color: var(--muted);
    cursor: default;
  }
  .note {
    background: var(--warn-bg);
    border: 1px solid var(--warn-line);
    border-radius: 10px;
    padding: 8px 12px;
    margin: 0 0 10px;
  }
  .migration p {
    margin: 0 0 8px;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .equity {
    background: var(--warn-bg);
    border: 1px solid var(--warn-line);
    border-radius: 10px;
    padding: 8px 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }
  .warn-text {
    color: var(--danger);
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
    color: var(--danger);
  }
</style>

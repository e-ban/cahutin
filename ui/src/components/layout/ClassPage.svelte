<script lang="ts">
  // Page « Ma classe » : la classe sur toute la largeur, un élève par ligne (photo, prénom, nom,
  // autorisation, symbole, niveau, groupe). Ouverte depuis l'accueil ou par « Modifier la classe… » du
  // tiroir. Tout s'enregistre seul dans Documents/Cahutin/Classes.
  import { open } from "@tauri-apps/plugin-dialog";
  import { assetUrl, classImportPhoto, classParseList } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { classes, groups, schoolYear } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { modal } from "../../lib/modal.svelte";
  import { pictoUrl, pictos, searchPictos } from "../../lib/pictos.svelte";
  import type { Level, Student } from "../../lib/types";

  const LEVELS: Level[] = ["PS", "MS", "GS"];

  let pasted = $state("");
  let pasting = $state(false);
  let newFirst = $state("");
  let symbolFor = $state<string | null>(null);
  let symbolQuery = $state("");
  // Création d'une autre classe alors qu'une classe est ouverte.
  let creating = $state(false);
  let pageEl = $state<HTMLElement>();

  $effect(() => void classes.refresh());
  $effect(() => void pictos.load());
  $effect(() => {
    void symbolFor;
    symbolQuery = "";
  });
  // Fermeture de la fenêtre : le champ en cours de frappe est validé d'abord.
  $effect(() =>
    editor.onFlush(() => {
      const a = document.activeElement;
      if ((a instanceof HTMLInputElement || a instanceof HTMLSelectElement) && pageEl?.contains(a)) a.blur();
    }),
  );

  const cls = $derived(classes.current?.class);
  const showCreate = $derived(!cls || creating);
  const groupList = $derived(cls ? groups(cls.students) : []);
  const symbols = $derived(symbolQuery.trim() ? searchPictos(pictos.index, symbolQuery).slice(0, 40) : []);

  /** Tri sans accents ni capitales, comme le moteur. */
  const byName = (a: Student, b: Student) => a.first_name.localeCompare(b.first_name, "fr", { sensitivity: "base" }) || a.last_name.localeCompare(b.last_name, "fr", { sensitivity: "base" });

  function setStudent(id: string, patch: Partial<Student>) {
    classes.update((c) => ({ ...c, students: c.students.map((s) => (s.id === id ? { ...s, ...patch } : s)) }));
  }

  async function addPasted() {
    if (!pasted.trim()) return;
    try {
      const added = await classParseList(pasted);
      classes.update((c) => ({ ...c, students: [...c.students, ...added].sort(byName) }));
      pasted = "";
      pasting = false;
    } catch (e) {
      editor.notice = String(e);
    }
  }

  async function createFromPaste() {
    if (!pasted.trim()) return;
    await classes.createNamed(fr.classDefaultName, schoolYear(new Date()));
    creating = false;
    if (classes.current) await addPasted();
  }

  /** Un élève ajouté à la main : son prénom, puis Entrée. */
  async function addOne() {
    const first = newFirst.trim();
    if (!first) return;
    const [s] = await classParseList(first);
    if (s) classes.update((c) => ({ ...c, students: [...c.students, s].sort(byName) }));
    newFirst = "";
  }

  async function choosePhoto(s: Student) {
    const picked = await open({ filters: [{ name: fr.photos, extensions: ["jpg", "jpeg", "png", "webp"] }] });
    if (typeof picked !== "string") return;
    try {
      const meta = await classImportPhoto(picked);
      editor.registerAssets([meta]);
      // L'autorisation concerne l'enfant : elle ne change pas avec la photo.
      classes.update((c) => ({ ...c, photos: { ...c.photos, [meta.id]: meta }, students: c.students.map((x) => (x.id === s.id ? { ...x, photo: meta.id } : x)) }));
    } catch (e) {
      editor.notice = String(e);
    }
  }

  /** Photos de toute la classe d'un coup : chacune va à l'enfant dont elle porte le prénom. */
  async function addPhotos() {
    const picked = await open({ multiple: true, filters: [{ name: fr.photos, extensions: ["jpg", "jpeg", "png", "webp"] }] });
    const paths = Array.isArray(picked) ? picked : typeof picked === "string" ? [picked] : [];
    if (!paths.length) return;
    const r = await classes.addPhotos(paths, (n) => ask(fr.classConsentAll(n), fr.classConsentYes));
    editor.notice = fr.classPhotosAdded(r.added, r.missed);
  }

  async function remove(s: Student) {
    if (!(await ask(fr.classRemoveStudent(s.first_name), fr.delete))) return;
    classes.update((c) => ({ ...c, students: c.students.filter((x) => x.id !== s.id) }));
  }

  async function switchTo(value: string) {
    if (value === "new") creating = true;
    else if (value) await classes.open(value);
  }

  async function close() {
    await classes.flush();
    editor.showClassPage = false;
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && modal.message === null && (symbolFor ? (symbolFor = null) : close())} />

<div class="class-page" bind:this={pageEl}>
  <div class="top">
    <button class="back" onclick={close}>← {fr.classPageBack}</button>
    <h1>🧒 {fr.classPageTitle}</h1>
    {#if cls && classes.list.length}
      <select class="switch" value="" onchange={(e) => switchTo(e.currentTarget.value)} aria-label={fr.classSwitch}>
        <option value="">{fr.classSwitch}…</option>
        {#each classes.list.filter((c) => c.path !== classes.current?.path) as c (c.path)}
          <option value={c.path}>{c.name} · {c.year}</option>
        {/each}
        <option value="new">＋ {fr.classNew}</option>
      </select>
    {/if}
  </div>

  {#if showCreate}
    <section class="create">
      {#if !cls && classes.list.length}
        <h2>{fr.classChoose}</h2>
        <div class="classes">
          {#each classes.list as c (c.path)}
            <button onclick={() => classes.open(c.path)}><b>{c.name}</b><span>{c.year} · {fr.classCount(c.students)}</span></button>
          {/each}
        </div>
      {/if}
      <h2>{cls || classes.list.length ? fr.classNew : fr.classFirst}</h2>
      <textarea rows="10" bind:value={pasted} placeholder={fr.classPastePlaceholder}></textarea>
      <div class="row-buttons">
        <button class="primary" disabled={!pasted.trim()} onclick={createFromPaste}>＋ {fr.classCreate}</button>
        {#if creating}<button onclick={() => (creating = false)}>{fr.cancel}</button>{/if}
      </div>
      <p class="hint">{fr.classPrivacy}</p>
    </section>
  {:else if cls}
    <div class="head">
      <label>{fr.className} <input class="name" value={cls.name} onchange={(e) => e.currentTarget.value.trim() && classes.update((c) => ({ ...c, name: e.currentTarget.value.trim() }))} /></label>
      <label>{fr.classYearLabel} <input class="year" value={cls.year} onchange={(e) => e.currentTarget.value.trim() && classes.update((c) => ({ ...c, year: e.currentTarget.value.trim() }))} /></label>
      <span class="count">{fr.classCount(cls.students.length)}</span>
      <span class="spacer"></span>
      <button onclick={() => (pasting = !pasting)}>📋 {fr.classPaste}</button>
      <button disabled={!cls.students.length} title={fr.classPhotosHint} onclick={addPhotos}>📷 {fr.classPhotosAdd}</button>
    </div>
    {#if pasting}
      <div class="paste">
        <textarea rows="5" bind:value={pasted} placeholder={fr.classPastePlaceholder}></textarea>
        <div class="row-buttons">
          <button class="primary" disabled={!pasted.trim()} onclick={addPasted}>{fr.classPasteAdd}</button>
          <button onclick={() => (pasting = false)}>{fr.cancel}</button>
        </div>
      </div>
    {/if}

    <table>
      <thead>
        <tr>
          <th>{fr.classPhoto}</th>
          <th>{fr.classFirstName}</th>
          <th>{fr.classLastName}</th>
          <th title={fr.classConsent}>📷 {fr.classConsentShort}</th>
          <th>{fr.classSymbol}</th>
          <th>{fr.classLevel}</th>
          <th>{fr.classGroup}</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each cls.students as s (s.id)}
          {@const photo = s.photo ? cls.photos[s.photo] : undefined}
          <tr>
            <td>
              <button class="face" title={photo ? fr.classPhotoChange : fr.classPhotoChoose} onclick={() => choosePhoto(s)}>
                {#if photo}<img src={assetUrl(photo.id, true)} alt="" />{:else}＋{/if}
              </button>
              {#if photo}<button class="link small" onclick={() => setStudent(s.id, { photo: null })}>{fr.classPhotoRemove}</button>{/if}
            </td>
            <td><input value={s.first_name} onchange={(e) => e.currentTarget.value.trim() && setStudent(s.id, { first_name: e.currentTarget.value.trim() })} /></td>
            <td><input value={s.last_name} onchange={(e) => setStudent(s.id, { last_name: e.currentTarget.value.trim() })} /></td>
            <td class="center"><input type="checkbox" aria-label={fr.classConsent} checked={s.photo_allowed} onchange={(e) => setStudent(s.id, { photo_allowed: e.currentTarget.checked })} /></td>
            <td>
              <button class="symbol" class:empty={!s.symbol} title={fr.classSymbolSearch} onclick={() => (symbolFor = symbolFor === s.id ? null : s.id)}>
                {#if s.symbol}<img src={pictoUrl(s.symbol)} alt="" />{:else}＋{/if}
              </button>
            </td>
            <td>
              <select value={s.level ?? ""} onchange={(e) => setStudent(s.id, { level: (e.currentTarget.value || null) as Level | null })}>
                <option value="">{fr.classNoLevel}</option>
                {#each LEVELS as l (l)}<option value={l}>{l}</option>{/each}
              </select>
            </td>
            <td><input list="class-page-groups" value={s.group} onchange={(e) => setStudent(s.id, { group: e.currentTarget.value.trim() })} /></td>
            <td><button class="remove" title={fr.classRemove} aria-label={fr.classRemove} onclick={() => remove(s)}>🗑</button></td>
          </tr>
          {#if symbolFor === s.id}
            <tr class="symbols-row">
              <td colspan="8">
                <div class="symbols">
                  <!-- svelte-ignore a11y_autofocus -->
                  <input placeholder={fr.classSymbolSearch} bind:value={symbolQuery} autofocus />
                  {#if s.symbol}<button onclick={() => setStudent(s.id, { symbol: null })}>{fr.classSymbolRemove}</button>{/if}
                  <div class="grid">
                    {#each symbols as p (p.code)}
                      <button title={p.fr[0]} onclick={() => (setStudent(s.id, { symbol: p.code }), (symbolFor = null))}><img src={pictoUrl(p.code)} alt="" /></button>
                    {/each}
                  </div>
                </div>
              </td>
            </tr>
          {/if}
        {/each}
        <tr class="add">
          <td></td>
          <td colspan="7">
            <input placeholder={fr.classAddOne} bind:value={newFirst} onkeydown={(e) => e.key === "Enter" && addOne()} />
            <button disabled={!newFirst.trim()} onclick={addOne}>＋ {fr.classAddOneButton}</button>
          </td>
        </tr>
      </tbody>
    </table>
    <datalist id="class-page-groups">
      {#each groupList as g (g)}<option value={g}></option>{/each}
    </datalist>
    <p class="hint">{fr.classPrivacy}</p>
  {/if}
</div>

<style>
  .class-page {
    position: fixed;
    inset: 0;
    background: var(--bg);
    padding: 32px max(24px, calc(50vw - 560px));
    overflow: auto;
    z-index: 5100; /* au-dessus de l'écran d'accueil, qu'elle peut recouvrir */
  }
  .top {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 20px;
  }
  h1 {
    font: 600 24px var(--ui-font);
    margin: 0;
    flex: 1;
  }
  h2 {
    margin: 20px 0 8px;
  }
  .back {
    border-color: transparent;
    background: transparent;
  }
  .head {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .head label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 12px;
    color: var(--muted);
  }
  .name {
    font: 600 16px var(--ui-font);
    width: 16em;
  }
  .year {
    width: 7em;
  }
  .count {
    color: var(--muted);
    padding-bottom: 6px;
  }
  .spacer {
    flex: 1;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    resize: vertical;
  }
  .create {
    max-width: 560px;
  }
  .paste {
    margin-bottom: 12px;
  }
  .row-buttons {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .classes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .classes button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    padding: 8px 12px;
  }
  .classes span {
    font-size: 12px;
    color: var(--muted);
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    background: var(--panel);
    border-radius: 10px;
    overflow: hidden;
  }
  th {
    text-align: left;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    padding: 8px 6px;
    border-bottom: 1px solid var(--line);
  }
  td {
    padding: 4px 6px;
    border-bottom: 1px solid var(--line);
    vertical-align: middle;
  }
  td input:not([type="checkbox"]) {
    width: 100%;
    box-sizing: border-box;
  }
  .center {
    text-align: center;
  }
  td input[type="checkbox"] {
    width: 18px;
    height: 18px;
  }
  .face,
  .symbol {
    width: 40px;
    height: 40px;
    padding: 0;
    border-radius: 50%;
    overflow: hidden;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    vertical-align: middle;
  }
  .symbol {
    border-radius: 8px;
    padding: 4px;
  }
  .face img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .symbol img {
    width: 100%;
    height: 100%;
  }
  .link {
    border: none;
    background: none;
    color: var(--accent);
    padding: 0 4px;
  }
  .small {
    font-size: 11px;
  }
  .remove {
    border-color: transparent;
    background: transparent;
  }
  .symbols {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    padding: 6px 0;
  }
  .symbols input {
    width: 18em;
  }
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    flex-basis: 100%;
  }
  .grid button {
    width: 40px;
    height: 40px;
    padding: 4px;
  }
  .grid img {
    width: 100%;
    height: 100%;
  }
  .add td {
    border-bottom: none;
  }
  .add input {
    width: 16em !important;
    margin-right: 8px;
  }
  .hint {
    font-size: 12px;
    color: var(--muted);
    max-width: 640px;
  }
</style>

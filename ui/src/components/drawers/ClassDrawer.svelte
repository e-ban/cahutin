<script lang="ts">
  // Liste de la classe : coller les prénoms suffit à la créer ; les photos s'ajoutent toutes ensemble,
  // reconnues par le prénom dans le nom du fichier ; l'autorisation se coche sur la ligne de l'élève. Le
  // reste (nom, niveau, groupe, symbole) est sur sa fiche. Tout s'enregistre seul dans Documents/Cahutin/Classes.
  import { open } from "@tauri-apps/plugin-dialog";
  import Drawer from "../layout/Drawer.svelte";
  import { assetUrl, classImportPhoto, classParseList } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { classes, groups, schoolYear } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { pictoUrl, pictos, searchPictos } from "../../lib/pictos.svelte";
  import type { Level, Student } from "../../lib/types";

  let { ongenerate }: { ongenerate: () => void } = $props();

  const LEVELS: Level[] = ["PS", "MS", "GS"];

  let pasted = $state("");
  let more = $state(false);
  let editing = $state<string | null>(null);
  let symbolQuery = $state("");

  $effect(() => {
    if (!classes.current) void classes.refresh();
  });
  $effect(() => void pictos.load());
  // La recherche de symbole repart de zéro pour chaque élève.
  $effect(() => {
    void editing;
    symbolQuery = "";
  });

  let drawerEl = $state<HTMLElement>();
  // Fermeture de la fenêtre ou de l'application : le champ en cours de frappe est validé d'abord.
  $effect(() =>
    editor.onFlush(() => {
      const a = document.activeElement;
      if (a instanceof HTMLInputElement && drawerEl?.contains(a)) a.blur();
    }),
  );

  const cls = $derived(classes.current?.class);
  const groupList = $derived(cls ? groups(cls.students) : []);
  const symbols = $derived(symbolQuery.trim() ? searchPictos(pictos.index, symbolQuery).slice(0, 24) : []);

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
    } catch (e) {
      editor.notice = String(e);
    }
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

  async function remove(s: Student) {
    if (!(await ask(fr.classRemoveStudent(s.first_name), fr.delete))) return;
    classes.update((c) => ({ ...c, students: c.students.filter((x) => x.id !== s.id) }));
    editing = null;
  }

  /** Première liste collée : la classe est créée avec un nom par défaut, à changer d'un clic. */
  async function createFromPaste() {
    if (!pasted.trim()) return;
    await classes.createNamed(fr.classDefaultName, schoolYear(new Date()));
    if (classes.current) await addPasted();
  }

  /** Photos de toute la classe d'un coup : chacune va à l'enfant dont elle porte le prénom. */
  async function addPhotos() {
    const picked = await open({ multiple: true, filters: [{ name: fr.photos, extensions: ["jpg", "jpeg", "png", "webp"] }] });
    const paths = Array.isArray(picked) ? picked : typeof picked === "string" ? [picked] : [];
    if (!paths.length) return;
    const r = await classes.addPhotos(paths, (n) => ask(fr.classConsentAll(n), fr.classConsentYes));
    editor.notice = fr.classPhotosAdded(r.added, r.missed);
  }
</script>

<div class="class-drawer" bind:this={drawerEl}>
<Drawer title={fr.classDrawer}>
  {#if !cls}
    {#if classes.list.length}
      <h3>{fr.classChoose}</h3>
      {#each classes.list as c (c.path)}
        <button class="class" onclick={() => classes.open(c.path)}>
          <b>{c.name}</b>
          <span>{c.year} · {fr.classCount(c.students)}</span>
        </button>
      {/each}
    {/if}
    <h3>{classes.list.length ? fr.classNew : fr.classFirst}</h3>
    <textarea rows="6" bind:value={pasted} placeholder={fr.classPastePlaceholder}></textarea>
    <button class="primary" disabled={!pasted.trim()} onclick={createFromPaste}>＋ {fr.classCreate}</button>
    <p class="hint">{fr.classPrivacy}</p>
  {:else}
    <div class="head">
      <div>
        <input class="title" title={fr.className} value={cls.name} onchange={(e) => e.currentTarget.value.trim() && classes.update((c) => ({ ...c, name: e.currentTarget.value.trim() }))} />
        <span>{cls.year} · {fr.classCount(cls.students.length)}</span>
      </div>
      <button class="link" onclick={() => classes.close()}>{fr.classSwitch}</button>
    </div>
    <button class="primary" disabled={!cls.students.length} onclick={ongenerate}>✨ {fr.classGenerate}</button>
    <button disabled={!cls.students.length} title={fr.classPhotosHint} onclick={addPhotos}>📷 {fr.classPhotosAdd}</button>

    <h3>{fr.classStudents}</h3>
    {#each cls.students as s (s.id)}
      {@const photo = s.photo ? cls.photos[s.photo] : undefined}
      <div class="student" class:open={editing === s.id}>
        <button class="row" onclick={() => (editing = editing === s.id ? null : s.id)}>
          <span class="face">
            {#if photo}<img src={assetUrl(photo.id, true)} alt="" />{:else if s.symbol}<img src={pictoUrl(s.symbol)} alt="" />{/if}
          </span>
          <span class="name">{s.first_name} {s.last_name}</span>
        </button>
        <!-- L'autorisation des parents, sur chaque ligne : on la recueille souvent avant les photos. -->
        <label class="consent" class:off={!s.photo_allowed} title={fr.classConsent}>
          <input type="checkbox" checked={s.photo_allowed} onchange={(e) => setStudent(s.id, { photo_allowed: e.currentTarget.checked })} />
          📷 {fr.classConsentShort}
        </label>
        {#if editing === s.id}
          <div class="edit">
            <label>{fr.classFirstName} <input value={s.first_name} onchange={(e) => e.currentTarget.value.trim() && setStudent(s.id, { first_name: e.currentTarget.value.trim() })} /></label>
            <div class="photo">
              <button onclick={() => choosePhoto(s)}>🖼 {photo ? fr.classPhotoChange : fr.classPhotoChoose}</button>
              {#if photo}<button class="link" onclick={() => setStudent(s.id, { photo: null })}>{fr.classPhotoRemove}</button>{/if}
            </div>
            <span>{fr.classSymbol}</span>
            <input placeholder={fr.classSymbolSearch} bind:value={symbolQuery} />
            <div class="symbols">
              {#if s.symbol}<button class="on" title={fr.classSymbolRemove} onclick={() => setStudent(s.id, { symbol: null })}><img src={pictoUrl(s.symbol)} alt="" /></button>{/if}
              {#each symbols as p (p.code)}
                <button title={p.fr[0]} onclick={() => setStudent(s.id, { symbol: p.code })}><img src={pictoUrl(p.code)} alt="" /></button>
              {/each}
            </div>
            {#if more}
              <label>{fr.classLastName} <input value={s.last_name} onchange={(e) => setStudent(s.id, { last_name: e.currentTarget.value.trim() })} /></label>
              <label>
                {fr.classLevel}
                <select value={s.level ?? ""} onchange={(e) => setStudent(s.id, { level: (e.currentTarget.value || null) as Level | null })}>
                  <option value="">{fr.classNoLevel}</option>
                  {#each LEVELS as l (l)}<option value={l}>{l}</option>{/each}
                </select>
              </label>
              <label>
                {fr.classGroup}
                <input list="class-groups" value={s.group} onchange={(e) => setStudent(s.id, { group: e.currentTarget.value.trim() })} />
              </label>
            {:else}
              <button class="link" onclick={() => (more = true)}>{fr.classMore}</button>
            {/if}
            <button class="danger" onclick={() => remove(s)}>🗑 {fr.classRemove}</button>
          </div>
        {/if}
      </div>
    {/each}
    <h3>{fr.classPaste}</h3>
    <textarea rows="3" bind:value={pasted} placeholder={fr.classPastePlaceholder}></textarea>
    <button disabled={!pasted.trim()} onclick={addPasted}>{fr.classPasteAdd}</button>
    <datalist id="class-groups">
      {#each groupList as g (g)}<option value={g}></option>{/each}
    </datalist>
  {/if}
</Drawer>
</div>

<style>
  .class-drawer {
    display: contents;
  }
  h3 {
    margin: 12px 0 4px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }
  /* Le tiroir est une colonne flexible : sans ces règles, la zone de saisie s'écrase en une ligne. */
  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    flex: none;
    min-height: 4.5em;
    resize: vertical;
  }
  .title {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
    font-weight: 700;
    border: 1px solid transparent;
    background: transparent;
    padding: 1px 2px;
  }
  .title:hover,
  .title:focus {
    border-color: var(--line);
    background: white;
  }
  .student {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
  }
  .student .row {
    flex: 1;
  }
  .student .edit {
    flex-basis: 100%;
  }
  .consent.off {
    color: #9a8f83;
  }
  .consent {
    flex-direction: row;
    align-items: center;
    gap: 3px;
    font-size: 12px;
    color: #4d6b2f;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
    padding: 8px;
  }
  .class,
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    text-align: left;
    width: 100%;
  }
  .class {
    flex-direction: column;
    align-items: flex-start;
  }
  .class span,
  .head span {
    color: #7a6f63;
    font-size: 12px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .head div {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .head .link {
    white-space: nowrap;
    margin-left: 8px;
  }
  .student {
    border-bottom: 1px solid var(--line);
  }
  .row {
    border: none;
    background: transparent;
    padding: 4px 2px;
  }
  .face {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: #f3efe8;
    overflow: hidden;
    flex: none;
  }
  .face img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .name {
    flex: 1;
  }
  .edit {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 4px 2px 10px;
  }
  .photo {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .symbols {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .symbols button {
    width: 34px;
    height: 34px;
    padding: 3px;
  }
  .symbols button.on {
    background: #e8f1fb;
  }
  .symbols img {
    width: 100%;
    height: 100%;
  }
  .link {
    border: none;
    background: none;
    color: var(--accent);
    padding: 0;
  }
  .danger {
    color: #c0392b;
  }
  .hint {
    font-size: 12px;
    color: #7a6f63;
  }
</style>

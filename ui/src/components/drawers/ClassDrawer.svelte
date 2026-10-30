<script lang="ts">
  // Liste de la classe : choisir ou créer une classe, coller des prénoms, régler chaque élève (photo et
  // autorisation, symbole, niveau, groupe). Tout s'enregistre seul dans Documents/Cahutin/Classes.
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

  let newName = $state("");
  let newYear = $state(schoolYear(new Date()));
  let pasted = $state("");
  let editing = $state<string | null>(null);
  let symbolQuery = $state("");

  $effect(() => {
    if (!classes.current) void classes.refresh();
  });
  $effect(() => void pictos.load());

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

  async function create() {
    await classes.create(newName, newYear);
    newName = "";
  }
</script>

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
    <h3>{fr.classNew}</h3>
    <label>{fr.className} <input bind:value={newName} placeholder={fr.classNamePlaceholder} /></label>
    <label>{fr.classYear} <input bind:value={newYear} /></label>
    <button class="primary" disabled={!newName.trim()} onclick={create}>＋ {fr.classCreate}</button>
    <p class="hint">{fr.classPrivacy}</p>
  {:else}
    <div class="head">
      <div>
        <b>{cls.name}</b>
        <span>{cls.year} · {fr.classCount(cls.students.length)}</span>
      </div>
      <button class="link" onclick={() => classes.close()}>{fr.classSwitch}</button>
    </div>
    <button class="primary" disabled={!cls.students.length} onclick={ongenerate}>✨ {fr.classGenerate}</button>

    <h3>{fr.classPaste}</h3>
    <textarea rows="4" bind:value={pasted} placeholder={fr.classPastePlaceholder}></textarea>
    <button disabled={!pasted.trim()} onclick={addPasted}>{fr.classPasteAdd}</button>

    <h3>{fr.classStudents}</h3>
    {#each cls.students as s (s.id)}
      {@const photo = s.photo ? cls.photos[s.photo] : undefined}
      <div class="student" class:open={editing === s.id}>
        <button class="row" onclick={() => (editing = editing === s.id ? null : s.id)}>
          <span class="face">
            {#if photo}<img src={assetUrl(photo.id, true)} alt="" />{:else if s.symbol}<img src={pictoUrl(s.symbol)} alt="" />{/if}
          </span>
          <span class="name">{s.first_name} {s.last_name}</span>
          {#if s.photo && !s.photo_allowed}<span class="warn" title={fr.classNoConsent}>⚠</span>{/if}
        </button>
        {#if editing === s.id}
          <div class="edit">
            <label>{fr.classFirstName} <input value={s.first_name} onchange={(e) => setStudent(s.id, { first_name: e.currentTarget.value.trim() })} /></label>
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
            <div class="photo">
              <button onclick={() => choosePhoto(s)}>🖼 {photo ? fr.classPhotoChange : fr.classPhotoChoose}</button>
              {#if photo}<button class="link" onclick={() => setStudent(s.id, { photo: null })}>{fr.classPhotoRemove}</button>{/if}
            </div>
            <label class="check">
              <input type="checkbox" checked={s.photo_allowed} onchange={(e) => setStudent(s.id, { photo_allowed: e.currentTarget.checked })} />
              {fr.classConsent}
            </label>
            <span>{fr.classSymbol}</span>
            <input placeholder={fr.classSymbolSearch} bind:value={symbolQuery} />
            <div class="symbols">
              {#if s.symbol}<button class="on" title={fr.classSymbolRemove} onclick={() => setStudent(s.id, { symbol: null })}><img src={pictoUrl(s.symbol)} alt="" /></button>{/if}
              {#each symbols as p (p.code)}
                <button title={p.fr[0]} onclick={() => setStudent(s.id, { symbol: p.code })}><img src={pictoUrl(p.code)} alt="" /></button>
              {/each}
            </div>
            <button class="danger" onclick={() => remove(s)}>🗑 {fr.classRemove}</button>
          </div>
        {/if}
      </div>
    {/each}
    <datalist id="class-groups">
      {#each groupList as g (g)}<option value={g}></option>{/each}
    </datalist>
  {/if}
</Drawer>

<style>
  h3 {
    margin: 12px 0 4px;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
  }
  label.check {
    flex-direction: row;
    align-items: center;
    gap: 6px;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    font: inherit;
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
  .warn {
    color: #c0392b;
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

<script lang="ts">
  // « Générer pour la classe » : une page par élève, ou la planche remplie, à partir de la page courante.
  // Les pages s'ajoutent après elle, en un seul pas d'annulation.
  import { classGenerate } from "../lib/api";
  import { classes, filterStudents, groups } from "../lib/classStore.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { insertImported } from "../lib/importPages";
  import type { FillMode, Level } from "../lib/types";

  let { onclose }: { onclose: () => void } = $props();

  let mode = $state<FillMode>("Sheet");
  // « all », « level:MS » ou « group:Les loups ».
  let who = $state("all");
  let error = $state<string | null>(null);
  let busy = $state(false);

  const students = $derived(classes.current?.class.students ?? []);
  const levels = $derived([...new Set(students.map((s) => s.level).filter((l): l is Level => !!l))].sort());
  const groupList = $derived(groups(students));
  const chosen = $derived(
    who === "all"
      ? students
      : who.startsWith("level:")
        ? filterStudents(students, { level: who.slice(6) as Level })
        : filterStudents(students, { group: who.slice(6) }),
  );

  async function generate() {
    error = null;
    busy = true;
    try {
      const result = await classGenerate(editor.page, chosen, mode);
      const placed = insertImported(editor.doc, editor.pageIndex, [result]);
      editor.commit(placed.doc);
      editor.pageIndex = placed.pageIndex;
      editor.selection = [];
      editor.notice = fr.generateDone(result.pages.length);
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="backdrop">
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="generate-title">
    <h2 id="generate-title">{fr.generateTitle}</h2>
    <p>{fr.generateFrom}</p>
    <label class="choice">
      <input type="radio" bind:group={mode} value="PagePerStudent" />
      <span><b>{fr.generatePerStudent}</b> {fr.generatePerStudentHint}</span>
    </label>
    <label class="choice">
      <input type="radio" bind:group={mode} value="Sheet" />
      <span><b>{fr.generateSheet}</b> {fr.generateSheetHint}</span>
    </label>
    <label class="who">
      {fr.generateWho}
      <select bind:value={who}>
        <option value="all">{fr.generateAll}</option>
        {#each levels as l (l)}<option value={`level:${l}`}>{fr.generateLevel(l)}</option>{/each}
        {#each groupList as g (g)}<option value={`group:${g}`}>{fr.generateGroup(g)}</option>{/each}
      </select>
    </label>
    <p class="hint">{fr.generateFields}</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="buttons">
      <button onclick={onclose}>{fr.cancel}</button>
      <button class="primary" disabled={busy || chosen.length === 0} onclick={generate}>{fr.generateGo(chosen.length)}</button>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(40, 30, 20, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    max-width: 480px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.25);
  }
  h2 {
    margin: 0;
    font-size: 18px;
  }
  p {
    margin: 0;
  }
  .choice {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .who {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .hint {
    font-size: 12px;
    color: #7a6f63;
  }
  .error {
    color: #c0392b;
  }
  .buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
</style>

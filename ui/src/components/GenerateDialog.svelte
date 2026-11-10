<script lang="ts">
  // « Remplir avec ma classe » : Cahutin voit si la page est une planche (étiquettes, cartes) ou une page à
  // décliner par élève ; les pages remplies prennent la place de la page modèle, en un pas d'annulation.
  import { classGenerate, classPlan } from "../lib/api";
  import { classes, groups, presetOf, presetStudents } from "../lib/classStore.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { replacePage } from "../lib/importPages";
  import type { GeneratePlan, Level } from "../lib/types";

  let { onclose }: { onclose: () => void } = $props();

  // Choix rapide : « all », « level:MS », « group:Les loups », ou « custom » quand les cases cochées n'en sont aucun.
  let who = $state("all");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let plan = $state<GeneratePlan | null>(null);

  // Le texte en cours d'écriture d'abord : un {prenom} juste posé doit être dans la page lue.
  editor.flushPending();
  classPlan(editor.page)
    .then((p) => (plan = p))
    .catch((e) => (error = String(e)));

  const students = $derived(classes.current?.class.students ?? []);
  const levels = $derived([...new Set(students.map((s) => s.level).filter((l): l is Level => !!l))].sort());
  const groupList = $derived(groups(students));
  const presets = $derived(["all", ...levels.map((l) => `level:${l}`), ...groupList.map((g) => `group:${g}`)]);
  // Les élèves cochés, toute la classe au départ ; l'ordre de la classe est gardé.
  let checked = $state(new Set(classes.current?.class.students.map((s) => s.id) ?? []));
  const chosen = $derived(students.filter((s) => checked.has(s.id)));

  function choosePreset(preset: string) {
    who = preset;
    checked = new Set(presetStudents(students, preset).map((s) => s.id));
  }

  function toggleStudent(id: string, on: boolean) {
    const next = new Set(checked);
    if (on) next.add(id);
    else next.delete(id);
    checked = next;
    who = presetOf(students, next, presets);
  }
  const pages = $derived(!plan ? 0 : plan.mode === "Sheet" ? Math.ceil(chosen.length / Math.max(1, plan.blocks)) : plan.mode === "WholeClass" ? 1 : chosen.length);

  async function generate() {
    if (!plan) return;
    error = null;
    busy = true;
    try {
      editor.flushPending();
      const photos = Object.keys(classes.current?.class.photos ?? {});
      const result = await classGenerate(editor.page, chosen, plan.mode, photos, classes.current?.class ?? null);
      const placed = replacePage(editor.doc, editor.pageIndex, result);
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
    {#if plan}
      <p class="plan">
        {plan.mode === "Sheet" ? fr.generateSheetPlan(chosen.length, plan.blocks, pages) : plan.mode === "WholeClass" ? fr.generateClassPlan(chosen.length) : fr.generatePagesPlan(chosen.length)}
      </p>
    {/if}
    {#if levels.length || groupList.length}
      <label class="who">
        {fr.generateWho}
        <select value={who} onchange={(e) => e.currentTarget.value !== "custom" && choosePreset(e.currentTarget.value)}>
          <option value="all">{fr.generateAll}</option>
          {#each levels as l (l)}<option value={`level:${l}`}>{fr.generateLevel(l)}</option>{/each}
          {#each groupList as g (g)}<option value={`group:${g}`}>{fr.generateGroup(g)}</option>{/each}
          {#if who === "custom"}<option value="custom">{fr.generateCustom}</option>{/if}
        </select>
      </label>
    {/if}
    {#if students.length > 1}
      <details class="pick">
        <summary>{fr.generatePick(chosen.length, students.length)}</summary>
        <div class="pick-all">
          <button onclick={() => choosePreset("all")}>{fr.generatePickAll}</button>
          <button onclick={() => ((checked = new Set()), (who = "custom"))}>{fr.generatePickNone}</button>
        </div>
        <div class="students">
          {#each students as s (s.id)}
            <label><input type="checkbox" checked={checked.has(s.id)} onchange={(e) => toggleStudent(s.id, e.currentTarget.checked)} /> {s.first_name} {s.last_name}</label>
          {/each}
        </div>
      </details>
    {/if}
    <p class="hint">{fr.generateReplaces}</p>
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <div class="buttons">
      <button onclick={onclose}>{fr.cancel}</button>
      <button class="primary" disabled={busy || !plan || chosen.length === 0} onclick={generate}>{fr.generateGo(chosen.length)}</button>
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
  .plan {
    font-size: 15px;
  }
  .who {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .pick summary {
    cursor: pointer;
    color: var(--accent);
  }
  .pick-all {
    display: flex;
    gap: 6px;
    margin: 8px 0 6px;
  }
  .students {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 2px 12px;
    max-height: 40vh;
    overflow-y: auto;
  }
  .students label {
    display: flex;
    gap: 6px;
    align-items: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

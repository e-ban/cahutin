<script lang="ts">
  // « Remplir avec ma classe » : Cahutin voit si la page est une planche (étiquettes, cartes) ou une page à
  // décliner par élève ; les pages remplies prennent la place de la page modèle, en un pas d'annulation.
  import { classGenerate, classPlan } from "../lib/api";
  import { classes, filterStudents, groups } from "../lib/classStore.svelte";
  import { editor } from "../lib/docStore.svelte";
  import { fr } from "../lib/i18n/fr";
  import { replacePage } from "../lib/importPages";
  import type { GeneratePlan, Level } from "../lib/types";

  let { onclose }: { onclose: () => void } = $props();

  // « all », « level:MS » ou « group:Les loups ».
  let who = $state("all");
  let error = $state<string | null>(null);
  let busy = $state(false);
  let plan = $state<GeneratePlan | null>(null);

  // Le texte en cours d'écriture d'abord : un {prénom} juste tapé doit être dans la page lue.
  editor.flushPending();
  classPlan(editor.page)
    .then((p) => (plan = p))
    .catch((e) => (error = String(e)));

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
  const pages = $derived(!plan ? 0 : plan.mode === "Sheet" ? Math.ceil(chosen.length / Math.max(1, plan.blocks)) : chosen.length);

  async function generate() {
    if (!plan) return;
    error = null;
    busy = true;
    try {
      editor.flushPending();
      const photos = Object.keys(classes.current?.class.photos ?? {});
      const result = await classGenerate(editor.page, chosen, plan.mode, photos);
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
      <p class="plan">{plan.mode === "Sheet" ? fr.generateSheetPlan(chosen.length, plan.blocks, pages) : fr.generatePagesPlan(chosen.length)}</p>
    {/if}
    {#if levels.length || groupList.length}
      <label class="who">
        {fr.generateWho}
        <select bind:value={who}>
          <option value="all">{fr.generateAll}</option>
          {#each levels as l (l)}<option value={`level:${l}`}>{fr.generateLevel(l)}</option>{/each}
          {#each groupList as g (g)}<option value={`group:${g}`}>{fr.generateGroup(g)}</option>{/each}
        </select>
      </label>
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

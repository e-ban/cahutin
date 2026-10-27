<script lang="ts">
  // Barre du mode modèle : découpage en cases, fusion, type des zones, libération, enregistrement.
  import { gridLiberate, gridMerge, gridSet, gridSetKind, gridSplit } from "../lib/api";
  import { ask } from "../lib/ask";
  import { editor } from "../lib/docStore.svelte";
  import { mergePair, spannedCell } from "../lib/templateMode";
  import { templates } from "../lib/templates.svelte";
  import type { Page } from "../lib/types";

  let cols = $state(2);
  let rows = $state(4);
  let naming = $state(false);
  let name = $state("");

  const pair = $derived(mergePair(editor.selection, editor.page));
  const spanned = $derived(spannedCell(editor.selection, editor.page));
  const selectedCell = $derived(
    editor.selection.length === 1 && editor.page.grid?.cells.some((c) => c.element_id === editor.selection[0]) ? editor.selection[0] : null,
  );
  const selected = $derived(editor.selected);

  async function run(op: () => Promise<Page>) {
    try {
      const page = await op();
      editor.updatePage(() => page);
    } catch (e) {
      editor.notice = String(e);
    }
  }

  async function apply() {
    if (editor.page.grid && !(await ask("Les zones actuelles de la grille seront remplacées. Continuer ?"))) return;
    await run(() => gridSet(editor.page, cols, rows));
    editor.selection = [];
  }

  async function liberate() {
    if (!(await ask("Libérer les cases ? Elles deviendront des zones à déplacer librement, et la grille disparaîtra."))) return;
    await run(() => gridLiberate(editor.page));
  }

  async function save() {
    if (!name.trim()) return;
    try {
      await templates.saveCurrentPage(name.trim());
      editor.notice = `Modèle « ${name.trim()} » enregistré.`;
      naming = false;
      name = "";
    } catch (e) {
      editor.notice = String(e);
    }
  }
</script>

<div class="bar">
  <span class="title">Mode modèle</span>
  <label>Colonnes <select bind:value={cols}>{#each [1, 2, 3, 4] as n}<option value={n}>{n}</option>{/each}</select></label>
  <label>Lignes <select bind:value={rows}>{#each [1, 2, 3, 4, 5, 6, 7, 8] as n}<option value={n}>{n}</option>{/each}</select></label>
  <button onclick={apply}>▦ Appliquer la grille</button>
  <span class="sep"></span>
  <button disabled={!pair} title="Maj+clic pour sélectionner deux cases" onclick={() => pair && run(() => gridMerge(editor.page, pair[0], pair[1]))}>⊞ Fusionner</button>
  <button disabled={!spanned} onclick={() => spanned && run(() => gridSplit(editor.page, spanned))}>⊟ Scinder</button>
  <button class="photo" disabled={!selectedCell} onclick={() => selectedCell && run(() => gridSetKind(editor.page, selectedCell, "Photo"))}>Zone photo</button>
  <button class="text" disabled={!selectedCell} onclick={() => selectedCell && run(() => gridSetKind(editor.page, selectedCell, "Text"))}>Zone texte</button>
  <button disabled={!editor.page.grid} onclick={liberate}>✥ Libérer</button>
  <span class="sep"></span>
  <button disabled={!selected} onclick={() => selected && editor.updateElement(selected.id, (e) => ({ ...e, locked: !e.locked }))}>
    {selected?.locked ? "🔓 Déverrouiller" : "🔒 Verrouiller"}
  </button>
  <button disabled={!selected || selected.kind.type === "Shape"} onclick={() => selected && editor.updateElement(selected.id, (e) => ({ ...e, zone: !e.zone }))}>
    {selected?.zone ? "Ne plus être une zone" : "Marquer comme zone"}
  </button>
  <span class="sep"></span>
  {#if naming}
    <input placeholder="Nom du modèle" bind:value={name} onkeydown={(e) => e.key === "Enter" && save()} />
    <button onclick={save} disabled={!name.trim()}>Enregistrer</button>
    <button onclick={() => (naming = false)}>Annuler</button>
  {:else}
    <button class="primary" onclick={() => (naming = true)}>💾 Enregistrer comme modèle</button>
  {/if}
</div>

<style>
  .bar {
    grid-area: tbar;
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    padding: 8px 14px;
    background: #fff7e8;
    border-bottom: 1px solid #f0d9a8;
  }
  .title {
    font-weight: 700;
    color: #b9770e;
  }
  label {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .sep {
    width: 1px;
    height: 22px;
    background: #f0d9a8;
  }
  .photo {
    border-color: var(--zone-photo);
  }
  .text {
    border-color: var(--zone-text);
  }
  .primary {
    background: #2e86de;
    color: white;
    border-color: #2e86de;
  }
</style>

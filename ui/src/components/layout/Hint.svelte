<script lang="ts">
  // Astuce discrète en bas de la page, liée à ce que l'on fait ; chacune au plus 3 fois.
  import { untrack } from "svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { firstHint, hints, type HintId } from "../../lib/hints.svelte";
  import { selectedTable } from "../../lib/table";
  import { fr } from "../../lib/i18n/fr";

  const TEXTS: Record<HintId, string> = {
    "dblclick-crop": fr.hintDblclickCrop,
    "drop-photo": fr.hintDropPhoto,
    "dblclick-text": fr.hintDblclickText,
    rightclick: fr.hintRightclick,
    "blur-faces": fr.hintBlurFaces,
    "table-cell": fr.hintTableCell,
  };

  // Nombre d'éléments différents sélectionnés depuis le lancement.
  let selectedCount = $state(0);
  let lastSelected: string | null = null;
  $effect(() => {
    const sel = editor.selection;
    if (sel.length === 1 && sel[0] !== lastSelected) {
      lastSelected = sel[0];
      untrack(() => selectedCount++);
    }
  });

  // Les astuces possibles, dans l'ordre : la première pas encore vue 3 fois est montrée.
  const wanted = $derived.by((): HintId[] => {
    if (editor.templateMode || editor.cropping || editor.editingText || editor.showWelcome || editor.printing) return [];
    if (selectedTable(editor.page, editor.selection)) return ["table-cell"];
    const e = editor.selected;
    if (e && selectedCount === 3) return ["rightclick"];
    if (e?.kind.type === "Photo" && e.kind.asset) return ["dblclick-crop", "blur-faces"];
    if (e?.kind.type === "Text" && e.zone) return ["dblclick-text"];
    const emptyZone = editor.page.elements.some((x) => x.zone && x.kind.type === "Photo" && !x.kind.asset);
    if (!e && editor.photos.length > 0 && emptyZone) return ["drop-photo"];
    return [];
  });

  let current = $state<HintId | null>(null);
  $effect(() => {
    const w = wanted;
    const enabled = hints.enabled;
    untrack(() => {
      if (current && w.includes(current)) return;
      current = firstHint(enabled, hints.counts, w);
      if (current) hints.shown(current);
    });
  });
</script>

{#if current}
  <div class="hint" role="status">
    <span>💡 {TEXTS[current]}</span>
    <button title={fr.closeHint} aria-label={fr.closeHint} onclick={() => (current = null)}>×</button>
  </div>
{/if}

<style>
  .hint {
    display: flex;
    align-items: center;
    gap: 8px;
    background: #2d2a26;
    color: white;
    border-radius: 18px;
    padding: 6px 8px 6px 14px;
    font-size: 13px;
    box-shadow: 0 4px 14px rgba(0, 0, 0, 0.2);
    white-space: nowrap;
  }
  button {
    border: 0;
    background: transparent;
    color: #ccc;
    padding: 0 6px;
  }
</style>

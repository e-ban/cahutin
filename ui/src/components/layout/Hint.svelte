<script lang="ts">
  // Astuce discrète en bas de la page, liée à ce que l'on fait ; chacune au plus 3 fois.
  import Icon from "../Icon.svelte";
  import { untrack } from "svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { firstHint, hints, type HintId } from "../../lib/hints.svelte";
  import { selectedTable } from "../../lib/table";
  import { fr } from "../../lib/i18n/fr";
  import { tour } from "../../lib/tour.svelte";

  const TEXTS: Record<HintId, string> = {
    "dblclick-crop": fr.hintDblclickCrop,
    "drop-photo": fr.hintDropPhoto,
    "dblclick-text": fr.hintDblclickText,
    rightclick: fr.hintRightclick,
    "blur-faces": fr.hintBlurFaces,
    "table-cell": fr.hintTableCell,
    "first-select": fr.hintFirstSelect,
    "first-drawer": fr.hintFirstDrawer,
    "tour-in-help": fr.hintTourInHelp,
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
    // Première séance, à la place de la visite : la barre à la première sélection, l'ajout au premier tiroir ; en
    // dernier, quand rien n'est choisi, où retrouver la visite.
    const first = tour.firstSession;
    const drawerAdds = editor.drawer === "images" || editor.drawer === "text" || editor.drawer === "shapes";
    const intro: HintId[] = !first ? [] : editor.selection.length > 0 ? ["first-select"] : drawerAdds ? ["first-drawer"] : [];
    const e = editor.selected;
    const usual = ((): HintId[] => {
      if (e && selectedCount === 3) return ["rightclick"];
      if (e?.kind.type === "Photo" && e.kind.asset) return ["dblclick-crop", "blur-faces"];
      if (e?.kind.type === "Text" && e.zone) return ["dblclick-text"];
      const emptyZone = editor.page.elements.some((x) => x.zone && x.kind.type === "Photo" && !x.kind.asset);
      if (!e && editor.photos.length > 0 && emptyZone) return ["drop-photo"];
      return [];
    })();
    const outro: HintId[] = first && editor.selection.length === 0 ? ["tour-in-help"] : [];
    return [...intro, ...usual, ...outro];
  });

  let current = $state<HintId | null>(null);
  $effect(() => {
    const w = wanted;
    const enabled = hints.enabled;
    untrack(() => {
      const next = firstHint(enabled, hints.counts, w);
      // L'astuce montrée reste tant qu'elle vaut encore, sauf si une plus utile arrive avant elle (un tiroir ouvert).
      if (current && w.includes(current) && (next === null || w.indexOf(next) >= w.indexOf(current))) return;
      current = next;
      if (current) hints.shown(current);
    });
  });
</script>

{#if current}
  <div class="hint" role="status">
    <span><Icon name="lightbulb" /> {TEXTS[current]}</span>
    <button title={fr.closeHint} aria-label={fr.closeHint} onclick={() => (current = null)}><Icon name="close" size={14} /></button>
  </div>
{/if}

<style>
  .hint {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--text);
    color: var(--panel);
    border-radius: 18px;
    padding: 6px 8px 6px 14px;
    font-size: 13px;
    box-shadow: 0 4px 14px var(--shadow);
    white-space: nowrap;
  }
  button {
    border: 0;
    background: transparent;
    color: color-mix(in srgb, var(--panel) 75%, transparent);
    padding: 0 6px;
  }
</style>

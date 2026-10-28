<script lang="ts">
  // Mode modèle : glisser d'une case à une autre fusionne le rectangle couvert.
  // Pendant le glissement, les cases couvertes se surlignent (en rouge si la fusion est impossible).
  import { gridMerge } from "../lib/api";
  import { editor } from "../lib/docStore.svelte";
  import { cellsCoveredBy } from "../lib/templateMode";

  let { pageEl }: { pageEl: HTMLElement | undefined } = $props();

  let from: string | null = null;
  let to: string | null = null;

  const isCell = (id: string | undefined) => !!id && !!editor.page.grid?.cells.some((c) => c.element_id === id);

  function cellAt(x: number, y: number): string | null {
    const id = document.elementFromPoint(x, y)?.closest<HTMLElement>(".page .element")?.dataset.id;
    return isCell(id) ? id! : null;
  }

  function onDown(ev: PointerEvent) {
    if (!editor.templateMode || ev.button !== 0 || editor.editingText) return;
    const id = (ev.target as HTMLElement).closest<HTMLElement>(".element")?.dataset.id;
    from = isCell(id) ? id! : null;
    to = from;
  }

  function onMove(ev: PointerEvent) {
    if (!from) return;
    const id = cellAt(ev.clientX, ev.clientY);
    if (!id || id === to) return;
    to = id;
    if (to === from) {
      editor.mergePreview = null;
      return;
    }
    const covered = cellsCoveredBy(editor.page, from, to);
    editor.mergePreview = covered ? { ids: covered, ok: true } : { ids: [from, to], ok: false };
  }

  async function onUp() {
    const [a, b, preview] = [from, to, editor.mergePreview];
    from = to = null;
    editor.mergePreview = null;
    if (!a || !b || a === b || !preview?.ok) return;
    const pageId = editor.page.id;
    try {
      const page = await gridMerge(editor.page, a, b);
      editor.updatePageById(pageId, () => page);
      editor.selection = [a];
    } catch (e) {
      editor.notice = String(e);
    }
  }

  $effect(() => {
    const el = pageEl;
    if (!el) return;
    el.addEventListener("pointerdown", onDown);
    return () => el.removeEventListener("pointerdown", onDown);
  });
</script>

<svelte:window onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp} />

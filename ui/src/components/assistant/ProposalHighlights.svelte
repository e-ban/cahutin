<script lang="ts">
  // Cadres en pointillés autour de ce que l'assistant a changé : l'étape affichée, ou toute la proposition.
  import { tick } from "svelte";
  import { assistant } from "../../lib/assistant.svelte";
  import { editor } from "../../lib/docStore.svelte";

  let { pageEl, stageEl }: { pageEl: HTMLElement | undefined; stageEl: HTMLElement | undefined } = $props();

  const ids = $derived.by(() => {
    if (assistant.viewing) return assistant.viewing.touched;
    return [...new Set(assistant.proposal?.steps.flatMap((s) => s.touched) ?? [])];
  });

  let boxes = $state<{ left: number; top: number; width: number; height: number }[]>([]);

  async function measure() {
    await tick();
    if (!pageEl || !editor.proposing) {
      boxes = [];
      return;
    }
    boxes = ids
      .map((id) => pageEl!.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`)?.getBoundingClientRect())
      .filter((r): r is DOMRect => !!r)
      .map((r) => ({ left: r.left - 4, top: r.top - 4, width: r.width + 8, height: r.height + 8 }));
  }

  $effect(() => {
    void [ids, editor.doc, editor.effectiveZoom, editor.pageIndex, editor.proposing];
    void measure();
  });

  $effect(() => {
    const el = stageEl;
    const again = () => void measure();
    el?.addEventListener("scroll", again);
    window.addEventListener("resize", again);
    return () => {
      el?.removeEventListener("scroll", again);
      window.removeEventListener("resize", again);
    };
  });
</script>

{#each boxes as b, i (i)}
  <div class="hl" style:left="{b.left}px" style:top="{b.top}px" style:width="{b.width}px" style:height="{b.height}px"></div>
{/each}

<style>
  .hl {
    position: fixed;
    border: 2px dashed var(--accent);
    border-radius: 6px;
    pointer-events: none;
    z-index: 5;
  }
</style>

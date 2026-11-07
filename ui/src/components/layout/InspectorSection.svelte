<script lang="ts">
  // Section repliable du panneau Réglages (et des tiroirs) : un clic sur son titre l'ouvre ou la ferme, et elle
  // reste ainsi.
  import type { Snippet } from "svelte";
  import { inspectorSections } from "../../lib/inspectorSections.svelte";

  let { id, title, open = true, children }: { id: string; title: string; open?: boolean; children: Snippet } = $props();
</script>

<details class="section" open={inspectorSections.isOpen(id, open)} ontoggle={(e) => inspectorSections.set(id, e.currentTarget.open)}>
  <summary>{title}</summary>
  <div class="body">{@render children()}</div>
</details>

<style>
  .section {
    border-top: 1px solid var(--line);
  }
  summary {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 0;
    font-weight: 600;
    cursor: pointer;
    list-style: none;
    user-select: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  summary::before {
    content: "▸";
    width: 1em;
    color: var(--muted);
    transition: transform 0.15s;
  }
  .section[open] > summary::before {
    transform: rotate(90deg);
  }
  summary:hover {
    color: var(--accent);
  }
  .body {
    padding-bottom: 6px;
  }
</style>

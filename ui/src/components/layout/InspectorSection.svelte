<script lang="ts">
  // Section repliable du panneau Réglages (et des tiroirs) : un clic sur son titre l'ouvre ou la ferme, et elle
  // reste ainsi.
  import type { Snippet } from "svelte";
  import { inspectorSections } from "../../lib/inspectorSections.svelte";

  let { id, title, open = true, hint, children }: { id: string; title: string; open?: boolean; hint?: string; children: Snippet } = $props();

  const isOpen = $derived(inspectorSections.isOpen(id, open));
</script>

<details class="section" open={isOpen} ontoggle={(e) => inspectorSections.set(id, e.currentTarget.open)}>
  <summary>
    <span class="title">{title}</span>
    <!-- Repliée, elle dit en quelques mots ce qu'elle contient. -->
    {#if hint && !isOpen}<small class="hint">{hint}</small>{/if}
  </summary>
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
  summary {
    flex-wrap: wrap;
  }
  .title {
    flex: 1;
  }
  .hint {
    flex-basis: 100%;
    padding-left: calc(1em + 8px);
    font-weight: 400;
    color: var(--muted);
  }
  summary:hover .hint {
    color: var(--muted);
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

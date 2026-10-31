<script lang="ts">
  // Tiroir Classe, à côté du document : la classe choisie et le remplissage de la page. La classe s'édite
  // sur sa page dédiée (« Modifier la classe… »).
  import Drawer from "../layout/Drawer.svelte";
  import { classes } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  let { ongenerate }: { ongenerate: () => void } = $props();

  $effect(() => void classes.refresh());

  const cls = $derived(classes.current?.class);
  const others = $derived(classes.list.filter((c) => c.path !== classes.current?.path));
</script>

<Drawer title={fr.classDrawer}>
  {#if cls}
    <div class="current">
      <b>{cls.name}</b>
      <span>{cls.year} · {fr.classCount(cls.students.length)}</span>
    </div>
    <button class="primary" disabled={!cls.students.length} onclick={ongenerate}>✨ {fr.classGenerate}</button>
    <button onclick={() => (editor.showClassPage = true)}>✎ {fr.classEdit}</button>
  {:else}
    <p class="hint">{classes.list.length ? fr.classChoose : fr.classNone}</p>
    <button class="primary" onclick={() => (editor.showClassPage = true)}>✎ {fr.classEdit}</button>
  {/if}
  {#if others.length}
    <h3>{cls ? fr.classSwitch : fr.classChoose}</h3>
    {#each others as c (c.path)}
      <button class="class" onclick={() => classes.open(c.path)}>
        <b>{c.name}</b>
        <span>{c.year} · {fr.classCount(c.students)}</span>
      </button>
    {/each}
  {/if}
</Drawer>

<style>
  h3 {
    margin: 12px 0 4px;
  }
  .current,
  .class {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
  }
  .current span,
  .class span {
    color: #7a6f63;
    font-size: 12px;
  }
  .class {
    text-align: left;
    width: 100%;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
    font-weight: 600;
    padding: 8px;
  }
  .hint {
    font-size: 13px;
    color: #7a6f63;
  }
</style>

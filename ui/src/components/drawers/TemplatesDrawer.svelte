<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import PageView from "../PageView.svelte";
  import { ask } from "../../lib/ask";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { templates } from "../../lib/templates.svelte";
</script>

{#snippet tile(t: (typeof templates.infos)[number])}
  <div class="tpl">
    <button class="mini" title={fr.newPageFrom(t.name)} onclick={() => templates.addPage(t.id).catch((e) => (editor.notice = String(e)))}>
      {#if templates.previews[t.id]}
        <PageView page={templates.previews[t.id]} doc={editor.doc} zoom={0.13 * Math.min(1, 210 / templates.previews[t.id].size.width_mm, 297 / templates.previews[t.id].size.height_mm)} />
      {/if}
    </button>
    <span class="name">{t.name}</span>
    {#if !t.builtin}
      <button
        class="del"
        title={fr.deleteTemplate}
        aria-label={fr.deleteTemplate}
        onclick={async () => {
          if (await ask(fr.confirmDeleteTemplate(t.name), fr.delete)) templates.remove(t.id).catch((e) => (editor.notice = String(e)));
        }}>×</button
      >
    {/if}
  </div>
{/snippet}

<Drawer title={fr.templates}>
  <div class="templates">
    {#each templates.infos.filter((t) => !t.for_class) as t (t.id)}{@render tile(t)}{/each}
  </div>
  <h3 title={fr.templatesForClassHint}>{fr.templatesForClass}</h3>
  <div class="templates">
    {#each templates.infos.filter((t) => t.for_class) as t (t.id)}{@render tile(t)}{/each}
  </div>
</Drawer>

<style>
  .templates {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }
  .tpl {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }
  .mini {
    padding: 4px;
    width: 112px;
    height: 154px;
    overflow: hidden;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .mini :global(.page) {
    pointer-events: none;
  }
  .name {
    font-size: 12px;
    text-align: center;
  }
  .del {
    position: absolute;
    top: 0;
    right: 0;
    padding: 0 6px;
    border-radius: 50%;
  }
</style>

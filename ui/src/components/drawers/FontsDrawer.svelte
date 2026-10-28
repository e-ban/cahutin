<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import Drawer from "../layout/Drawer.svelte";
  import { addFont, removeFont } from "../../lib/api";
  import { ask } from "../../lib/ask";
  import { editor } from "../../lib/docStore.svelte";
  import { fonts } from "../../lib/fonts.svelte";
  import { groupFamilies, searchFamilies } from "../../lib/fontSearch";
  import { fr } from "../../lib/i18n/fr";
  import { setFont } from "../../lib/textFormat.svelte";
  import type { FontFamilyInfo } from "../../lib/types";

  let query = $state("");
  const groups = $derived(groupFamilies(searchFamilies(fonts.families, query)));
  const current = $derived(editor.selected?.kind.type === "Text" ? editor.selected.kind.style.font : null);
  let explain = $state<string | null>(null);

  async function pick() {
    const path = await open({ filters: [{ name: "Polices", extensions: ["ttf", "otf", "TTF", "OTF"] }] });
    if (typeof path !== "string") return;
    try {
      const info = await addFont(path);
      await fonts.load();
      editor.notice = fr.fontAdded(info.name);
    } catch (e) {
      editor.notice = String(e);
    }
  }

  async function remove(f: FontFamilyInfo) {
    if (!(await ask(fr.confirmRemoveFont(f.name), fr.delete))) return;
    try {
      await removeFont(f.name);
      await fonts.load();
    } catch (e) {
      editor.notice = String(e);
    }
  }

  function apply(f: FontFamilyInfo) {
    if (editor.selected?.kind.type === "Text") setFont(f.name);
  }
</script>

{#snippet row(f: FontFamilyInfo)}
  <div class="font" class:on={current === f.name}>
    <button class="name" style:font-family={`"${f.name}", "Andika"`} onpointerdown={(e) => e.preventDefault()} onclick={() => apply(f)}>{f.name}</button>
    {#if f.kind}<small class="tag">{f.kind}</small>{/if}
    {#if !f.embeddable}
      <button class="tag warn" onclick={() => (explain = explain === f.name ? null : f.name)}>{fr.exportWarning}</button>
    {/if}
    {#if f.user}<button class="del" title={fr.removeFont} aria-label={fr.removeFont} onclick={() => remove(f)}>×</button>{/if}
  </div>
  {#if explain === f.name}<p class="explain">{fr.restrictedFontExplain}</p>{/if}
{/snippet}

<Drawer title={fr.fonts}>
  <button class="add" onclick={pick}>{fr.addFont}</button>
  <input class="search" type="search" bind:value={query} placeholder={fr.searchFont} aria-label={fr.searchFont} />
  <p class="hint">{fr.applyFontHint}</p>
  {#if groups.builtin.length}
    <h3>{fr.builtinFonts}</h3>
    {#each groups.builtin as f (f.name)}{@render row(f)}{/each}
  {/if}
  {#if groups.user.length || !query}
    <h3>{fr.userFonts}</h3>
    {#each groups.user as f (f.name)}{@render row(f)}{:else}<p class="hint">{fr.noUserFonts}</p>{/each}
  {/if}
  {#if groups.system.length}
    <h3>{fr.systemFonts} ({groups.system.length})</h3>
    {#each groups.system as f (f.name)}{@render row(f)}{/each}
  {/if}
  {#if query && !groups.builtin.length && !groups.user.length && !groups.system.length}
    <p class="hint">{fr.noFontFound}</p>
  {/if}
</Drawer>

<style>
  .add {
    padding: 8px;
  }
  h3 {
    margin: 6px 0 0;
  }
  .search {
    padding: 6px 8px;
  }
  .font {
    content-visibility: auto;
    contain-intrinsic-size: auto 36px;
    background: white;
    border: 1px solid var(--line);
    border-radius: 7px;
    padding: 2px 6px 2px 2px;
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .font.on {
    border-color: var(--accent);
    background: #f2f8fe;
  }
  .name {
    flex: 1;
    min-width: 0;
    text-align: left;
    font-size: 16px;
    border: 0;
    background: transparent;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    font-size: 10px;
    border-radius: 8px;
    padding: 1px 6px;
    background: #eee;
    color: #666;
    border: 0;
    white-space: nowrap;
  }
  .tag.warn {
    background: #fff1e0;
    color: #b9770e;
  }
  .del {
    border: 0;
    background: transparent;
    padding: 0 4px;
    color: var(--muted);
  }
  .explain {
    margin: 0;
    font-size: 12px;
    background: #fff8ef;
    border: 1px solid #f0d9a8;
    border-radius: 6px;
    padding: 6px 8px;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
</style>

<script lang="ts">
  // Tiroir Images : toutes les images au même endroit. Photos (celles de ce document, la photothèque de la
  // classe), Mes images (gardées pour tous les documents), Pictogrammes (OpenMoji). Une recherche cherche à la
  // fois dans Mes images et les pictogrammes.
  import { onDestroy } from "svelte";
  import Drawer from "../layout/Drawer.svelte";
  import MyImagesPanel from "./MyImagesPanel.svelte";
  import PhotosPanel from "./PhotosPanel.svelte";
  import PictoPanel from "./PictoPanel.svelte";
  import { editor, type ImagesTab } from "../../lib/docStore.svelte";
  import { photoZoneElement } from "../../lib/factory";
  import { fr } from "../../lib/i18n/fr";

  // Le mode « Remplacer » prend fin quand on choisit autre chose sur la page, ou qu'on ferme le tiroir.
  onDestroy(() => ((editor.replacingPictos = false), (editor.replacingPhoto = false)));
  let selectionWhenOpened = editor.selection;
  $effect.pre(() => {
    if (editor.selection !== selectionWhenOpened) {
      selectionWhenOpened = editor.selection;
      editor.replacingPictos = false;
      editor.replacingPhoto = false;
    }
  });

  let query = $state("");

  function addPhotoZone() {
    const el = photoZoneElement(editor.page);
    editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
    editor.selection = [el.id];
  }

  // Pendant « Remplacer » un pictogramme, seuls les pictogrammes comptent.
  const tabs = $derived(
    (
      [
        { id: "photos", label: fr.photos },
        { id: "mine", label: fr.myImages },
        { id: "pictos", label: fr.pictograms },
      ] as { id: ImagesTab; label: string }[]
    ).filter((t) => t.id === "pictos" || !editor.replacingPictos),
  );
  const tab = $derived(tabs.some((t) => t.id === editor.imagesTab) ? editor.imagesTab : "pictos");
</script>

<Drawer title={fr.images}>
  <div class="tabs" role="tablist">
    {#each tabs as t (t.id)}
      <button role="tab" aria-selected={tab === t.id} class:on={tab === t.id} onclick={() => ((query = ""), editor.setImagesTab(t.id))}>{t.label}</button>
    {/each}
  </div>
  {#if tab === "photos" && editor.templateMode}
    <!-- Un modèle n'a pas de photos, seulement des cases où l'on en déposera une. -->
    <button class="zone" onclick={addPhotoZone}>⬚ {fr.photoZoneAdd}</button>
    <p class="hint">{fr.photoZoneHint}</p>
  {:else if tab === "photos"}
    <PhotosPanel />
  {:else}
    <input class="search" type="search" bind:value={query} placeholder={fr.searchPicture} aria-label={fr.searchPicture} />
    {#if query.trim()}
      {#if !editor.replacingPictos}<MyImagesPanel {query} searching />{/if}
      <h4>{fr.pictograms}</h4>
      <PictoPanel {query} />
    {:else if tab === "mine"}
      <MyImagesPanel onphotos={editor.templateMode ? undefined : () => editor.setImagesTab("photos")} />
    {:else}
      <PictoPanel />
    {/if}
  {/if}
</Drawer>

<style>
  .tabs {
    display: flex;
    border: 1px solid var(--line);
    border-radius: 8px;
    overflow: hidden;
  }
  .tabs button {
    flex: 1 1 auto;
    border: 0;
    border-radius: 0;
    padding: 7px 2px;
    font-size: 12px;
    white-space: nowrap;
    background: white;
  }
  .tabs button + button {
    border-left: 1px solid var(--line);
  }
  .tabs .on {
    background: #e8f1fb;
    color: var(--accent);
    font-weight: 600;
  }
  .zone {
    padding: 8px;
    font-size: 14px;
    font-weight: 600;
  }
  .hint {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .search {
    padding: 6px 8px;
  }
  h4 {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--muted);
    text-transform: uppercase;
  }
</style>

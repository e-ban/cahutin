<script lang="ts">
  // Tiroir Images : toutes les images au même endroit. Photos (celles de ce document, la photothèque de la
  // classe), Mes images (gardées pour tous les documents), Pictogrammes (OpenMoji). Une recherche cherche à la
  // fois dans Mes images et les pictogrammes.
  import Icon from "../Icon.svelte";
  import { onDestroy } from "svelte";
  import Drawer from "../layout/Drawer.svelte";
  import MyImagesPanel from "./MyImagesPanel.svelte";
  import PhotosPanel from "./PhotosPanel.svelte";
  import PictoPanel from "./PictoPanel.svelte";
  import ImagesWide from "./ImagesWide.svelte";
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
  // Une image de la vue agrandie est en train d'être glissée : le tiroir reprend sa largeur.
  let dragging = $state(false);

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

  // Agrandi, un clic n'importe où hors du tiroir (la page, les barres) le réduit, et ne fait rien d'autre.
  // Les boîtes de dialogue et les menus ouverts au-dessus ne comptent pas.
  $effect(() => {
    if (!editor.imagesWide || dragging) return;
    let swallowing = false;
    const outside = (t: EventTarget | null) =>
      t instanceof Element && !t.closest('aside.drawer, [role="dialog"], [role="menu"], [role="listbox"]');
    const down = (e: Event) => {
      if (!outside(e.target)) return;
      swallowing = true;
      e.preventDefault();
      e.stopPropagation();
      editor.closeImagesWide();
    };
    const swallow = (e: Event) => {
      if (!swallowing) return;
      e.preventDefault();
      e.stopPropagation();
      if (e.type === "click") swallowing = false;
    };
    window.addEventListener("pointerdown", down, true);
    for (const t of ["mousedown", "pointerup", "mouseup", "click"]) window.addEventListener(t, swallow, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      // Le reste du clic qui a réduit (relâchement, clic) arrive après : il est encore avalé, puis on s'arrête.
      setTimeout(() => {
        for (const t of ["mousedown", "pointerup", "mouseup", "click"]) window.removeEventListener(t, swallow, true);
      }, 600);
    };
  });
  const edge = $derived(
    !editor.templateMode && !editor.replacingPictos
      ? {
          label: editor.imagesWide ? fr.imagesNarrow : fr.imagesWideHint,
          ontoggle: () => (editor.imagesWide ? editor.closeImagesWide() : editor.openImagesWide()),
        }
      : undefined,
  );
</script>

<Drawer title={fr.images} wide={editor.imagesWide && !dragging} {edge}>
  {#if editor.imagesWide}
    <!-- Pendant un glisser, la vue agrandie reste dans la page (l'image tenue en fait partie : la retirer
         annulerait le glisser) mais ne se voit plus ; le tiroir montre son contenu habituel. -->
    <div class="wide-host" class:held={dragging}>
      <ImagesWide ondrag={(d) => (dragging = d)} />
    </div>
  {/if}
  {#if !editor.imagesWide || dragging}
  <div class="tabs" role="tablist">
    {#each tabs as t (t.id)}
      <button role="tab" data-guide="tab-{t.id}" aria-selected={tab === t.id} class:on={tab === t.id} onclick={() => ((query = ""), editor.setImagesTab(t.id))}>{t.label}</button>
    {/each}
  </div>
  {#if tab === "photos" && editor.templateMode}
    <!-- Un modèle n'a pas de photos, seulement des cases où l'on en déposera une. -->
    <button class="zone" onclick={addPhotoZone}><Icon name="frame" /> {fr.photoZoneAdd}</button>
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
  {/if}
</Drawer>

<style>
  .wide-host {
    display: contents;
  }
  /* Hors de la vue, sans place ni clic, mais toujours dans la page. */
  .wide-host.held {
    display: block;
    position: fixed;
    left: -10000px;
    top: 0;
    width: 1px;
    height: 1px;
    overflow: hidden;
    opacity: 0;
    pointer-events: none;
  }
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
    background: var(--surface);
  }
  .tabs button + button {
    border-left: 1px solid var(--line);
  }
  .tabs .on {
    background: var(--accent-soft);
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

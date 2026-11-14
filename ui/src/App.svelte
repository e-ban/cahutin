<script lang="ts">
  import "./fonts.css";
  import "./print.css";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount, tick, untrack } from "svelte";
  import Modal from "./components/Modal.svelte";
  import DateDialog from "./components/DateDialog.svelte";
  import PrintRoot from "./components/PrintRoot.svelte";
  import { cancelFileImport, confirmLeave, exportOdg, exportPdf, importFiles, importPaths, openExampleDoc, openPath, save } from "./lib/fileActions";
  import { fr } from "./lib/i18n/fr";
  import { ask } from "./lib/ask";
  import BoxSelect from "./components/BoxSelect.svelte";
  import ContextBar from "./components/ContextBar.svelte";
  import ContextMenu, { type ContextTarget } from "./components/layout/ContextMenu.svelte";
  import { facingOf } from "./lib/spread";
  import { classLibrary, warnConsent } from "./lib/phototheque.svelte";
  import PhotothequePage from "./components/layout/PhotothequePage.svelte";
  import PageView from "./components/PageView.svelte";
  import CellBar from "./components/CellBar.svelte";
  import TableBar from "./components/TableBar.svelte";
  import GridDragMerge from "./components/GridDragMerge.svelte";
  import TemplateBanner from "./components/layout/TemplateBanner.svelte";
  import ClassBanner from "./components/layout/ClassBanner.svelte";
  import { hasNameFields } from "./lib/classFill";
  import GridDrawer from "./components/drawers/GridDrawer.svelte";
  import DecorDrawer from "./components/drawers/DecorDrawer.svelte";
  import ClassDrawer from "./components/drawers/ClassDrawer.svelte";
  import GenerateDialog from "./components/GenerateDialog.svelte";
  import FaceBlurDialog from "./components/FaceBlurDialog.svelte";
  import { classes } from "./lib/classStore.svelte";
  import { completeTypedText, fillWaitingClassFields } from "./lib/fieldPlacing.svelte";
  import About from "./components/layout/About.svelte";
  import Preferences from "./components/layout/Preferences.svelte";
  import BackgroundPanel from "./components/BackgroundPanel.svelte";
  import Hint from "./components/layout/Hint.svelte";
  import Tour from "./components/Tour.svelte";
  import TipsPanel from "./components/TipsPanel.svelte";
  import { shouldStartTour, tour, tourMayStart } from "./lib/tour.svelte";
  import { modal } from "./lib/modal.svelte";
  import Welcome from "./components/layout/Welcome.svelte";
  import ClassPage from "./components/layout/ClassPage.svelte";
  import WhatsNew from "./components/layout/WhatsNew.svelte";
  import { shouldShowWhatsNew, WHATS_NEW } from "./lib/whatsNew";
  import { readPref, writePref } from "./lib/prefs";
  import UpdateBanner from "./components/layout/UpdateBanner.svelte";
  import { updates } from "./lib/updates.svelte";
  import Transformer from "./components/Transformer.svelte";
  import GroupTransformer from "./components/GroupTransformer.svelte";
  import BottomBar from "./components/layout/BottomBar.svelte";
  import Inspector from "./components/layout/Inspector.svelte";
  import Rail from "./components/layout/Rail.svelte";
  import TopBar from "./components/layout/TopBar.svelte";
  import FontsDrawer from "./components/drawers/FontsDrawer.svelte";
  import ImagesDrawer from "./components/drawers/ImagesDrawer.svelte";
  import CollectionPage from "./components/layout/CollectionPage.svelte";
  import ShapesDrawer from "./components/drawers/ShapesDrawer.svelte";
  import TemplatesDrawer from "./components/drawers/TemplatesDrawer.svelte";
  import TextDrawer from "./components/drawers/TextDrawer.svelte";
  import FieldsDrawer from "./components/drawers/FieldsDrawer.svelte";
  import { editor } from "./lib/docStore.svelte";
  import { assistant } from "./lib/assistant.svelte";
  import AssistantPanel from "./components/assistant/AssistantPanel.svelte";
  import ProposalHighlights from "./components/assistant/ProposalHighlights.svelte";
  import ProposalBar from "./components/assistant/ProposalBar.svelte";
  import { fonts } from "./lib/fonts.svelte";
  import { addPhotoAt } from "./lib/editActions";
  import { dropFiles, guardWindowDrop, handlePasteEvent } from "./lib/incoming";
  import { blockBelow, expandSelection, groupOf, pathOf } from "./lib/groups";
  import { pictoElement, pictos } from "./lib/pictos.svelte";
  import { endNudge, handleKeyUp, handleShortcut, handleWheelZoom } from "./lib/shortcuts";
  import { roundedFrame } from "./lib/crop";
  import { newDocument, newId } from "./lib/factory";
  import { templates } from "./lib/templates.svelte";
  import { mmToPx } from "./lib/units";
  import type { AssetMeta, Crop, Frame, RichText } from "./lib/types";
  import { myImages } from "./lib/myImages.svelte";
  import { settings } from "./lib/settings.svelte";
  import { guides } from "./lib/guides.svelte";
  import { finishedText } from "./lib/textLook";
  import GuideBook from "./components/layout/GuideBook.svelte";
  import GuideWalk from "./components/GuideWalk.svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { launchDocument } from "./lib/api";
  import * as api from "./lib/api";
  import { clearBackup } from "./lib/backupCopy";
  import { offerBackup, startBackups } from "./lib/backup";
  import { autosave, autosaver } from "./lib/autosave.svelte";
  import { themeVars } from "./lib/themes";

  editor.reset(newDocument(), null);
  editor.showWelcome = true;
  fonts.load();
  templates.refresh();
  void classes.restore();
  // Après une panne : la copie de secours, si l'utilisateur la reprend. Sinon, ouvert par un double-clic sur
  // un fichier .carnet : ce document, à la place de l'accueil.
  void offerBackup()
    .then((restored) => (restored ? null : launchDocument()))
    .then((path) => (path ? openPath(path, false) : undefined))
    .catch(() => {});
  onMount(() => startBackups());
  // Un document qui a un fichier s'y enregistre tout seul (Préférences > Enregistrement).
  onMount(() => autosave.start());
  // Après une mise à jour : les nouveautés, une fois, pour qui utilisait déjà Cahutin.
  if (shouldShowWhatsNew(readPref<string | null>("whatsNewSeen", null), readPref("tourDone", false))) editor.showWhatsNew = true;
  function closeWhatsNew() {
    editor.showWhatsNew = false;
    writePref("whatsNewSeen", WHATS_NEW.version);
  }
  // Dialogue « Générer pour la classe ».
  let generateOpen = $state(false);
  // Nouvelles versions : seulement si l'utilisateur l'a accepté, au plus une fois par jour.
  void updates.checkIfDue();
  // Accès pour les tests de bout en bout (tauri-driver), en développement seulement.
  if (import.meta.env.DEV) Object.assign(window, { __editor: editor, __carnet: { importPaths, importFiles, fonts, save, openPath, openExampleDoc, settings, guides, whatsNew: WHATS_NEW, api, assistant, myImages, templates, classes, classLibrary, exportPdf, exportOdg, autosave, autosaver } });

  // Préférences d'affichage : police et couleurs de l'interface, taille des poignées, taille de tout (la
  // fenêtre zoome comme Ctrl+ dans un navigateur ; la page, en « Ajuster », reste à la largeur de l'écran).
  $effect(() => {
    const root = document.documentElement;
    root.style.setProperty("--ui-font", `"${settings.uiFont}", system-ui, sans-serif`);
    root.toggleAttribute("data-contrast", settings.highContrast);
    // Le thème : ses couleurs sur la racine ; un thème sombre change aussi les contrôles et barres de défilement.
    const theme = themeVars(settings.theme, settings.highContrast);
    for (const [name, value] of theme.vars) root.style.setProperty(name, value);
    root.style.colorScheme = theme.dark ? "dark" : "light";
    root.toggleAttribute("data-dark", theme.dark);
    root.toggleAttribute("data-big-handles", settings.bigHandles);
    root.toggleAttribute("data-reduce-motion", settings.reduceMotion);
    root.toggleAttribute("data-cues", settings.doubleCues);
    root.style.setProperty("--label-delay", `${settings.labelDelay}s`);
  });
  $effect(() => {
    const scale = settings.uiScale;
    getCurrentWebview().setZoom(scale).catch(() => {});
  });

  // Visite guidée : au premier lancement, quand l'écran d'accueil se ferme.
  let welcomeWasOpen = editor.showWelcome;
  $effect(() => {
    const open = editor.showWelcome;
    if (shouldStartTour(tour.done, welcomeWasOpen, open) && tour.step === null) tour.pending = true;
    welcomeWasOpen = open;
  });
  $effect(() => {
    if (tourMayStart(tour.pending, modal.message !== null, !!editor.notice)) tour.start();
  });

  // Fermeture de la fenêtre : demander d'enregistrer s'il y a des modifications (un document qui s'enregistre tout
  // seul l'est sans question, lib/fileActions.ts confirmLeave).
  // Polices du poste lues en arrière-plan par le moteur : la liste se met à jour quand elles arrivent.
  onMount(() => void assistant.load());
  onMount(() => {
    const stop = listen("fonts-changed", () => fonts.load()).catch(() => undefined);
    return () => void stop.then((u) => u?.());
  });

  onMount(() => {
    let unlisten: (() => void) | undefined;
    try {
      const win = getCurrentWindow();
      win
        .onCloseRequested(async (event) => {
          editor.flushPending();
          // Une proposition de l'assistant ne rend pas le document « modifié » : la question vient d'abord, puis
          // la fermeture suit son cours (classe à enregistrer, document modifié).
          const hadProposal = assistant.proposal !== null;
          if (hadProposal) {
            event.preventDefault();
            if (!(await assistant.settle())) return;
          }
          // La classe s'enregistre un peu après chaque modification : ce qui attend part maintenant. Si
          // l'enregistrement échoue, on ne ferme pas sans prévenir.
          await classes.flush();
          if (classes.unsaved) {
            event.preventDefault();
            if (await ask(fr.classUnsavedClose, fr.classCloseAnyway)) await win.destroy();
            return;
          }
          // Rien à enregistrer (revenu à l'état enregistré) : la copie de secours part avec la fenêtre.
          if (!editor.dirty) {
            await clearBackup();
            if (hadProposal) await win.destroy();
            return;
          }
          event.preventDefault();
          if (await confirmLeave()) await win.destroy();
        })
        .then((u) => (unlisten = u))
        .catch(() => {});
    } catch {
      // Hors de l'application (aperçu dans un navigateur) : rien à faire.
    }
    return () => unlisten?.();
  });

  let stageWidth = $state(800);
  let stageHeight = $state(600);
  let pageEl = $state<HTMLElement>();
  let stageEl = $state<HTMLElement>();
  let contextTarget = $state<ContextTarget | null>(null);

  function openContextMenu(id: string | null, xMm: number, yMm: number, clientX: number, clientY: number, stack: string[]) {
    if (id && !editor.selection.includes(id)) editor.selection = [id];
    if (!id) editor.selection = [];
    contextTarget = { id, xMm, yMm, clientX, clientY, below: blockBelow(editor.page, stack, editor.selection, editor.inside) };
  }
  const facing = $derived(editor.spread ? facingOf(editor.pageIndex, editor.doc.pages.length) : null);

  // Ctrl+molette : zoom de la page, le point sous la souris reste en place. Écouteur non passif, pour que
  // la vue web ne zoome pas toute l'interface.
  $effect(() => {
    const stage = stageEl;
    const onWheel = async (ev: WheelEvent) => {
      const before = editor.effectiveZoom;
      if (handleWheelZoom(ev) === null || !stage) return;
      const r = stage.getBoundingClientRect();
      if (ev.clientX < r.left || ev.clientX > r.right || ev.clientY < r.top || ev.clientY > r.bottom) return;
      const x = ev.clientX - r.left;
      const y = ev.clientY - r.top;
      const sx = stage.scrollLeft + x;
      const sy = stage.scrollTop + y;
      const ratio = editor.effectiveZoom / before;
      await tick();
      stage.scrollLeft = sx * ratio - x;
      stage.scrollTop = sy * ratio - y;
    };
    window.addEventListener("wheel", onWheel, { passive: false });
    return () => window.removeEventListener("wheel", onWheel);
  });

  // « Ajuster » : la page entière est visible, marges de la scène comprises.
  $effect(() => {
    const pages = editor.spread && facingOf(editor.pageIndex, editor.doc.pages.length) ? 2 : 1;
    const byWidth = (stageWidth - 64) / mmToPx(editor.page.size.width_mm * pages, 1);
    const byHeight = (stageHeight - 64) / mmToPx(editor.page.size.height_mm, 1);
    editor.fitZoom = Math.max(0.2, Math.min(byWidth, byHeight));
  });

  /** Photo déposée : dans une zone photo, elle la remplit ; ailleurs, une nouvelle photo de 80 mm de large. */
  function dropAsset(assetId: string, elementId: string | null, xMm: number, yMm: number) {
    const meta = editor.photos.find((p) => p.meta.id === assetId)?.meta ?? editor.doc.assets[assetId];
    if (!meta) return;
    warnConsent(assetId);
    placeDropped(meta, elementId, xMm, yMm);
  }

  async function dropMyImage(name: string, elementId: string | null, xMm: number, yMm: number) {
    try {
      placeDropped(await myImages.use(name), elementId, xMm, yMm);
    } catch {
      editor.notice = fr.myImagesFailed;
    }
  }

  /** Une photo de la photothèque lâchée sur la page : elle rejoint d'abord les photos du document. */
  async function dropClassPhoto(id: string, elementId: string | null, xMm: number, yMm: number) {
    try {
      const [meta] = await classLibrary.useInDocument([id]);
      if (!meta) return;
      warnConsent(id);
      placeDropped(meta, elementId, xMm, yMm);
    } catch (e) {
      editor.notice = String(e);
    }
  }

  /** Une image lâchée sur une case photo la remplit ; ailleurs, elle est posée là, à ses proportions. */
  function placeDropped(meta: AssetMeta, elementId: string | null, xMm: number, yMm: number) {
    if (elementId) {
      editor.placePhoto(elementId, meta);
      editor.selection = [elementId];
      return;
    }
    addPhotoAt(meta, xMm, yMm);
  }

  /** Un geste du recadrage (glisser, molette, rognage) : un pas d'annulation. */
  function applyCrop(id: string, frame: Frame, crop: Crop) {
    editor.updateElement(id, (el) => (el.kind.type === "Photo" ? { ...el, frame: roundedFrame(frame), kind: { ...el.kind, crop } } : el));
  }

  function endTextEdit(id: string, content: RichText) {
    editor.editingText = null;
    const e = editor.page.elements.find((x) => x.id === id);
    if (!e || e.kind.type !== "Text") return;
    // Une police, une taille ou une couleur donnée à tout le texte pendant l'écriture devient celle du bloc,
    // dans le même pas d'annulation que le texte.
    const done = finishedText(e.kind, content);
    if (!done) return;
    const before = e.kind.content;
    const { style, content: after } = done;
    editor.updateElement(id, (el) => (el.kind.type === "Text" ? { ...el, kind: { ...el.kind, style, content: after } } : el));
    void completeTypedText(id, before, after);
  }

  // Une classe choisie ou complétée (son école…) écrit les champs de la classe qui l'attendaient.
  $effect(() => {
    const cls = classes.current?.class;
    if (cls) untrack(() => void fillWaitingClassFields(cls));
  });

  // Un clic sélectionne tout le groupe de l'élément ; l'élément modifié seul (double-clic) reste seul.
  function select(id: string | null, add: boolean) {
    if (id === null) {
      editor.selection = [];
      editor.focusedMember = null;
      editor.insideGroup = [];
      return;
    }
    if (editor.focusedMember === id) return;
    editor.focusedMember = null;
    // Dans le groupe où l'on est entré, un clic prend un de ses blocs ; hors de lui, on en sort.
    const el = editor.page.elements.find((e) => e.id === id);
    if (!el || !editor.insideGroup.every((g, i) => pathOf(el)[i] === g)) editor.insideGroup = [];
    const members = groupOf(editor.page, id, editor.insideGroup);
    if (add) {
      const all = members.every((m) => editor.selection.includes(m));
      editor.selection = all ? editor.selection.filter((s) => !members.includes(s)) : expandSelection(editor.page, [...editor.selection, ...members], editor.insideGroup);
    } else if (editor.selection.length !== members.length || !members.every((m) => editor.selection.includes(m))) {
      editor.selection = members;
    }
  }

  /** Double-clic dans un groupe : on entre d'un niveau. Le sous-groupe de l'élément est sélectionné ; s'il
   *  n'y en a plus, l'élément est modifié seul (rend vrai). */
  function focusMember(id: string): boolean {
    const el = editor.page.elements.find((e) => e.id === id);
    if (!el) return false;
    const inside = editor.inside;
    const path = pathOf(el);
    if (path.length > inside.length + 1) {
      editor.focusedMember = null;
      editor.insideGroup = path.slice(0, inside.length + 1);
      editor.selection = groupOf(editor.page, id, editor.insideGroup);
      return false;
    }
    editor.insideGroup = path;
    editor.focusedMember = id;
    editor.selection = [id];
    return true;
  }
</script>

{#snippet facingPage(index: number)}
  <button class="facing" title={fr.pageN(index + 1)} onclick={() => ((editor.pageIndex = index), (editor.selection = []))}>
    <PageView page={editor.doc.pages[index]} doc={editor.doc} zoom={editor.effectiveZoom} />
  </button>
{/snippet}

<svelte:window onkeydown={handleShortcut} onkeyup={handleKeyUp} onblur={endNudge} onpaste={handlePasteEvent} ondragover={guardWindowDrop} ondrop={guardWindowDrop} />

<!-- Pendant la visite ou le panneau Astuces, l'application est inerte : ni clic ni clavier ne l'atteignent. -->
<div class="app" inert={tour.step !== null || editor.showTips}>
  <TopBar />
  {#if editor.proposing}<ProposalBar />{:else if editor.templateMode}<TemplateBanner />{:else if hasNameFields(editor.page)}<ClassBanner ongenerate={() => (generateOpen = true)} />{/if}
  <Rail />
  {#if editor.drawer === "templates"}<TemplatesDrawer />
  {:else if editor.drawer === "images"}<ImagesDrawer />
  {:else if editor.drawer === "text"}<TextDrawer />
  {:else if editor.drawer === "fields"}<FieldsDrawer />
  {:else if editor.drawer === "shapes"}<ShapesDrawer />
  {:else if editor.drawer === "fonts"}<FontsDrawer />
  {:else if editor.drawer === "grid"}<GridDrawer />
  {:else if editor.drawer === "decor"}<DecorDrawer />
  {:else if editor.drawer === "class"}<ClassDrawer ongenerate={() => (generateOpen = true)} />
  {/if}
  <main
    class="stage"
    class:cvd={editor.cvdView && settings.cvdPreview}
    bind:this={stageEl}
    bind:clientWidth={stageWidth}
    bind:clientHeight={stageHeight}
    onpointerdown={(ev) => {
      // Clic dans la vue, hors de la page : comme un clic sur la page vide.
      if (ev.target === ev.currentTarget && !ev.shiftKey && !editor.cropping) select(null, false);
    }}
  >
    {#if facing?.side === "left"}{@render facingPage(facing.index)}{/if}
    <PageView
      page={editor.page}
      doc={editor.doc}
      zoom={editor.effectiveZoom}
      selection={editor.selection}
      cropping={editor.cropping}
      editingText={editor.editingText}
      templateMode={editor.templateMode}
      onselect={select}
      ondropasset={dropAsset}
      ondropmyimage={dropMyImage}
      ondropclassphoto={dropClassPhoto}
      ondropfiles={dropFiles}
      oncropstart={(id) => {
        editor.selection = [id];
        editor.cropping = id;
      }}
      oncrop={applyCrop}
      oncropend={() => (editor.cropping = null)}
      onedittext={(id) => {
        // Pendant une proposition de l'assistant, la page ne se modifie pas : la frappe serait perdue.
        if (editor.proposing) return;
        editor.selection = [id];
        editor.editingText = id;
      }}
      ontextdone={endTextEdit}
      oncontext={openContextMenu}
      onfocusmember={focusMember}
      ondroppicto={(code, x, y) => {
        const el = { ...pictoElement(editor.page, code, { x_mm: x, y_mm: y }, undefined, pictos.outline), locked: editor.templateMode };
        editor.updatePage((p) => ({ ...p, elements: [...p.elements, el] }));
        editor.selection = [el.id];
      }}
      bind:pageEl
    />
    {#if facing?.side === "right"}{@render facingPage(facing.index)}{/if}
    <ProposalHighlights {pageEl} {stageEl} />
    <!-- Pendant une proposition de l'assistant, la page ne se modifie pas : ni poignées ni barre d'outils. -->
    {#if !editor.proposing}
      <Transformer {pageEl} zoom={editor.effectiveZoom} />
      <GroupTransformer {pageEl} zoom={editor.effectiveZoom} onpick={select} />
      <ContextBar {pageEl} {stageEl} />
    {/if}
    <BoxSelect {pageEl} {stageEl} zoom={editor.effectiveZoom} />
    <CellBar {pageEl} {stageEl} />
  <TableBar {pageEl} {stageEl} />
    <GridDragMerge {pageEl} />
    {#if contextTarget}<ContextMenu target={contextTarget} onclose={() => (contextTarget = null)} onselect={(id) => select(id, false)} />{/if}
  </main>
  <div class="hint-slot"><Hint /></div>
  {#if assistant.open && assistant.enabled}<AssistantPanel />{:else}<Inspector />{/if}
  <BottomBar />
  {#if editor.showWelcome}<Welcome />{/if}
  {#if editor.showClassPage}<ClassPage />{/if}
  {#if editor.showPhototheque}<PhotothequePage />{/if}
  {#if editor.showCollection}<CollectionPage />{/if}
  {#if editor.showAbout}<About />{/if}
{#if editor.showPreferences}<Preferences />{/if}
  <UpdateBanner />
  {#if editor.showBackground}<BackgroundPanel />{/if}
  <Modal />
<DateDialog />
  {#if editor.blurring}<FaceBlurDialog assetId={editor.blurring} onclose={() => (editor.blurring = null)} />{/if}
  {#if generateOpen && classes.current}<GenerateDialog onclose={() => (generateOpen = false)} />{/if}
  {#if editor.fileImport}
    <div class="notice" role="status">
      <pre>{fr.importingFile(editor.fileImport.name)}</pre>
      <button onclick={cancelFileImport}>{fr.cancel}</button>
    </div>
  {:else if editor.busy}
    <div class="notice" role="status"><pre>{editor.busy}</pre></div>
  {:else if editor.notice}
    <div class="notice" role="alert">
      <pre>{editor.notice}</pre>
      {#if editor.noticeAction}
        <button onclick={() => editor.runNoticeAction()}>{editor.noticeAction.label}</button>
      {/if}
      <button onclick={() => (editor.notice = null)}>OK</button>
    </div>
  {/if}
</div>
{#if editor.showTips}<TipsPanel />{/if}
{#if editor.showWhatsNew}<WhatsNew onclose={closeWhatsNew} />{/if}
<Tour />
{#if guides.showList}<GuideBook />{/if}
<GuideWalk />
<!-- Vision daltonienne (rouge-vert, deutéranopie) : matrice de Machado, Oliveira et Fernandes (2009). -->
<svg class="filters" aria-hidden="true">
  <filter id="cvd-deuteranopia">
    <feColorMatrix type="matrix" values="0.367 0.861 -0.228 0 0  0.280 0.673 0.047 0 0  -0.012 0.043 0.969 0 0  0 0 0 1 0" />
  </filter>
</svg>
{#if editor.printing}<PrintRoot />{/if}

<style>
  :global(:root) {
    /* Les couleurs du thème Papier, avant que lib/themes.ts pose celles du thème choisi. */
    --bg: #efe9df;
    --panel: #fbf8f3;
    --line: #e2d9cb;
    --text: #2d2a26;
    --muted: #8a8278;
    --accent: #2e86de;
    --accent-strong: #c0392b;
    --surface: #ffffff;
    --on-accent: #ffffff;
    --accent-soft: #e8f1fb;
    --danger: #c0392b;
    --danger-soft: #fdecea;
    --warn-bg: #fff4e5;
    --warn-line: #f0b86e;
    --warn-text: #8a4f00;
    --ok: #2e7d32;
    --shadow: rgba(0, 0, 0, 0.15);
    --backdrop: rgba(40, 30, 20, 0.35);
    --zone-photo: #2e86de;
    --zone-text: #e67e22;
    --ui-font: "Atkinson Hyperlegible", system-ui, sans-serif;
    --title-font: "Playwrite FR Moderne", cursive;
  }
  /* Icônes Lucide (Icon.svelte) : posées sur la ligne du texte, jamais écrasées dans un bouton étroit. */
  :global(svg.icon) {
    flex: none;
    vertical-align: -0.2em;
  }
  /* Contraste élevé (ses couleurs viennent de lib/themes.ts) : des bordures plus épaisses. */
  :global(:root[data-contrast] button, :root[data-contrast] select, :root[data-contrast] input) {
    border-width: 2px;
  }
  /* Poignée de rotation à 28 px de l'élément (40 chez Moveable) : la barre d'outils au-dessus ne la couvre pas. */
  :global(.moveable-rotation) {
    height: calc(28px * var(--zoom, 1)) !important;
  }
  /* Grandes poignées : deux fois plus grosses, pour le doigt ou une souris difficile. */
  :global(:root[data-big-handles] .moveable-control) {
    width: 26px !important;
    height: 26px !important;
    margin-top: -13px !important;
    margin-left: -13px !important;
  }
  /* Animations réduites : tout passe d'un coup (les attentes, comme celle des languettes, restent). */
  :global(:root[data-reduce-motion] *),
  :global(:root[data-reduce-motion] *::before),
  :global(:root[data-reduce-motion] *::after) {
    transition-duration: 0s !important;
    animation-duration: 0s !important;
    scroll-behavior: auto !important;
  }
  /* Repères doublés : un bouton actif est aussi souligné, pas seulement coloré. */
  :global(:root[data-cues] button.on),
  :global(:root[data-cues] button[aria-pressed="true"]) {
    box-shadow: inset 0 -3px 0 currentColor;
  }
  .filters {
    position: absolute;
    width: 0;
    height: 0;
  }
  .stage.cvd :global(.page) {
    filter: url(#cvd-deuteranopia);
  }
  :global(html, body) {
    margin: 0;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    font: 14px var(--ui-font);
  }
  :global(button, select, input) {
    font: inherit;
    color: inherit;
    border: 1px solid var(--line);
    background: var(--surface);
    border-radius: 6px;
    padding: 5px 10px;
  }
  :global(button:not(:disabled)) {
    cursor: pointer;
  }
  :global(button:not(:disabled):hover) {
    border-color: var(--accent);
  }
  :global(button:disabled) {
    opacity: 0.45;
  }
  :global(h3) {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    margin: 4px 0 10px;
  }
  .app {
    display: grid;
    height: 100vh;
    grid-template-columns: auto auto 1fr auto;
    grid-template-rows: auto auto 1fr auto;
    grid-template-areas: "top top top top" "tbar tbar tbar tbar" "rail drawer stage right" "bottom bottom bottom bottom";
  }
  .notice {
    position: fixed;
    left: 50%;
    bottom: 70px;
    transform: translateX(-50%);
    max-width: 640px;
    background: var(--warn-bg);
    border: 1px solid var(--warn-line);
    border-radius: 10px;
    box-shadow: 0 6px 24px var(--shadow);
    padding: 12px 16px;
    display: flex;
    gap: 14px;
    align-items: flex-start;
    z-index: 6000;
  }
  .hint-slot {
    grid-area: stage;
    align-self: end;
    justify-self: center;
    margin-bottom: 14px;
    z-index: 30;
  }
  .notice pre {
    margin: 0;
    font: inherit;
    white-space: pre-wrap;
  }
  /* Page d'en face : visible, pas modifiable ; un clic l'ouvre. */
  .facing {
    flex: none;
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    opacity: 0.92;
  }
  .facing:hover {
    opacity: 1;
  }
  .facing :global(*) {
    pointer-events: none;
  }
  .stage {
    grid-area: stage;
    overflow: auto;
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 32px;
    min-width: 0;
    min-height: 0;
    position: relative;
  }
</style>

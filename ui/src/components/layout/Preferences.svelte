<script lang="ts">
  // Préférences de l'application : affichage (taille, police, contraste, poignées), barre d'outils, nouvelle
  // page, graphisme, export, astuces, avis de nouvelles versions. Chaque choix s'applique tout de suite et reste
  // sur le poste.
  import { canExportPdf } from "../../lib/api";
  import { editor } from "../../lib/docStore.svelte";
  import { hints } from "../../lib/hints.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { EMBEDDED_FAMILIES } from "../../lib/fontNames";
  import type { Level } from "../../lib/graphisme";
  import type { Format } from "../../lib/pageFormat";
  import { LABEL_DELAYS, UI_FONTS, UI_SCALES, settings, type LabelDelay, type UiFont, type UiScale } from "../../lib/settings.svelte";
  import { updates } from "../../lib/updates.svelte";

  const close = () => (editor.showPreferences = false);

  // Le choix de l'export n'a de sens que là où le PDF est proposé.
  let pdf = $state(false);
  canExportPdf().then((v) => (pdf = v)).catch(() => (pdf = false));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} />

<div class="backdrop" role="presentation" onclick={close}>
  <div class="box" role="dialog" aria-modal="true" aria-labelledby="prefs-title" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <h2 id="prefs-title">⚙ {fr.preferences}</h2>

    <!-- Sur un grand écran, les sections se rangent en deux colonnes ; sur un petit, en une. -->
    <div class="sections">
    <section>
      <h3>{fr.prefsDisplay}</h3>
      <div class="row">
        <span><b>{fr.prefsUiScale}</b><small>{fr.prefsUiScaleHint}</small></span>
        <div class="seg" role="radiogroup" aria-label={fr.prefsUiScale}>
          {#each UI_SCALES as scale (scale)}
            <button role="radio" aria-checked={settings.uiScale === scale} class:on={settings.uiScale === scale} onclick={() => settings.set("uiScale", scale as UiScale)}>{Math.round(scale * 100)} %</button>
          {/each}
        </div>
      </div>
      <div class="row">
        <span><b>{fr.prefsUiFont}</b><small>{fr.prefsUiFontHint}</small></span>
        <select value={settings.uiFont} style:font-family={`"${settings.uiFont}"`} onchange={(e) => settings.set("uiFont", e.currentTarget.value as UiFont)}>
          {#each UI_FONTS as font (font)}<option value={font} style:font-family={`"${font}"`}>{fr.prefsUiFontNames[font]}</option>{/each}
        </select>
      </div>
      <label class="choice">
        <input type="checkbox" checked={settings.highContrast} onchange={(e) => settings.set("highContrast", e.currentTarget.checked)} />
        <span><b>{fr.prefsContrast}</b><small>{fr.prefsContrastHint}</small></span>
      </label>
      <label class="choice">
        <input type="checkbox" checked={settings.bigHandles} onchange={(e) => settings.set("bigHandles", e.currentTarget.checked)} />
        <span><b>{fr.prefsBigHandles}</b><small>{fr.prefsBigHandlesHint}</small></span>
      </label>
      <label class="choice">
        <input type="checkbox" checked={settings.reduceMotion} onchange={(e) => settings.set("reduceMotion", e.currentTarget.checked)} />
        <span><b>{fr.prefsReduceMotion}</b><small>{fr.prefsReduceMotionHint}</small></span>
      </label>
    </section>

    <section>
      <h3>{fr.prefsColorBlind}</h3>
      <label class="choice">
        <input type="checkbox" checked={settings.colorNames} onchange={(e) => settings.set("colorNames", e.currentTarget.checked)} />
        <span><b>{fr.prefsColorNames}</b><small>{fr.prefsColorNamesHint}</small></span>
      </label>
      <label class="choice">
        <input type="checkbox" checked={settings.doubleCues} onchange={(e) => settings.set("doubleCues", e.currentTarget.checked)} />
        <span><b>{fr.prefsDoubleCues}</b><small>{fr.prefsDoubleCuesHint}</small></span>
      </label>
      <label class="choice">
        <input type="checkbox" checked={settings.cvdPalette} onchange={(e) => settings.set("cvdPalette", e.currentTarget.checked)} />
        <span><b>{fr.prefsCvdPalette}</b><small>{fr.prefsCvdPaletteHint}</small></span>
      </label>
      <label class="choice">
        <input type="checkbox" checked={settings.cvdPreview} onchange={(e) => settings.set("cvdPreview", e.currentTarget.checked)} />
        <span><b>{fr.prefsCvdPreview}</b><small>{fr.prefsCvdPreviewHint}</small></span>
      </label>
    </section>

    <section>
      <h3>{fr.prefsToolbar}</h3>
      <label class="choice">
        <input type="radio" name="toolbar" checked={!settings.compactToolbar} onchange={() => settings.setCompactToolbar(false)} />
        <span><b>{fr.prefsToolbarFull}</b><small>{fr.prefsToolbarFullHint}</small></span>
      </label>
      <label class="choice">
        <input type="radio" name="toolbar" checked={settings.compactToolbar} onchange={() => settings.setCompactToolbar(true)} />
        <span><b>{fr.prefsToolbarCompact}</b><small>{fr.prefsToolbarCompactHint}</small></span>
      </label>
      {#if settings.compactToolbar}
        <div class="row sub">
          <span><b>{fr.prefsLabelDelay}</b><small>{fr.prefsLabelDelayHint}</small></span>
          <select value={settings.labelDelay} aria-label={fr.prefsLabelDelay} onchange={(e) => settings.set("labelDelay", Number(e.currentTarget.value) as LabelDelay)}>
            {#each LABEL_DELAYS as d (d)}<option value={d}>{fr.prefsLabelDelays[d]}</option>{/each}
          </select>
        </div>
      {/if}
      <label class="choice">
        <input type="checkbox" checked={settings.moreMenu} onchange={(e) => settings.setMoreMenu(e.currentTarget.checked)} />
        <span><b>{fr.prefsMoreMenu}</b><small>{fr.prefsMoreMenuHint}</small></span>
      </label>
    </section>

    <section>
      <h3>{fr.prefsNewPage}</h3>
      <div class="row">
        <span><b>{fr.prefsPageFormat}</b><small>{fr.prefsNewPageHint}</small></span>
        <span class="controls">
          <select value={settings.pageFormat} aria-label={fr.prefsPageFormat} onchange={(e) => settings.set("pageFormat", e.currentTarget.value as Format)}>
            <option value="A5">A5</option>
            <option value="A4">A4</option>
            <option value="A3">A3</option>
          </select>
          <select value={settings.pageLandscape ? "l" : "p"} aria-label={fr.landscape} onchange={(e) => settings.set("pageLandscape", e.currentTarget.value === "l")}>
            <option value="p">{fr.portrait}</option>
            <option value="l">{fr.landscape}</option>
          </select>
        </span>
      </div>
      <div class="row">
        <span><b>{fr.prefsTextFont}</b><small>{fr.prefsTextFontHint}</small></span>
        <select value={settings.textFont} style:font-family={`"${settings.textFont}"`} onchange={(e) => settings.set("textFont", e.currentTarget.value)}>
          {#each EMBEDDED_FAMILIES as font (font)}<option value={font} style:font-family={`"${font}"`}>{font}</option>{/each}
        </select>
      </div>
    </section>

    <section>
      <h3>{fr.prefsGraphisme}</h3>
      <div class="row">
        <span><b>{fr.graphismeLevel}</b><small>{fr.prefsGraphismeHint}</small></span>
        <select value={settings.graphismeLevel} aria-label={fr.graphismeLevel} onchange={(e) => settings.set("graphismeLevel", e.currentTarget.value as Level)}>
          <option value="PS">PS</option>
          <option value="MS">MS</option>
          <option value="GS">GS</option>
        </select>
      </div>
    </section>

    {#if pdf}
      <section>
        <h3>{fr.prefsExport}</h3>
        <label class="choice">
          <input type="radio" name="export" checked={settings.preferredExport === "pdf"} onchange={() => settings.set("preferredExport", "pdf")} />
          <span><b>{fr.prefsExportPdf}</b><small>{fr.prefsExportHint}</small></span>
        </label>
        <label class="choice">
          <input type="radio" name="export" checked={settings.preferredExport === "odg"} onchange={() => settings.set("preferredExport", "odg")} />
          <span><b>{fr.prefsExportOdg}</b></span>
        </label>
      </section>
    {/if}

    <section>
      <h3>{fr.prefsHelp}</h3>
      <label class="choice">
        <input type="checkbox" checked={hints.enabled} onchange={() => hints.toggle()} />
        <span><b>{fr.hintsWhileWorking}</b></span>
      </label>
      {#if !updates.store}
        <label class="choice">
          <input type="checkbox" checked={updates.consent === true} onchange={(e) => updates.setConsent(e.currentTarget.checked)} />
          <span><b>{fr.updatesMenu}</b></span>
        </label>
      {/if}
    </section>
    </div>

    <div class="buttons"><button class="primary" onclick={close}>{fr.close}</button></div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    width: min(1000px, 94vw);
    max-height: 88vh;
    overflow-y: auto;
    box-sizing: border-box;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
  }
  .sections {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(420px, 100%), 1fr));
    gap: 4px 36px;
    align-items: start;
  }
  h2 {
    font: 700 22px var(--title-font);
    color: var(--accent-strong);
    margin: 0 0 8px;
  }
  h3 {
    margin: 14px 0 6px;
    font-size: 13px;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .choice {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 6px 0;
  }
  .choice input {
    margin-top: 3px;
  }
  .choice span {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row {
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-between;
    padding: 6px 0;
  }
  .row > span:first-child {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row b {
    font-weight: 500;
  }
  .row small {
    color: var(--muted);
  }
  .row.sub {
    padding-left: 26px;
  }
  .controls {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }
  .row select {
    flex-shrink: 0;
    max-width: 260px;
  }
  .seg {
    display: inline-flex;
    flex-shrink: 0;
  }
  .seg button {
    border-radius: 0;
    margin-left: -1px;
  }
  .seg button:first-child {
    border-radius: 6px 0 0 6px;
  }
  .seg button:last-child {
    border-radius: 0 6px 6px 0;
  }
  .seg button.on {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .choice b {
    font-weight: 500;
  }
  .choice small {
    color: var(--muted);
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    margin-top: 16px;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
    padding: 7px 18px;
  }
</style>

<script lang="ts">
  // Affichage : le thème (avec un aperçu de chaque ambiance), la taille de tout, la police des menus.
  import { fr } from "../../lib/i18n/fr";
  import { THEMES, THEME_IDS } from "../../lib/themes";
  import { UI_FONTS, UI_SCALES, settings, type UiFont, type UiScale } from "../../lib/settings.svelte";
</script>

<!-- Le thème : un aperçu de chaque ambiance (le fond, une barre, un bouton), la page blanche au milieu. -->
<div class="theme-head"><b>{fr.prefsTheme}</b><small>{fr.prefsThemeHint}</small></div>
<div class="themes" role="radiogroup" aria-label={fr.prefsTheme}>
  {#each THEME_IDS as id (id)}
    {@const c = THEMES[id].colors}
    <button role="radio" class="theme" aria-checked={settings.theme === id} class:on={settings.theme === id} title={fr.themeHints[id]} onclick={() => settings.set("theme", id)}>
      <span class="swatch" style:background={c.bg} aria-hidden="true">
        <span class="bar" style:background={c.panel} style:border-color={c.line}><span class="dot" style:background={c.accent}></span></span>
        <span class="paper"></span>
      </span>
      <span class="theme-name">{fr.themeNames[id]}</span>
    </button>
  {/each}
</div>
{#if settings.highContrast}<p class="theme-note">{fr.prefsThemeContrast}</p>{/if}
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

<style>
  .theme-head {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 0;
  }
  .theme-head b {
    font-weight: 500;
  }
  .theme-head small,
  .theme-note {
    color: var(--muted);
  }
  .theme-note {
    margin: 4px 0 0;
    font-size: 12.5px;
  }
  .themes {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .theme {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 6px;
  }
  .theme.on {
    border: 2px solid var(--accent);
    background: var(--accent-soft);
  }
  /* L'aperçu : le fond du thème, sa barre avec le point d'accent, et la page blanche. */
  .swatch {
    position: relative;
    width: 100%;
    height: 46px;
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid var(--line);
  }
  .swatch .bar {
    position: absolute;
    inset: 0 0 auto;
    height: 11px;
    border-bottom: 1px solid;
  }
  .swatch .dot {
    position: absolute;
    right: 4px;
    top: 3px;
    width: 12px;
    height: 5px;
    border-radius: 3px;
  }
  .swatch .paper {
    position: absolute;
    left: 50%;
    top: 16px;
    width: 20px;
    height: 26px;
    transform: translateX(-50%);
    background: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  }
  .theme-name {
    font-size: 13px;
  }
</style>

<script lang="ts">
  // À propos : version, licences et mentions (OpenMoji, CLDR, Lucide, polices).
  import { getVersion } from "@tauri-apps/api/app";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  let version = $state("");
  getVersion().then((v) => (version = v)).catch(() => (version = ""));
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && (editor.showAbout = false)} />

<div class="backdrop" role="presentation" onclick={() => (editor.showAbout = false)}>
  <div class="box" role="dialog" aria-label={fr.aboutTitle} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
    <img class="logo" src="/logo/cahutin.svg" alt="" />
    <h2>{fr.appName}</h2>
    {#if version}<p>{fr.aboutVersion(version)}</p>{/if}
    <p>{fr.aboutFree}</p>
    <p>{fr.aboutLicense}</p>
    <p>{fr.aboutNetwork}</p>
    <p>{fr.aboutOpenmoji}</p>
    <p>{fr.aboutCldr}</p>
    <p>{fr.aboutLucide}</p>
    <p>{fr.aboutFonts}</p>
    <button onclick={() => (editor.showAbout = false)}>{fr.close}</button>
  </div>
</div>

<style>
  .logo {
    display: block;
    width: 110px;
    height: 110px;
    margin: 0 auto 4px;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 6000;
  }
  .box {
    background: var(--panel);
    border-radius: 12px;
    padding: 20px 24px;
    max-width: 460px;
    box-shadow: 0 10px 30px var(--shadow);
  }
  h2 {
    font: 700 22px var(--title-font);
    color: var(--accent-strong);
    margin: 0 0 8px;
  }
  p {
    margin: 6px 0;
  }
</style>

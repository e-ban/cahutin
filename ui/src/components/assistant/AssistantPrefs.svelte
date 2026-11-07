<script lang="ts">
  // Section des Préférences : activer l'assistant (désactivé par défaut), dire ce qui part vers lui, régler
  // l'adresse, la clé et le modèle, essayer la connexion.
  import { assistantConfigure, assistantTest } from "../../lib/api";
  import { assistant } from "../../lib/assistant.svelte";
  import { fr } from "../../lib/i18n/fr";

  const t = fr.assistant;
  let wanted = $state(assistant.enabled);
  let url = $state(assistant.settings?.url || "http://localhost:11434/v1");
  let model = $state(assistant.settings?.model || "qwen3:4b");
  let key = $state("");
  let status = $state<{ ok: boolean; text: string } | null>(null);
  let testing = $state(false);

  /** Une adresse hors de cet ordinateur : les données partent vers un service extérieur. */
  const remote = $derived.by(() => {
    try {
      const host = new URL(url).hostname;
      return !["localhost", "127.0.0.1", "[::1]", "::1"].includes(host);
    } catch {
      return false;
    }
  });

  async function apply(enabled: boolean) {
    try {
      assistant.settings = await assistantConfigure(enabled, url, model, key || undefined);
      key = "";
      if (!enabled) assistant.turnedOff();
    } catch (e) {
      status = { ok: false, text: String(e) };
    }
  }

  async function test() {
    testing = true;
    status = null;
    try {
      await assistantTest(url, model, key || undefined);
      status = { ok: true, text: t.prefsTestOk };
    } catch (e) {
      status = { ok: false, text: String(e) };
    } finally {
      testing = false;
    }
  }

  function toggle(on: boolean) {
    wanted = on;
    status = null;
    if (!on && assistant.enabled) void apply(false);
  }
</script>

<section>
  <h3>{t.prefsTitle}</h3>
  <label class="choice">
    <input type="checkbox" checked={wanted} onchange={(e) => toggle(e.currentTarget.checked)} />
    <span><b>{t.prefsEnable}</b><small>{t.prefsEnableHint}</small></span>
  </label>
  {#if wanted}
    <div class="explain">
      <p>{t.prefsWhat}</p>
      <p>✔ {t.prefsSent}</p>
      <p>✖ {t.prefsNeverSent}</p>
      <p class="warn">{t.prefsNoNames}</p>
    </div>
    <label class="field">
      <b>{t.prefsUrl}</b>
      <input type="url" bind:value={url} placeholder="http://localhost:11434/v1" spellcheck="false" />
      <small>{t.prefsUrlHint}</small>
    </label>
    {#if remote}<p class="warn">{t.prefsRemote}</p>{/if}
    <label class="field">
      <b>{t.prefsKey}</b>
      <input type="password" bind:value={key} autocomplete="off" />
      {#if assistant.settings?.has_key}<small>{t.prefsKeySaved}</small>{/if}
    </label>
    <label class="field">
      <b>{t.prefsModel}</b>
      <input bind:value={model} placeholder="qwen3:4b" spellcheck="false" />
    </label>
    <div class="actions">
      <button onclick={test} disabled={testing || !url || !model}>{testing ? t.prefsTesting : t.prefsTest}</button>
      {#if assistant.enabled}
        <button onclick={() => apply(true)} disabled={!url || !model}>{t.prefsSave}</button>
        <button onclick={() => toggle(false)}>{t.prefsDisable}</button>
      {:else}
        <button class="primary" onclick={() => apply(true)} disabled={!url || !model}>{t.prefsActivate}</button>
      {/if}
    </div>
    {#if status}<p class:ok={status.ok} class:err={!status.ok}>{status.text}</p>{/if}
  {/if}
</section>

<style>
  h3 {
    font-size: 15px;
    margin: 14px 0 6px;
  }
  .choice {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 6px 0;
    cursor: pointer;
  }
  .choice span,
  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .choice b,
  .field b {
    font-weight: 500;
  }
  small {
    color: var(--muted);
  }
  .explain {
    background: var(--bg);
    border-radius: 8px;
    padding: 4px 12px;
    margin: 4px 0 8px;
  }
  .explain p {
    margin: 6px 0;
  }
  .field {
    padding: 6px 0;
  }
  .field input {
    width: 100%;
    box-sizing: border-box;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px 0;
  }
  .warn {
    color: var(--accent-strong);
    font-weight: 500;
  }
  .ok {
    color: #2e7d32;
  }
  .err {
    color: #c62828;
  }
  .primary {
    background: var(--accent);
    color: white;
    border-color: var(--accent);
  }
  .primary:disabled {
    opacity: 0.5;
  }
</style>

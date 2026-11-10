<script lang="ts">
  // Section des Préférences : activer l'assistant (désactivé par défaut), dire ce qui part vers lui, choisir le
  // service (Claude, OpenAI, Gemini, Ollama sur cet ordinateur, ou un autre : adresse et modèle à la main), essayer
  // la connexion. Le nom du modèle d'un service connu se change dans « Réglages avancés » : ces noms changent vite.
  // L'activation essaie d'abord la connexion : l'assistant n'apparaît que s'il répond.
  import Icon from "../Icon.svelte";
  import { assistantConfigure, assistantTest } from "../../lib/api";
  import { assistant } from "../../lib/assistant.svelte";
  import { fr } from "../../lib/i18n/fr";

  const t = fr.assistant;
  const SERVICES = [
    { id: "claude", label: t.serviceClaude, url: "https://api.anthropic.com/v1", model: "claude-haiku-5-5" },
    { id: "openai", label: t.serviceOpenai, url: "https://api.openai.com/v1", model: "gpt-6-luna" },
    { id: "gemini", label: t.serviceGemini, url: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-3.8-flash" },
    { id: "ollama", label: t.serviceOllama, url: "http://localhost:11434/v1", model: "qwen3:4b" },
    { id: "other", label: t.serviceOther, url: "", model: "" },
  ] as const;
  type Service = (typeof SERVICES)[number]["id"];

  const savedUrl = assistant.settings?.url ?? "";
  let service = $state<Service>(SERVICES.find((x) => x.url && x.url === savedUrl)?.id ?? (savedUrl ? "other" : "claude"));
  let wanted = $state(assistant.enabled);
  let url = $state(savedUrl || SERVICES[0].url);
  let model = $state(assistant.settings?.model || SERVICES[0].model);
  let key = $state("");
  let status = $state<{ ok: boolean; text: string } | null>(null);
  let working = $state(false);

  function choose(id: Service) {
    service = id;
    status = null;
    const s = SERVICES.find((x) => x.id === id)!;
    if (id !== "other") ((url = s.url), (model = s.model));
  }

  /** Une clé déjà enregistrée pour cette adresse (une autre adresse ne la reçoit jamais). */
  const keySaved = $derived(!!assistant.settings?.has_key && assistant.settings.url === url);
  /** Les services en ligne connus demandent une clé. */
  const needsKey = $derived(service === "claude" || service === "openai" || service === "gemini");
  const keyHints: Partial<Record<Service, string>> = { claude: t.serviceClaudeHint, openai: t.serviceOpenaiHint, gemini: t.serviceGeminiHint };
  const ready = $derived(!!url && !!model && (!needsKey || !!key || keySaved));

  /** Une adresse hors de cet ordinateur : les données partent vers un service extérieur. */
  const remote = $derived.by(() => {
    try {
      const host = new URL(url).hostname;
      return !["localhost", "127.0.0.1", "[::1]", "::1"].includes(host);
    } catch {
      return false;
    }
  });

  /** Essaie la connexion, puis enregistre et active : l'assistant n'apparaît que s'il répond. */
  async function activate() {
    working = true;
    status = { ok: true, text: t.prefsTesting };
    try {
      await assistantTest(url, model, key || undefined);
      const wasOn = assistant.enabled;
      assistant.settings = await assistantConfigure(true, url, model, key || undefined);
      key = "";
      status = { ok: true, text: wasOn ? t.prefsSaved : t.prefsActivated };
    } catch (e) {
      status = { ok: false, text: String(e) };
    } finally {
      working = false;
    }
  }

  async function turnOff() {
    try {
      assistant.settings = await assistantConfigure(false, url, model, undefined);
      assistant.turnedOff();
    } catch (e) {
      status = { ok: false, text: String(e) };
    }
  }

  /** Efface la clé enregistrée ; sans elle, l'assistant se désactive. */
  async function forgetKey() {
    try {
      assistant.settings = await assistantConfigure(false, url, model, "");
      assistant.turnedOff();
      status = { ok: true, text: t.prefsKeyForgotten };
    } catch (e) {
      status = { ok: false, text: String(e) };
    }
  }

  function toggle(on: boolean) {
    wanted = on;
    status = null;
    if (!on && assistant.enabled) void turnOff();
  }
</script>

<section>
  <h3>{t.prefsTitle}</h3>
  <label class="choice">
    <input type="checkbox" checked={wanted} onchange={(e) => toggle(e.currentTarget.checked)} />
    <span><b>{t.prefsEnable}</b><small>{assistant.enabled ? t.prefsEnabledHint : wanted ? t.prefsSetupHint : t.prefsEnableHint}</small></span>
  </label>
  {#if wanted}
    <div class="explain">
      <p>{t.prefsWhat}</p>
      <p><Icon name="check" /> {t.prefsSent}</p>
      <p><Icon name="x" /> {t.prefsNeverSent}</p>
      <p class="warn">{t.prefsNoNames}</p>
    </div>
    <label class="field">
      <b>{t.prefsService}</b>
      <select value={service} onchange={(e) => choose(e.currentTarget.value as Service)}>
        {#each SERVICES as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
      </select>
      {#if keyHints[service]}<small>{keyHints[service]}</small>{/if}
      {#if service === "ollama"}<small>{t.prefsUrlHint}</small>{/if}
    </label>
    {#if service === "other"}
      <label class="field">
        <b>{t.prefsUrl}</b>
        <input type="url" bind:value={url} placeholder="https://…/v1" spellcheck="false" />
      </label>
      <label class="field">
        <b>{t.prefsModel}</b>
        <input bind:value={model} spellcheck="false" />
      </label>
    {/if}
    {#if remote}<p class="warn">{t.prefsRemote}</p>{/if}
    {#if service !== "ollama"}
      <label class="field">
        <b>{needsKey ? t.prefsKeyNeeded : t.prefsKey}</b>
        <input type="password" bind:value={key} autocomplete="off" />
        {#if keySaved}<small>{t.prefsKeySaved}</small>{/if}
      </label>
      {#if keySaved}<button class="forget" onclick={forgetKey} disabled={working}>{t.prefsForgetKey}</button>{/if}
    {/if}
    {#if service !== "other"}
      <details class="advanced">
        <summary>{t.prefsAdvanced}</summary>
        <label class="field">
          <b>{t.prefsModel}</b>
          <input bind:value={model} spellcheck="false" />
          <small>{t.prefsModelHint}</small>
        </label>
      </details>
    {/if}
    <div class="actions">
      <button class="primary" onclick={activate} disabled={working || !ready}>{assistant.enabled ? t.prefsSave : t.prefsActivate}</button>
      {#if assistant.enabled}<button onclick={() => toggle(false)} disabled={working}>{t.prefsDisable}</button>{/if}
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
  .field input,
  .field select {
    width: 100%;
    box-sizing: border-box;
  }
  .forget {
    align-self: flex-start;
  }
  .advanced summary {
    cursor: pointer;
    color: var(--muted);
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

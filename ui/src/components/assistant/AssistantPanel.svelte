<script lang="ts">
  // Panneau de l'assistant, à droite à la place de l'inspecteur : la proposition en cours (Accepter /
  // Refuser), le fil des demandes avec leurs étapes cliquables, et le champ de saisie.
  import { tick } from "svelte";
  import { assistant } from "../../lib/assistant.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import type { AssistantStep } from "../../lib/types";

  const t = fr.assistant;
  let text = $state("");
  let threadEl = $state<HTMLElement>();

  async function send() {
    const message = text;
    text = "";
    const sending = assistant.send(message);
    await tick();
    threadEl?.scrollTo({ top: threadEl.scrollHeight });
    await sending;
    await tick();
    threadEl?.scrollTo({ top: threadEl.scrollHeight });
  }

  function pick(step: AssistantStep) {
    assistant.view(assistant.viewing === step ? null : step);
  }
</script>

<aside class="right" aria-label={t.title}>
  <header>
    <h2>✦ {t.title}</h2>
    <!-- Le panneau prend la place des réglages : un élément choisi, on peut y revenir d'un clic. -->
    {#if editor.selection.length && !editor.proposing}<button class="ghost settings" onclick={() => (assistant.open = false)}>⚙ {t.showSettings}</button>{/if}
    <button class="ghost" title={t.close} aria-label={t.close} onclick={() => (assistant.open = false)}>✕</button>
  </header>

  <div class="thread" bind:this={threadEl}>
    {#if assistant.thread.length === 0}<p class="empty">{t.empty}</p>{/if}
    {#each assistant.thread as entry, i (i)}
      {#if entry.kind === "user"}
        <p class="user">{entry.text}</p>
      {:else if entry.kind === "error"}
        <p class="error">{entry.text}</p>
      {:else}
        <div class="reply">
          {#if entry.text}<p>{entry.text}</p>{/if}
          {#if entry.steps.length}
            <ol>
              {#each entry.steps as step, j (j)}
                <li>
                  <button class="step" class:on={assistant.viewing === step} onclick={() => pick(step)}>
                    {#each step.summary as line, k (k)}<span>{line}</span>{/each}
                  </button>
                </li>
              {/each}
            </ol>
          {:else}
            <small>{t.noChange}</small>
          {/if}
          {#if entry.outcome === "accepted"}<small class="outcome ok">✔ {t.accepted}</small>{/if}
          {#if entry.outcome === "refused"}<small class="outcome">✖ {t.refused}</small>{/if}
          <!-- Une réponse déplacée se signale à l'auteur (règles du Microsoft Store sur l'IA générative). -->
          <button class="ghost report" title={t.reportHint} onclick={() => void assistant.report(entry)}>⚑ {t.report}</button>
        </div>
      {/if}
    {/each}
    {#if assistant.busy}
      <div class="busy">
        <span>{assistant.stopping ? t.stopping : t.working}</span>
        <button onclick={() => assistant.stop()} disabled={assistant.stopping}>{t.stop}</button>
      </div>
    {/if}
  </div>

  <form
    onsubmit={(e) => {
      e.preventDefault();
      void send();
    }}
  >
    <small>{t.selectionHint(editor.selection.length)}</small>
    <textarea
      bind:value={text}
      rows="3"
      placeholder={t.placeholder}
      disabled={assistant.busy}
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          void send();
        }
      }}
    ></textarea>
    <button class="primary" type="submit" disabled={assistant.busy || !text.trim()}>{t.send}</button>
  </form>
</aside>

<style>
  .right {
    grid-area: right;
    width: 300px;
    background: var(--panel);
    border-left: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 8px 12px 12px;
    gap: 8px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .settings {
    margin-left: auto;
  }
  h2 {
    font: 700 16px var(--title-font);
    color: var(--accent-strong);
    margin: 0;
  }
  .thread {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 0;
  }
  .empty {
    color: var(--muted);
  }
  .user {
    align-self: flex-end;
    background: var(--accent);
    color: white;
    border-radius: 10px 10px 2px 10px;
    padding: 6px 10px;
    margin: 0;
    max-width: 90%;
    white-space: pre-wrap;
  }
  .reply {
    background: var(--bg);
    border-radius: 10px 10px 10px 2px;
    padding: 6px 10px;
  }
  .reply p {
    margin: 0 0 4px;
    white-space: pre-wrap;
  }
  ol {
    margin: 4px 0;
    padding-left: 20px;
  }
  .step {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    width: 100%;
    border: 1px solid transparent;
    background: none;
    padding: 2px 4px;
    border-radius: 6px;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }
  .step:hover {
    border-color: var(--line);
  }
  .step.on {
    border-color: var(--accent);
    background: var(--panel);
  }
  .outcome {
    display: block;
    margin-top: 4px;
    color: var(--muted);
  }
  .outcome.ok {
    color: #2e7d32;
  }
  .report {
    display: block;
    margin: 4px 0 0 auto;
    font-size: 12px;
    color: var(--muted);
  }
  .error {
    color: #c62828;
    margin: 0;
  }
  .busy {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    color: var(--muted);
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  form small {
    color: var(--muted);
  }
  textarea {
    resize: vertical;
    font: inherit;
    width: 100%;
    box-sizing: border-box;
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

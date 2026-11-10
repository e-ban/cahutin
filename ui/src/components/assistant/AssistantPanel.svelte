<script lang="ts">
  // Panneau de l'assistant, à droite à la place de l'inspecteur : la proposition en cours (Accepter /
  // Refuser), le fil des demandes avec leurs étapes cliquables, et le champ de saisie.
  import Icon from "../Icon.svelte";
  import { tick } from "svelte";
  import { assistant, MAX_TEXT_CHARS } from "../../lib/assistant.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";
  import type { AssistantStep } from "../../lib/types";

  const t = fr.assistant;
  let text = $state("");
  let threadEl = $state<HTMLElement>();
  let textEl = $state<HTMLTextAreaElement>();
  /** Les réponses dont le détail du coût est déplié. */
  let costOpen = $state<Set<number>>(new Set());
  const toggleCost = (i: number) => {
    const next = new Set(costOpen);
    if (!next.delete(i)) next.add(i);
    costOpen = next;
  };
  /** La suite proposée par la dernière réponse, au-dessus du champ ; cachée une fois dans le champ. */
  const next = $derived.by(() => {
    const last = assistant.thread.at(-1);
    return !assistant.busy && last?.kind === "assistant" && last.suggestion && text.trim() !== last.suggestion ? last.suggestion : null;
  });

  async function send() {
    const message = text;
    // Un message trop long est refusé : il reste dans le champ, pour le raccourcir.
    if ([...message.trim()].length <= MAX_TEXT_CHARS) text = "";
    const sending = assistant.send(message);
    await tick();
    threadEl?.scrollTo({ top: threadEl.scrollHeight });
    await sending;
    await tick();
    threadEl?.scrollTo({ top: threadEl.scrollHeight });
  }

  async function retry() {
    await assistant.retry();
    await tick();
    threadEl?.scrollTo({ top: threadEl.scrollHeight });
  }

  /** Une idée de demande, ou la suite proposée par l'assistant, remplit le champ, à envoyer telle quelle ou à compléter. */
  function suggest(idea: string) {
    text = idea;
    textEl?.focus();
  }

  function pick(step: AssistantStep) {
    assistant.view(assistant.viewing === step ? null : step);
  }
</script>

<aside class="right" aria-label={t.title}>
  <header>
    <h2><Icon name="assistant" /> {t.title}</h2>
    <!-- Le panneau prend la place des réglages : un élément choisi, on peut y revenir d'un clic. -->
    {#if editor.selection.length && !editor.proposing}<button class="ghost settings" onclick={() => (assistant.open = false)}><Icon name="settings" /> {t.showSettings}</button>{/if}
    <button class="ghost" title={t.close} aria-label={t.close} onclick={() => (assistant.open = false)}><Icon name="close" /></button>
  </header>

  <div class="thread" bind:this={threadEl}>
    {#if assistant.thread.length === 0}
      <div class="empty">
        <p>{t.empty}</p>
        <ul class="ideas">
          {#each t.ideas as idea (idea)}<li><button onclick={() => suggest(idea)}>{idea}</button></li>{/each}
        </ul>
      </div>
    {/if}
    {#each assistant.thread as entry, i (i)}
      {#if entry.kind === "user"}
        <p class="user">{entry.text}</p>
      {:else if entry.kind === "error"}
        <p class="error">{entry.text}</p>
        {#if i === assistant.thread.length - 1 && assistant.retryable && !assistant.busy}<button class="retry" onclick={() => void retry()}><Icon name="refresh" /> {t.retry}</button>{/if}
      {:else}
        <div class="reply">
          {#if entry.text}<p>{entry.text}</p>{/if}
          {#if entry.steps.length}
            <ol>
              {#each entry.steps as step, j (j)}
                <li>
                  {#if entry.undone?.includes(step)}
                    <span class="step undone">
                      {#each step.summary as line, k (k)}<s>{line}</s>{/each}
                      <small>{t.undone}</small>
                      {#if assistant.redoable === step && !assistant.busy}
                        <button class="ghost undo" title={t.redoHint} onclick={() => assistant.restore()}><Icon name="redo" /> {t.redo}</button>
                      {/if}
                    </span>
                  {:else}
                    <div class="row">
                      <button class="step" class:on={assistant.viewing === step} onclick={() => pick(step)}>
                        {#each step.summary as line, k (k)}<span>{line}</span>{/each}
                      </button>
                      {#if entry.outcome === "pending" && !assistant.busy}
                        <button class="ghost undo" title={t.undoHint} onclick={() => void assistant.undo(step)}><Icon name="undo" /> {t.undo}</button>
                      {/if}
                    </div>
                  {/if}
                </li>
              {/each}
            </ol>
          {:else}
            <small>{t.noChange}</small>
          {/if}
          {#if entry.outcome === "accepted"}<small class="outcome ok"><Icon name="check" size={14} /> {t.accepted}</small>{/if}
          {#if entry.outcome === "refused"}<small class="outcome"><Icon name="x" size={14} /> {t.refused}</small>{/if}
          <!-- Le coût à gauche, « Signaler » à droite, sur une même ligne ; le détail du coût se déplie dessous. -->
          <div class="foot">
            {#if entry.cost}
              {@const c = entry.cost}
              <button class="ghost cost" title={t.costHint} aria-label="{t.cost} : {t.costLevels[c.level - 1]}" aria-expanded={costOpen.has(i)} onclick={() => toggleCost(i)}>
                {t.cost} <span class="dots">{"●".repeat(c.level)}{"○".repeat(3 - c.level)}</span> <Icon name={costOpen.has(i) ? "chevronDown" : "chevronRight"} size={14} />
              </button>
            {/if}
            <!-- Une réponse déplacée se signale à l'auteur (règles du Microsoft Store sur l'IA générative). -->
            <button class="ghost report" title={t.reportHint} onclick={() => void assistant.report(entry)}><Icon name="flag" /> {t.report}</button>
          </div>
          {#if entry.cost && costOpen.has(i)}
            {@const c = entry.cost}
            <small class="cost-detail">{t.costLevels[c.level - 1]} : {t.costDetail(c.rounds, c.sent, c.sent ? Math.round((c.cached / c.sent) * 100) : 0, c.received)}</small>
          {/if}
        </div>
      {/if}
    {/each}
    {#if assistant.thread.length && !assistant.busy}
      <button class="ghost restart" title={t.restartHint} onclick={() => void assistant.restart()}><Icon name="restart" /> {t.restart}</button>
    {/if}
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
    {#if next}<button type="button" class="next" title={t.nextHint} onclick={() => suggest(next)}><Icon name="arrowRight" size={14} /> {next}</button>{/if}
    <textarea
      bind:this={textEl}
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
  .empty p {
    margin: 0 0 6px;
  }
  .ideas {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .ideas button {
    width: 100%;
    text-align: left;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .retry {
    align-self: flex-start;
  }
  .next {
    width: 100%;
    text-align: left;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .restart {
    align-self: center;
    font-size: 12px;
    color: var(--muted);
  }
  .user {
    align-self: flex-end;
    background: var(--accent);
    color: var(--on-accent);
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
  .row {
    display: flex;
    align-items: flex-start;
    gap: 4px;
  }
  .undo {
    flex: none;
    font-size: 12px;
    color: var(--muted);
  }
  .undone {
    color: var(--muted);
  }
  .undone small {
    font-style: italic;
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
    color: var(--ok);
  }
  .foot {
    display: flex;
    align-items: center;
    margin-top: 4px;
  }
  .cost {
    font-size: 12px;
    color: var(--muted);
  }
  .dots {
    letter-spacing: 1px;
  }
  .cost-detail {
    display: block;
    color: var(--muted);
  }
  .report {
    margin-left: auto;
    font-size: 12px;
    color: var(--muted);
  }
  .error {
    color: var(--danger);
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
    color: var(--on-accent);
    border-color: var(--accent);
  }
  .primary:disabled {
    opacity: 0.5;
  }
</style>

<script lang="ts">
  // La liste des champs, avec ce que chacun devient : dans le tiroir Champs (poser ; `only` : une seule famille par
  // section) et sous le bouton « Champ » de la barre d'un texte (ajouter, ou enlever un champ coché). `current` :
  // les champs du texte choisi, cochés.
  import { onMount } from "svelte";
  import { autoFieldExamples } from "../lib/api";
  import { CLASS_INFO_FIELDS, DATE_FIELDS, FIELDS, fieldLabel, letterKey, monthKey } from "../lib/classFields";
  import { classes } from "../lib/classStore.svelte";
  import { fr } from "../lib/i18n/fr";

  let { onpick, current = [], only }: { onpick: (key: string) => void; current?: string[]; only?: "student" | "date" | "class" } = $props();
  const groups = $derived([
    { title: fr.fieldsDate, list: DATE_FIELDS, show: !only || only === "date" },
    { title: fr.fieldsClassInfo, list: CLASS_INFO_FIELDS, show: !only || only === "class" },
  ]);

  let letter = $state(1);
  let month = $state(new Date().getMonth() + 1);
  const MONTHS = Array.from({ length: 12 }, (_, i) => i + 1);

  // Ce que donne chaque champ de la date et de la classe aujourd'hui, avec la classe choisie.
  let examples = $state<Record<string, string>>({});
  onMount(() => {
    autoFieldExamples(classes.current?.class ?? null)
      .then((list) => (examples = Object.fromEntries(list)))
      .catch(() => {});
  });
</script>

<div class="fields">
  {#if !only || only === "student"}
  {#each FIELDS as f (f.key)}
    <button class="field" class:on={current.includes(f.key)} aria-pressed={current.includes(f.key)} title={current.includes(f.key) ? fr.fieldRemove : undefined} onclick={() => onpick(f.key)}>
      <span class="name">{current.includes(f.key) ? "✓ " : ""}{f.label}</span>
      <small>{f.hint}</small>
    </button>
  {/each}
  <div class="more">
    <span class="name">{fr.fieldLetter}</span>
    <small>{fr.fieldLetterHint}</small>
    <div class="row">
      <label>{fr.fieldLetterNumber} <input type="number" min="1" max="12" bind:value={letter} /></label>
      <button onclick={() => onpick(letterKey(Math.min(12, Math.max(1, Math.round(letter) || 1)), true))}>{fr.fieldLetterCapital}</button>
      <button onclick={() => onpick(letterKey(Math.min(12, Math.max(1, Math.round(letter) || 1)), false))}>{fr.fieldLetterAsWritten}</button>
    </div>
  </div>
  <div class="more">
    <span class="name">{fr.fieldMonth}</span>
    <small>{fr.fieldMonthHint}</small>
    <div class="row">
      <select bind:value={month} aria-label={fr.fieldMonth}>
        {#each MONTHS as m (m)}<option value={m}>{fieldLabel(monthKey(m))?.replace(/^Anniversaires d(e |')/, "")}</option>{/each}
      </select>
      <button onclick={() => onpick(monthKey(month))}>{fr.fieldPlace}</button>
    </div>
  </div>
  {/if}
  {#each groups.filter((g) => g.show) as g (g.title)}
    {#if !only}<h4>{g.title}</h4>{/if}
    {#each g.list as f (f.key)}
      <button class="field" class:on={current.includes(f.key)} aria-pressed={current.includes(f.key)} title={current.includes(f.key) ? fr.fieldRemove : undefined} onclick={() => onpick(f.key)}>
        <span class="name">{current.includes(f.key) ? "✓ " : ""}{f.label}</span>
        <small>{examples[f.key] ? `« ${examples[f.key]} »` : f.hint}</small>
      </button>
    {/each}
  {/each}
</div>

<style>
  .fields {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .field,
  .more {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    text-align: left;
    gap: 1px;
    padding: 5px 8px;
  }
  .more {
    border-top: 1px solid var(--line);
    margin-top: 2px;
  }
  .field.on {
    border-color: var(--accent);
  }
  .name {
    font-weight: 600;
  }
  h4 {
    margin: 8px 0 0;
    padding-top: 6px;
    border-top: 1px solid var(--line);
    font-size: 0.85em;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  small {
    color: var(--muted);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
  }
  .row input {
    width: 3.5em;
  }
</style>

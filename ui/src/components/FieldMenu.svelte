<script lang="ts">
  // La liste des champs de la classe, avec ce que chacun devient : dans le tiroir Texte (poser) et sous le bouton
  // « Champ » de la barre d'un texte (ajouter ou changer). `current` : les champs du texte choisi, cochés.
  import { FIELDS, fieldLabel, letterKey, monthKey } from "../lib/classFields";
  import { fr } from "../lib/i18n/fr";

  let { onpick, current = [] }: { onpick: (key: string) => void; current?: string[] } = $props();

  let letter = $state(1);
  let month = $state(new Date().getMonth() + 1);
  const MONTHS = Array.from({ length: 12 }, (_, i) => i + 1);
</script>

<div class="fields">
  {#each FIELDS as f (f.key)}
    <button class="field" class:on={current.includes(f.key)} aria-pressed={current.includes(f.key)} onclick={() => onpick(f.key)}>
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

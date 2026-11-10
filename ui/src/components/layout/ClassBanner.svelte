<script lang="ts">
  // Une page qui attend des prénoms (modèles Étiquettes, {prenom}) : on propose de la remplir avec la classe,
  // là où le besoin apparaît. Sans classe (ou classe vide), le bandeau ouvre la page « Ma classe ».
  import Icon from "../Icon.svelte";
  import { classes } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  let { ongenerate }: { ongenerate: () => void } = $props();

  function go() {
    if (classes.current?.class.students.length) ongenerate();
    else editor.showClassPage = true;
  }
</script>

<div class="banner">
  <span class="text"><Icon name="class" /> {fr.classBanner}</span>
  <span class="spacer"></span>
  <button class="primary" data-guide="class-fill" onclick={go}>{classes.current?.class.students.length ? fr.classBannerFill(classes.current.class.name) : fr.classBannerStart}</button>
</div>

<style>
  .banner {
    grid-area: tbar;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    background: var(--accent-soft);
    border-bottom: 1px solid var(--line);
    color: var(--text);
  }
  .spacer {
    flex: 1;
  }
  button {
    white-space: nowrap;
  }
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
  }
</style>

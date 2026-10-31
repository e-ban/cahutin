<script lang="ts">
  // Une page qui attend des prénoms (modèles Étiquettes, {prénom}) : on propose de la remplir avec la classe,
  // là où le besoin apparaît. Sans classe, le bandeau mène à la liste à coller.
  import { classes } from "../../lib/classStore.svelte";
  import { editor } from "../../lib/docStore.svelte";
  import { fr } from "../../lib/i18n/fr";

  let { ongenerate }: { ongenerate: () => void } = $props();

  function go() {
    if (classes.current?.class.students.length) ongenerate();
    else editor.drawer = "class";
  }
</script>

<div class="banner">
  <span class="text">🧒 {fr.classBanner}</span>
  <span class="spacer"></span>
  <button class="primary" onclick={go}>{classes.current?.class.students.length ? fr.classBannerFill(classes.current.class.name) : fr.classBannerStart}</button>
</div>

<style>
  .banner {
    grid-area: tbar;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px 12px;
    background: #eaf2fb;
    border-bottom: 1px solid #c9dcf2;
    color: #1f3a5f;
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
    color: white;
    font-weight: 600;
  }
</style>

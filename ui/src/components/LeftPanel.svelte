<script lang="ts">
  import { fr } from "../lib/i18n/fr";
</script>

<aside class="left">
  <section>
    <h3>{fr.photos}</h3>
  </section>
</aside>

<style>
  .left {
    grid-area: left;
    background: var(--panel);
    border-right: 1px solid var(--line);
    overflow-y: auto;
    padding: 12px;
  }
</style>

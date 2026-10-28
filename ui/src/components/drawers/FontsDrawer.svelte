<script lang="ts">
  import Drawer from "../layout/Drawer.svelte";
  import { fonts } from "../../lib/fonts.svelte";
  import { fr } from "../../lib/i18n/fr";
</script>

<Drawer title={fr.fonts}>
  {#each fonts.families as f (f.name)}
    <div class="font">
      <span style:font-family={`"${f.name}"`}>{f.name}</span>
      {#if f.kind}<small>{f.kind}</small>{/if}
    </div>
  {/each}
</Drawer>

<style>
  .font {
    background: white;
    border: 1px solid var(--line);
    border-radius: 7px;
    padding: 6px 9px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .font span {
    font-size: 16px;
  }
  small {
    font-size: 10px;
    border-radius: 8px;
    padding: 1px 6px;
    background: #eee;
    color: #666;
  }
</style>

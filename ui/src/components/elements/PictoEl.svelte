<script lang="ts">
  // Image de la banque OpenMoji, en couleur ou en contour à colorier : le même SVG qu'à l'export. Code inconnu :
  // un carré gris.
  import { pictoUrl } from "../../lib/pictos.svelte";

  let { code, outline = false }: { code: string; outline?: boolean } = $props();
  let broken = $state(false);
  $effect(() => {
    void code;
    void outline;
    broken = false;
  });
</script>

{#if broken}
  <div class="missing" aria-label="Image introuvable"></div>
{:else}
  <img src={pictoUrl(code, outline)} alt="" draggable="false" onerror={() => (broken = true)} />
{/if}

<style>
  img,
  .missing {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .missing {
    background: #e5e5e5;
  }
</style>

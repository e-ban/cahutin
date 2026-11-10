<script lang="ts">
  // Une icône de l'interface, dessinée au trait dans la couleur du texte. `size` en pixels ; décorative (cachée aux
  // lecteurs d'écran) : le nom du bouton porte le sens.
  import { ICONS, type IconName } from "../lib/icons";

  let { name, size = 16 }: { name: IconName; size?: number } = $props();
  const Glyph = $derived(ICONS[name]);
</script>

<Glyph {size} strokeWidth={1.75} class="icon" aria-hidden="true" />

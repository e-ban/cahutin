<script lang="ts">
  // Tiroir Champs : les champs de l'élève (remplis pour chaque enfant), de la date et de la classe. Un clic pose un
  // texte qui contient le champ, ou l'insère au curseur pendant l'écriture.
  import Drawer from "../layout/Drawer.svelte";
  import { fr } from "../../lib/i18n/fr";
  import { placeField } from "../../lib/fieldPlacing.svelte";
  import FieldMenu from "../FieldMenu.svelte";
  import InspectorSection from "../layout/InspectorSection.svelte";

  function addField(key: string) {
    void placeField(key, null);
  }
</script>

<Drawer title={fr.fieldsDrawer}>
  <p class="hint">{fr.fieldsDrawerHint}</p>
  <InspectorSection id="text-fields" title={fr.fieldsTitle} hint={fr.sectionHints.textFields}>
    <p class="hint">{fr.fieldsHint}</p>
    <FieldMenu only="student" onpick={addField} />
  </InspectorSection>
  <InspectorSection id="text-date" title={fr.fieldsDate} hint={fr.sectionHints.textDate} open={false}>
    <p class="hint">{fr.fieldsDateHint}</p>
    <FieldMenu only="date" onpick={addField} />
  </InspectorSection>
  <InspectorSection id="text-class" title={fr.fieldsClassInfo} hint={fr.sectionHints.textClass} open={false}>
    <p class="hint">{fr.fieldsClassHint}</p>
    <FieldMenu only="class" onpick={addField} />
  </InspectorSection>
</Drawer>

<style>
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 0.9em;
  }
</style>

<script lang="ts">
  // Une photo de la photothèque en grand, pour voir qui est dessus : les prénoms à côté, à cocher, les
  // flèches pour passer à la suivante. La liste est celle du moment de l'ouverture : une photo étiquetée
  // reste dans le défilement même si elle quitte le filtre « À trier ».
  import Icon from "../Icon.svelte";
  import { classPhotoUrl } from "../../lib/api";
  import { fr } from "../../lib/i18n/fr";
  import { dayOf, photoVersion, withoutConsent } from "../../lib/phototheque";
  import { classLibrary } from "../../lib/phototheque.svelte";
  import type { Student } from "../../lib/types";

  let { ids, start, students, names, onclose, onerror }: { ids: string[]; start: number; students: Student[]; names: Map<string, string>; onclose: () => void; onerror: (e: unknown) => void } = $props();

  // La position de départ ne compte qu'à l'ouverture ; ensuite les flèches la changent.
  // svelte-ignore state_referenced_locally
  let i = $state(start);
  const photo = $derived(classLibrary.photos.find((p) => p.id === ids[i]));
  const missing = $derived(photo ? withoutConsent(photo, students) : []);

  const go = (d: number) => (i = Math.min(ids.length - 1, Math.max(0, i + d)));

  async function toggle(studentId: string) {
    if (!photo) return;
    const on = photo.students.includes(studentId);
    try {
      await classLibrary.tag([photo.id], on ? { remove_students: [studentId] } : { add_students: [studentId] });
    } catch (e) {
      onerror(e);
    }
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
    else if (e.key === "ArrowRight" || e.key === "PageDown") go(1);
    else if (e.key === "ArrowLeft" || e.key === "PageUp") go(-1);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="viewer" role="dialog" aria-modal="true" aria-label={fr.libraryViewer}>
  <div class="image">
    {#if photo}<img src={classPhotoUrl(photo.id, false, photoVersion(photo))} alt="" />{/if}
    <button class="nav prev" disabled={i === 0} aria-label={fr.libraryPrev} onclick={() => go(-1)}><Icon name="chevronLeft" size={32} /></button>
    <button class="nav next" disabled={i === ids.length - 1} aria-label={fr.libraryNext} onclick={() => go(1)}><Icon name="chevronRight" size={32} /></button>
  </div>
  <aside>
    <div class="head">
      <span>{fr.libraryPosition(i + 1, ids.length)}{#if photo}{` · ${fr.libraryDay(dayOf(photo))}`}{/if}</span>
      <button onclick={onclose}><Icon name="close" /> {fr.close}</button>
    </div>
    <p class="hint">{fr.libraryWhoOne}</p>
    {#if photo}
      <div class="kids">
        {#each students as s (s.id)}
          {@const on = photo.students.includes(s.id)}
          <button class="kid" class:on aria-pressed={on} onclick={() => toggle(s.id)}>{names.get(s.id)}{#if !s.photo_allowed}<span title={fr.libraryNoConsentHint}> <Icon name="ban" size={14} /></span>{/if}</button>
        {/each}
      </div>
      {#if missing.length}<p class="warn">{fr.libraryNoConsent(missing.map((s) => names.get(s.id) ?? s.first_name))}</p>{/if}
      {#if photo.tags.length}<p class="hint">{photo.tags.join(" · ")}</p>{/if}
    {/if}
  </aside>
</div>

<style>
  .viewer {
    position: fixed;
    inset: 0;
    z-index: 5200;
    display: grid;
    grid-template-columns: 1fr 300px;
    background: rgba(20, 20, 20, 0.92);
  }
  .image {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    padding: 16px;
  }
  .image img {
    max-width: 100%;
    max-height: calc(100vh - 32px);
    object-fit: contain;
  }
  .nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    display: grid;
    place-items: center;
    padding: 10px;
    background: color-mix(in srgb, var(--surface) 85%, transparent);
    border-radius: 50%;
  }
  .nav:disabled {
    opacity: 0.3;
  }
  .prev {
    left: 16px;
  }
  .next {
    right: 16px;
  }
  aside {
    background: var(--panel);
    padding: 14px;
    overflow: auto;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .kids {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .kid {
    border-radius: 14px;
    padding: 2px 10px;
    font-size: 13px;
  }
  .kid.on {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .hint {
    color: var(--muted);
    font-size: 13px;
  }
  .warn {
    color: var(--danger);
    font-size: 13px;
  }
</style>

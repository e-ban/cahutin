// La barre du haut se resserre par paliers quand la place manque (petite fenêtre, grande taille d'interface) :
// rien n'est coupé ni caché, chaque palier ne fait que raccourcir l'écriture de quelques commandes. Le palier est
// choisi en mesurant la barre (components/layout/TopBar.svelte) : le premier où tout tient, en partant du premier.

/** Ce que fait chaque palier, cumulé avec les précédents (le champ du titre a déjà rétréci avant le palier 1). */
export const TOP_BAR_STEPS = [
  "gaps", // les écarts entre les commandes se réduisent
  "appName", // « Cahutin » à côté du logo s'efface (le logo reste)
  "toolIcons", // « Préférences », « Modèle » et « Assistant » en icône seule
  "shortOutput", // « Imprimer / Exporter » devient « Imprimer »
  "tight", // écarts, marges et champ du titre au plus juste
  "outputIcon", // puis « Imprimer » en icône seule
  "autoBox", // la case « Automatique » sans son mot
  "saveIcon", // en tout dernier recours, « Enregistrer » en icône seule (son point reste)
] as const;
export type TopBarStep = (typeof TOP_BAR_STEPS)[number];

/** Le dernier palier : au-delà, la barre ne se resserre plus. */
export const TOP_BAR_MAX = TOP_BAR_STEPS.length;

/** Un palier fait-il ce pas ? Palier 0 : la barre entière, comme quand la place suffit. */
export function topBarHas(level: number, step: TopBarStep): boolean {
  return TOP_BAR_STEPS.indexOf(step) < level;
}

/** Le premier palier où tout tient (`fits` met la barre au palier demandé et la mesure), ou le dernier. */
export function firstFitting(fits: (level: number) => boolean, max = TOP_BAR_MAX): number {
  for (let level = 0; level < max; level++) if (fits(level)) return level;
  return max;
}

# Images : vue agrandie du tiroir (2026-11-12)

Mesure de l'étape 3 (vue agrandie commune), mêmes machine, limites (2 cœurs, 4 Go) et photos générées que la
référence `docs/mesures/images-2026-11-12-reference.md`. Passage Timonier R-12, lancé sur `bb6c06b` ; les
corrections de la relecture (`db3d8cc`) ne touchent pas à l'affichage mesuré ici.

| Scène | Photos | Temps (ms) | Plus long gel (ms) | Mémoire de pointe cumulée (Mo) | Résultat |
|---|---|---|---|---|---|
| tiroir-ouvrir | 100 | 19 | 17 | 1639 | ok |
| tiroir-ouvrir | 300 | 42 | 40 | 2261 | ok |
| tiroir-ouvrir | 500 | 56 | 54 | 2817 | ok |
| agrandi-ouvrir | 500 | 83 | 40 | 2817 | ok |

## Lecture

- **Vue agrandie** : 83 ms pour s'ouvrir avec 500 photos, loin du seuil de 1 000 ms. Les vignettes sont déjà en
  cache (le tiroir les a chargées juste avant) : la mesure dit surtout le temps de mise en page.
- **Tiroir** : inchangé par rapport à l'étape 2 (56 ms contre 73 ms à 500 photos, dans la variation d'un
  passage à l'autre).

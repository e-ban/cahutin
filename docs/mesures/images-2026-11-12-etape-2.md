# Images : tiroir Photos après l'étape 2 (2026-11-12)

Mesures du tiroir Photos après l'étape 2 (barre fixe, photos par jour, numéro de page, « Non placées » par
défaut), comparées à la référence `docs/mesures/images-2026-11-12-reference.md`. Même machine, mêmes limites
(2 cœurs, 4 Go), mêmes photos générées.

- **Commit mesuré** : `4083937` ; passages Timonier R-9 (import, ouverture, défilement) et R-11 (filtre, avec
  le critère corrigé ci-dessous).

| Scène | Photos | Avant (ms) | Après (ms) | Gel avant (ms) | Gel après (ms) | Mémoire avant (Mo) | Mémoire après (Mo) |
|---|---|---|---|---|---|---|---|
| tiroir-import | 100 | 71584 | 71339 | 51 | 75 | 1623 | 1753 |
| tiroir-ouvrir | 100 | 22 | 24 | 18 | 21 | 1623 | 1753 |
| tiroir-filtre | 100 | 54 | 20 | 28 | 16 | 1623 | 1782 |
| tiroir-import | 300 | 143187 | 142559 | 28 | **112** | 1811 | 2866 |
| tiroir-ouvrir | 300 | 51 | 47 | 43 | 44 | 1811 | 2866 |
| tiroir-filtre | 300 | 65 | 17 | 32 | 17 | 1811 | 2404 |
| tiroir-import | 500 | 143530 | 143130 | 40 | **114** | 2117 | 2866 |
| tiroir-ouvrir | 500 | 78 | 73 | 66 | 70 | 2117 | 2866 |
| tiroir-filtre | 500 | 57 | 17 | 31 | 16 | 2117 | 2484 |
| tiroir-defiler | 500 | 826 | 859 | 18 | 18 | 2117 | 2866 |

## Lecture

- **Affichage** : inchangé. Ouvrir le tiroir reste sous 80 ms et le défilement sans gel.
- **Filtre par dossier** : 17 à 20 ms, une seule image. Ce n'est pas un gain de vitesse : avec les photos
  rangées par jour, chaque photo passe en haut de la liste pendant l'import et ses vignettes sont déjà chargées
  quand on filtre. Le banc mesure donc ici le temps de réaffichage. Son critère a changé en conséquence (prêt
  quand la liste ne montre plus que le dossier) : les deux colonnes ne mesurent pas tout à fait la même chose.
- **Gel pendant l'import** : 112 et 114 ms à 300 et 500 photos, au-dessus du seuil de 100 ms. La référence
  avait vu jusqu'à 446 ms sur d'autres passages pour le même lot : ce gel varie beaucoup, un seul passage ne
  dit pas si l'étape l'a aggravé. Une cause plausible à vérifier : chaque photo importée, plus récente, se
  range en haut et fait redessiner la liste.
- **Mémoire** : plus haute (2,4 à 2,9 Go contre 1,8 à 2,1 Go). À surveiller plutôt qu'à conclure : la mesure
  compte aussi le cache des fichiers lus par l'application, qui varie d'un passage à l'autre (deux passages de
  cette étape donnent 2,9 et 2,4 Go à 300 photos).

# Images : mesures de référence avant refonte (2026-11-12)

Référence de l'étape 1 de la refonte de la gestion des images
(`docs/superpowers/specs/2026-11-12-gestion-des-images-design.md`), avant les étapes 2 à 4.

- **Commit mesuré** : `ae21742` (application en version de développement, bibliothèques d'images optimisées).
- **Machine** : AMD Ryzen 7 9800X3D, 30 Go, disque NVMe, btrfs, Linux 7.3 (CachyOS).
- **Limites imposées à l'application** : 2 cœurs (`taskset -c 0,1`), 4 Go de mémoire sans échange
  (`systemd-run -p MemoryMax=4G`). Le disque dur classique **n'est pas simulé** : les lectures de photos sont
  ici bien plus rapides que sur un ordinateur d'école.
- **Photos** : générées par `scripts/mesures/photos.sh`, 4000 x 3000 px, 3,1 à 5,5 Mo, une sur vingt avec date
  EXIF.
- **Lancement** : `timonier test --junit <sortie>/mesures.xml -- scripts/mesures/mesurer.sh <sortie>`
  (passage Timonier R-5).
- **Seuils** : moins de 1 000 ms jusqu'au premier écran utile ; aucun gel de l'interface au-delà de 100 ms
  pendant un import.

| Scène | Photos | Temps (ms) | Plus long gel (ms) | Mémoire de pointe (Mo) | Résultat |
|---|---|---|---|---|---|
| tiroir-import | 100 | 71535 | 107 | 1117 | gel de 107 ms > 100 ms |
| tiroir-ouvrir | 100 | 22 | 18 | 1117 | ok |
| tiroir-filtre | 100 | 17 | 17 | 1117 | ok |
| tiroir-import | 300 | 143127 | 32 | 1239 | ok |
| tiroir-ouvrir | 300 | 52 | 44 | 1239 | ok |
| tiroir-filtre | 300 | 17 | 17 | 1239 | ok |
| tiroir-import | 500 | 143332 | 28 | 1328 | ok |
| tiroir-ouvrir | 500 | 74 | 62 | 1328 | ok |
| tiroir-filtre | 500 | 17 | 17 | 1328 | ok |
| tiroir-defiler | 500 | 813 | 17 | 1328 | ok |
| phototheque-lot | 1000 | 36606 | 57 | 1438 | ok |
| phototheque-ouvrir | 1000 | 80 | 79 | 1438 | ok |
| phototheque-enfant | 1000 | 17 | 17 | 1438 | ok |

`tiroir-import` : les tailles sont cumulées (100, puis 200 de plus, puis 200 de plus), le temps est celui du
lot ajouté. `phototheque-lot` : une sortie de 50 photos ajoutée à une photothèque qui en compte déjà 950 ; on
n'ajoute jamais des milliers de photos d'un coup.

## Lecture

- **Au-delà du seuil** : seul le gel pendant l'import dans le tiroir. Il varie beaucoup d'un passage à
  l'autre pour le même lot de 100 photos : 446 ms, 205 ms, puis 107 ms sur trois passages. Les deux premiers
  passages ont aussi dépassé 100 ms à 300 photos (379 et 221 ms). L'import gèle donc l'interface par
  à-coups ; il faudra plusieurs passages pour juger une amélioration.
- **Le plus lent** : l'import lui-même, environ 0,7 s par photo dans le tiroir et dans la photothèque sur
  2 cœurs. Hors seuil (il ne bloque pas l'écran), mais c'est l'attente que l'enseignant ressent. Dans un
  passage précédent, ajouter 1 000 photos d'un coup à la photothèque a pris 724 s et gelé l'interface
  767 ms.
- **Pour les étapes 2 à 4** : l'affichage tient déjà le seuil à cette échelle (ouvrir, filtrer, défiler,
  tous sous 100 ms), donc l'étape 2 peut changer la présentation sans craindre la vitesse ; l'enjeu est le gel
  pendant l'import. Les photothèques de 5 000 et 10 000 photos seront mesurées à l'étape 4, quand les dossiers
  suivis éviteront les copies (aujourd'hui, en remplir une de 10 000 prendrait deux heures).

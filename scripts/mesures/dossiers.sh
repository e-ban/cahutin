#!/usr/bin/env bash
# Les dossiers à suivre pour les mesures de la photothèque : pour chaque n, <destination>/<n>/ contient les n
# premières photos générées (sortie-01, sortie-02, …), 50 par dossier. Le parcours ne suit pas les sous-dossiers
# qui sont des liens : ce sont de vraies copies, faites par clonage (`cp --reflink=always`, sans espace disque
# en plus sur btrfs ou xfs), jamais des copies complètes de 10 000 photos.
# Usage : scripts/mesures/dossiers.sh <dossier des photos générées> <destination> <n>…
set -euo pipefail
photos=${1:?dossier des photos générées}
dest=${2:?destination}
shift 2
mkdir -p "$dest"
for n in "$@"; do
  [ $((n % 50)) -eq 0 ] || { echo "n doit être un multiple de 50 (une sortie) : $n" >&2; exit 2; }
  cible="$dest/$n"
  if [ -d "$cible" ] && [ "$(find "$cible" -name '*.jpg' | wc -l)" -eq "$n" ]; then continue; fi
  rm -rf "$cible"
  mkdir -p "$cible"
  for d in $(ls "$photos" | grep '^sortie-' | sort | head -n $((n / 50))); do
    cp -a --reflink=always "$photos/$d" "$cible/$d"
  done
  [ "$(find "$cible" -name '*.jpg' | wc -l)" -eq "$n" ] || { echo "Il manque des photos dans $cible (photos générées : $photos)." >&2; exit 2; }
done

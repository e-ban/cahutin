#!/usr/bin/env bash
# Photos de mesure, toutes dessinées par ImageMagick (jamais de vraie photo) : 40 images de base de 4000x3000
# (3 à 6 Mo), puis autant de copies légères (reflink) qu'il en faut, rangées par « sortie » de 50 photos et
# datées de septembre à mars. Une photo sur vingt porte aussi une date d'appareil (EXIF).
# Usage : scripts/mesures/photos.sh <dossier> <nombre>
set -euo pipefail
dir=${1:?dossier}
n=${2:?nombre}
bases="$dir/.bases"
mkdir -p "$bases"

# Les bases : un ciel, un sol, deux formes colorées et un grain plus ou moins fort (la taille du fichier).
skies=('#cfe3f5' '#d9ecd0' '#bfe0f7' '#fde2c4' '#e8def5')
grounds=('#e9d9c4' '#6aa84f' '#c8b49a' '#a1887f')
shapes=('#e74c3c' '#2980b9' '#8e44ad' '#f1c40f' '#27ae60' '#e67e22' '#26a69a' '#5c6bc0')
for i in $(seq 0 39); do
  f="$bases/base-$i.jpg"
  [ -s "$f" ] && continue
  grain=$(awk "BEGIN { print 0.22 + ($i % 5) * 0.03 }")
  magick -size 4000x3000 "xc:${skies[i % 5]}" \( -size 4000x1200 "xc:${grounds[i % 4]}" \) -gravity south -composite \
    -fill "${shapes[i % 8]}" -draw "roundrectangle $((300 + i * 20)),1100 1700,2100 60,60" \
    -fill "${shapes[(i + 3) % 8]}" -draw "circle 2700,1600 2700,$((1800 + i * 10))" \
    -attenuate "$grain" +noise Gaussian -quality 92 "$f"
done

# Au-delà de 500 photos, de vraies copies rempliraient le disque : seules les copies légères sont permises.
reflink=always
probe="$dir/.essai-reflink"
if ! cp --reflink=always "$bases/base-0.jpg" "$probe" 2>/dev/null; then
  if [ "$n" -gt 500 ]; then
    echo "Copies légères (reflink) impossibles ici : choisissez un dossier sur btrfs ou XFS." >&2
    exit 2
  fi
  reflink=never
fi
rm -f "$probe"

start=$(date -d 2026-09-01 +%s)
span=$(( $(date -d 2027-03-31 +%s) - start ))
exif="$dir/.exif-args"
: >"$exif"
made=()
for k in $(seq 1 "$n"); do
  sub=$(printf '%s/sortie-%02d' "$dir" $(( (k - 1) / 50 + 1 )))
  f=$(printf '%s/photo-%05d.jpg' "$sub" "$k")
  [ -e "$f" ] && continue
  mkdir -p "$sub"
  cp --reflink=$reflink "$bases/base-$(( k % 40 )).jpg" "$f"
  made+=("$k")
  # Une date d'appareil pour une photo sur vingt (exiftool réécrit le fichier entier : moins de place), écrite en un seul passage d'exiftool à la fin.
  if [ $(( k % 20 )) -eq 1 ]; then
    when=$(( start + span * (k - 1) / (n > 1 ? n - 1 : 1) ))
    printf -- '-DateTimeOriginal=%s\n%s\n-execute\n' "$(date -d "@$when" '+%Y:%m:%d %H:%M:%S')" "$f" >>"$exif"
  fi
done
[ -s "$exif" ] && exiftool -@ "$exif" -common_args -q -overwrite_original
rm -f "$exif"
# Les dates de fichier en dernier : exiftool les aurait changées. Les photos d'une même sortie se suivent.
for k in "${made[@]}"; do
  when=$(( start + span * (k - 1) / (n > 1 ? n - 1 : 1) ))
  touch -d "@$when" "$(printf '%s/sortie-%02d/photo-%05d.jpg' "$dir" $(( (k - 1) / 50 + 1 )) "$k")"
done

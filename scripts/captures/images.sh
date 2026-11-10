#!/usr/bin/env bash
# Images de test des captures, toutes dessinées par ImageMagick (jamais de vraie photo) :
# photos d'activités de classe, sur trois jours, et dessins pour la Collection.
# Usage : scripts/captures/images.sh <dossier>
set -euo pipefail
dir=${1:?dossier}
mkdir -p "$dir/photos" "$dir/collection"

# Un « tableau » coloré : fond, quelques formes, comme une photo d'activité vue de loin.
photo() {
  local name=$1 sky=$2 ground=$3 a=$4 b=$5 when=$6
  magick -size 1200x900 "xc:$sky" \( -size 1200x360 "xc:$ground" \) -gravity south -composite \
    -fill "$a" -draw "roundrectangle 160,330 520,640 30,30" \
    -fill "$b" -draw "circle 820,470 820,600" \
    -fill '#ffffff' -draw "rectangle 600,180 1080,260" \
    "$dir/photos/$name.jpg"
  touch -d "$when" "$dir/photos/$name.jpg"
}
photo peinture-1 '#cfe3f5' '#e9d9c4' '#e74c3c' '#f1c40f' '2026-10-05 10:00'
photo peinture-2 '#cfe3f5' '#e9d9c4' '#2980b9' '#e67e22' '2026-10-05 10:20'
photo peinture-3 '#d9ecd0' '#e9d9c4' '#8e44ad' '#27ae60' '2026-10-05 10:40'
photo jardin-1 '#bfe0f7' '#6aa84f' '#795548' '#f6c945' '2026-10-07 14:00'
photo jardin-2 '#bfe0f7' '#6aa84f' '#a1887f' '#ef5350' '2026-10-07 14:15'
photo jardin-3 '#bfe0f7' '#6aa84f' '#8d6e63' '#ab47bc' '2026-10-07 14:30'
photo jeux-1 '#fde2c4' '#c8b49a' '#26a69a' '#5c6bc0' '2026-10-09 09:30'
photo jeux-2 '#fde2c4' '#c8b49a' '#ff7043' '#66bb6a' '2026-10-09 09:45'

# Dessins de la Collection, sur fond transparent.
draw() { magick -size 600x600 xc:none "$@"; }
draw -fill '#e53935' -draw "circle 300,330 300,520" -fill '#6d4c41' -draw "rectangle 290,100 310,170" -fill '#43a047' -draw "ellipse 360,140 70,30 0,360" "$dir/collection/pomme.png"
draw -fill '#fb8c00' -draw "ellipse 300,330 220,170 0,360" -fill '#5d4037' -draw "rectangle 285,130 315,180" "$dir/collection/citrouille.png"
draw -fill '#c0592b' -draw "polygon 300,60 470,300 300,540 130,300" -stroke '#7a3415' -strokewidth 8 -draw "line 300,80 300,540" "$dir/collection/feuille.png"
draw -fill '#1e88e5' -draw "ellipse 280,300 190,110 0,360" -draw "polygon 450,300 560,200 560,400" -fill '#ffffff' -draw "circle 190,280 190,300" "$dir/collection/poisson.png"
draw -fill '#9e9e9e' -draw "circle 300,340 300,500" -draw "polygon 190,230 230,90 290,200" -draw "polygon 410,230 370,90 310,200" -fill '#212121' -draw "circle 250,320 250,338" -draw "circle 350,320 350,338" "$dir/collection/chat.png"

#!/usr/bin/env bash
# Captures d'écran de la release (README, site, Store, nouveautés), sur les exemples de Cahutin.
# Prérequis : tauri-driver (cargo install tauri-driver), WebKitWebDriver, xvfb-run, ImageMagick, Node.
# Usage : scripts/captures/capturer.sh <dossier de sortie> [scène…]
#         GUIDES=1 scripts/captures/capturer.sh assets/guides [guide/image…]   (images des guides)
# Le guide du flou demande FLOU_IMAGE : une illustration de classe (pas une vraie photo d'enfants).
# Les images de l'assistant demandent Ollama et le modèle qwen3:4b : Ollama est lancé s'il ne tourne pas.
# L'application tourne avec un dossier personnel jetable : ni vos documents ni vos réglages ne sont touchés.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
out=$(realpath "${1:?dossier de sortie}"); shift
work=$(mktemp -d)
mkdir -p "$out" "$work/home/Documents"
# La photo de test : un paysage dessiné.
magick -size 1200x900 gradient:'#7fb7e6-#cfe7f7' \( -size 1200x380 xc:'#6aa84f' \) -gravity south -composite \
  -fill '#2e6b34' -draw "polygon 120,560 230,300 340,560" -draw "polygon 300,560 400,340 500,560" -draw "polygon 760,560 880,260 1000,560" \
  -fill '#c8553d' -draw "rectangle 820,470 1040,640" -fill '#a33b2a' -draw "polygon 800,480 930,380 1060,480" \
  -fill '#f6c945' -draw "circle 1060,170 1060,250" "$work/paysage.jpg"
"$here/images.sh" "$work/images"
(cd "$root" && cargo build -p carnet-app)
(cd "$root" && npx vite --port 5173 --strictPort >"$work/vite.log" 2>&1) & vite=$!
(cd "$work" && HOME="$work/home" XDG_DATA_HOME="$work/home/.local/share" XDG_CONFIG_HOME="$work/home/.config" XDG_CACHE_HOME="$work/home/.cache" \
  xvfb-run -a -s "-screen 0 1920x1080x24" tauri-driver --native-driver "$(command -v WebKitWebDriver)" >"$work/driver.log" 2>&1) & driver=$!
# À la fin, tout s'arrête : serveur, pilote, application et écran virtuel, avec leurs processus enfants.
killtree() { local c; for c in $(pgrep -P "$1"); do killtree "$c"; done; kill "$1" 2>/dev/null || true; }
ollama=
if command -v ollama >/dev/null && ! curl -s -m 2 localhost:11434/api/tags >/dev/null; then
  OLLAMA_CONTEXT_LENGTH=16384 setsid ollama serve >"$work/ollama.log" 2>&1 & ollama=$!
fi
trap 'killtree $vite; killtree $driver; [ -n "$ollama" ] && killtree $ollama; true' EXIT
sleep 5
script=scenes.mjs
[ -n "${GUIDES:-}" ] && script=guides.mjs
PHOTO="$work/paysage.jpg" IMAGES="$work/images" HOME_CAP="$work/home/Documents" node "$here/$script" "$out" "$@"

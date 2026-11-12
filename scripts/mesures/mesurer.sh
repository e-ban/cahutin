#!/usr/bin/env bash
# Banc des images sur une « machine modeste » : l'application tourne sur 2 cœurs avec 4 Go de mémoire, dans un
# dossier personnel jetable (ni vos documents ni vos réglages ne sont touchés). Le disque n'est pas ralenti.
# Prérequis : tauri-driver, WebKitWebDriver, xvfb-run, systemd (session utilisateur), ImageMagick, exiftool, Node.
# Usage : scripts/mesures/mesurer.sh <dossier de sortie> [scène…]
#         PHOTOS=<dossier> pour reprendre des photos déjà générées (sinon 10 000 dans <sortie>/.photos).
# Pour Timonier, qui ne lit que le JUnit écrit pendant son propre lancement :
#         timonier test --junit <sortie>/mesures.xml -- scripts/mesures/mesurer.sh <sortie>
# L'application est la version de développement (bibliothèques d'images optimisées, voir Cargo.toml) ; son
# interface est servie par vite.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
mkdir -p "${1:?dossier de sortie}"
out=$(realpath "$1"); shift
photos=${PHOTOS:-$out/.photos}
[ -n "${PHOTOS:-}" ] || "$here/photos.sh" "$photos" 10000
work=$(mktemp -d)
mkdir -p "$work/home/Documents"
: >"$out/.pid"
(cd "$root" && cargo build -p carnet-app)
(cd "$root" && npx vite --port 5173 --strictPort >"$work/vite.log" 2>&1) & vite=$!
# Le pilote et l'application qu'il lance héritent des limites : 2 cœurs, 4 Go sans échange.
(cd "$work" && HOME="$work/home" XDG_DATA_HOME="$work/home/.local/share" XDG_CONFIG_HOME="$work/home/.config" XDG_CACHE_HOME="$work/home/.cache" \
  systemd-run --user --scope -q -p MemoryMax=4G -p MemorySwapMax=0 \
  taskset -c 0,1 xvfb-run -a -s "-screen 0 1920x1080x24" tauri-driver --native-driver "$(command -v WebKitWebDriver)" >"$work/driver.log" 2>&1) & driver=$!
killtree() { local c; for c in $(pgrep -P "$1"); do killtree "$c"; done; kill "$1" 2>/dev/null || true; }
descendants() { local c; for c in $(pgrep -P "$1"); do echo "$c"; descendants "$c"; done; }
# Le PID de l'application, parmi les descendants du pilote (jamais un autre « cahutin » de la machine).
(for _ in $(seq 1 600); do
  for p in $(descendants "$driver"); do
    if [ "$(cat "/proc/$p/comm" 2>/dev/null)" = cahutin ]; then echo "$p" >"$out/.pid"; exit 0; fi
  done
  sleep 0.5
done) & finder=$!
trap 'killtree $finder; killtree $vite; killtree $driver; true' EXIT
sleep 5
node "$here/images.mjs" "$photos" "$out" "$@"
echo "Résultats : $out/mesures.md et $out/mesures.xml"

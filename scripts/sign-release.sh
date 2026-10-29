#!/usr/bin/env bash
# Signe les empreintes d'une version publiée : télécharge les paquets et SHA256SUMS (ou le calcule s'il
# manque), vérifie chaque empreinte, signe SHA256SUMS avec la clé GPG du dépôt, puis ajoute SHA256SUMS
# et SHA256SUMS.asc à la version. Usage : scripts/sign-release.sh v0.5.0
set -euo pipefail

tag=${1:?usage : scripts/sign-release.sh vX.Y.Z}
repo=e-ban/cahutin
key=$(git config --get user.signingkey)
work=$(mktemp -d)
cd "$work"

gh release download "$tag" --repo "$repo" --pattern 'Cahutin_*'
if gh release download "$tag" --repo "$repo" --pattern SHA256SUMS 2>/dev/null; then
  sha256sum --check SHA256SUMS
else
  sha256sum Cahutin_* > SHA256SUMS
fi
cat SHA256SUMS

gpg --local-user "$key" --armor --detach-sign --output SHA256SUMS.asc SHA256SUMS
gpg --verify SHA256SUMS.asc SHA256SUMS
gh release upload "$tag" SHA256SUMS SHA256SUMS.asc --repo "$repo" --clobber
echo "Empreintes signées ajoutées à $tag ($work)."

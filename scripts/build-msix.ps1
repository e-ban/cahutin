# Paquet MSIX pour le Microsoft Store, à partir de l'application construite en version Store
# (CAHUTIN_STORE=1 : sans avis de nouvelle version, le Store fait les mises à jour).
# Usage : scripts/build-msix.ps1 -Name <Identity/Name> -Publisher "CN=…" -PublisherDisplay "…"
param(
  [Parameter(Mandatory)] [string] $Name,
  [Parameter(Mandatory)] [string] $Publisher,
  [Parameter(Mandatory)] [string] $PublisherDisplay
)
$ErrorActionPreference = "Stop"

# Version du Store : quatre nombres, le dernier à 0.
$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"$' | Select-Object -First 1).Matches[0].Groups[1].Value
$layout = "target/msix/layout"
Remove-Item -Recurse -Force target/msix -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$layout/Assets" | Out-Null

Copy-Item target/release/cahutin.exe $layout
foreach ($icon in "StoreLogo", "Square44x44Logo", "Square150x150Logo") {
  Copy-Item "src-tauri/icons/$icon.png" "$layout/Assets/"
}
(Get-Content packaging/msix/AppxManifest.xml -Raw -Encoding utf8) `
  -replace '\{\{NAME\}\}', $Name `
  -replace '\{\{PUBLISHER\}\}', [System.Security.SecurityElement]::Escape($Publisher) `
  -replace '\{\{PUBLISHER_DISPLAY\}\}', [System.Security.SecurityElement]::Escape($PublisherDisplay) `
  -replace '\{\{VERSION\}\}', "$version.0" |
  Set-Content "$layout/AppxManifest.xml" -Encoding utf8

$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" | Sort-Object FullName | Select-Object -Last 1
if (-not $makeappx) { throw "makeappx.exe introuvable (Windows SDK)" }
& $makeappx.FullName pack /o /d $layout /p "target/msix/Cahutin_${version}_x64.msix"
if ($LASTEXITCODE -ne 0) { throw "makeappx a échoué" }
Write-Output "target/msix/Cahutin_${version}_x64.msix"

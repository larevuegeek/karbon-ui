# Export each menu open (--open) at 1280x900, dump it and compare it with the Tauri capture.
#
#   cargo build --release --example rustmusic_html
#   pwsh examples/rustmusic_html/tools/menus.ps1 -Reference <dir of menu-*.json/png> [-Out <dir>] [-Menus profile,sleep]
#
# Writes <Out>/menu-<name>.png/json, report-<name>.txt (compare.py, only the elements
# of the menu: those absent from the page without menu) and side-by-side crops of the menu.
param(
    [Parameter(Mandatory = $true)][string]$Reference,
    [string]$Out = "target/rustmusic-menus",
    [string[]]$Menus = @(),
    [string]$Renderer = "software"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot/../../.."
$exe = Join-Path $root "target/release/examples/rustmusic_html.exe"
$compare = Join-Path $PSScriptRoot "compare.py"
# --open name -> capture, crop (logical x,y,w,h around the menu)
$captures = [ordered]@{
    sleep    = @("menu-01-minuteur-de-veille", "590,40,290,300")
    profile  = @("menu-02-menu-du-profil", "840,40,270,320")
    source   = @("menu-03-changer-de-source", "0,60,300,300")
    open     = @("menu-04-ouvrir", "0,60,320,420")
    library  = @("menu-05-changer-de-bibliothèque", "0,300,320,420")
    display  = @("menu-06-options-d-affichage", "0,180,320,400")
    mix      = @("menu-07-actions-du-mix", "300,440,420,320")
    output   = @("menu-08-sortie-audio", "800,420,480,460")
    queue    = @("menu-09-file-d-attente", "760,40,520,860")
    lyrics   = @("menu-10-paroles", "760,40,520,860")
    network  = @("menu-11-lecteurs-réseau-upnp-dlna", "880,440,400,460")
    whatsnew = @("menu-12-nouveautés", "200,40,880,820")
    search   = @("menu-search", "290,0,560,520")
}
New-Item -ItemType Directory -Force $Out | Out-Null
$names = if ($Menus.Count) { $Menus -split "," } else { $captures.Keys }
foreach ($name in $names) {
    $capture, $crop = $captures[$name]
    $json = Join-Path $Out "menu-$name.json"
    $png = Join-Path $Out "menu-$name.png"
    & $exe --open $name --dump $json --export $png --size 1280x900 --scale 1.5 "--renderer=$Renderer" | Out-Null
    $report = Join-Path $Out "report-$name.txt"
    python -I $compare (Join-Path $Reference "$capture.json") $json --only-new (Join-Path $Reference "home-1280x900.json") --menu $name `
        --ref-png (Join-Path $Reference "$capture.png") --png $png --out (Join-Path $Out "crops-$name") --crop $crop > $report
    $summary = Get-Content $report | Select-String "^matched" | Select-Object -Last 1
    Write-Output "$name  $summary"
}

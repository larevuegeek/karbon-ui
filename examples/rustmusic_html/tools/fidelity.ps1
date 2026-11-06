# Dump, export and compare the page with the Tauri captures at the five reference sizes.
#
#   cargo build --release --example rustmusic_html
#   pwsh examples/rustmusic_html/tools/fidelity.ps1 -Reference <dir of home-WxH.json/png> [-Out <dir>] [-Sizes 1280x900,960x700]
#
# Writes <Out>/home-WxH.json, home-WxH.png and report-WxH.txt (see compare.py).
param(
    [Parameter(Mandatory = $true)][string]$Reference,
    [string]$Out = "target/rustmusic-fidelity",
    [string[]]$Sizes = @("960x700", "1100x760", "1280x900", "1440x1040", "1600x1000"),
    [string]$Renderer = "software"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot/../../.."
$exe = Join-Path $root "target/release/examples/rustmusic_html.exe"
$compare = Join-Path $PSScriptRoot "compare.py"
New-Item -ItemType Directory -Force $Out | Out-Null
foreach ($size in $Sizes) {
    $json = Join-Path $Out "home-$size.json"
    $png = Join-Path $Out "home-$size.png"
    & $exe --dump $json --export $png --size $size --scale 1.5 "--renderer=$Renderer" | Out-Null
    $report = Join-Path $Out "report-$size.txt"
    python -I $compare (Join-Path $Reference "home-$size.json") $json > $report
    $summary = Get-Content $report | Select-String "^matched" | Select-Object -Last 1
    Write-Output "$size  $summary"
}

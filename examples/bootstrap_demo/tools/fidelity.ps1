# Render the demo natively and its plain HTML equivalent with Microsoft Edge (headless,
# temporary profile under target/), then compare them side by side.
#
#   cargo build --release --example bootstrap_demo
#   pwsh examples/bootstrap_demo/tools/fidelity.ps1 [-Out artifacts/bootstrap] [-Size 1280x2000] [-Scale 1.5]
#        [-Theme light|dark] [-Open dropdown,modal,collapse] [-Tab profile-pane] [-Switch] [-Name light]
#
# Writes <Out>/<Name>-edge.png, <Name>-native.png, <Name>-side-by-side.png, <Name>-diff.png
# and <Name>-report.txt. Edge never uses the user's profile (--user-data-dir in target/).
param(
    [string]$Out = "artifacts/bootstrap",
    [string]$Size = "1280x2000",
    [double]$Scale = 1.5,
    [string]$Theme = "light",
    [string]$Open = "",
    [string]$Tab = "",
    [string]$Name = "",
    [string]$Renderer = "software",
    [switch]$Switch
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot/../../.."
if (-not $Name) { $Name = $Theme + $(if ($Open) { "-" + ($Open -replace ',', '-') } else { "" }) + $(if ($Tab) { "-" + $Tab } else { "" }) }
New-Item -ItemType Directory -Force $Out | Out-Null
$Out = Resolve-Path $Out
$work = Join-Path $root "target/bootstrap-fidelity"
New-Item -ItemType Directory -Force $work | Out-Null
$profile = Join-Path $work "edge-profile"
$html = Join-Path $work "$Name.html"
$refArgs = @($html, "--theme", $Theme)
if ($Open) { $refArgs += @("--open", $Open) }
if ($Tab) { $refArgs += @("--tab", $Tab) }
if ($Switch) { $refArgs += "--switch" }
python -I (Join-Path $PSScriptRoot "reference.py") @refArgs
$w, $h = $Size -split 'x'
$edge = Join-Path $Out "$Name-edge.png"
$url = "file:///" + (($html -replace '\\', '/') -replace ' ', '%20')
$edgeExe = "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"
$p = Start-Process -FilePath $edgeExe -Wait -PassThru -NoNewWindow -ArgumentList @(
    "--headless=new", "--disable-gpu", "--no-first-run", "--no-default-browser-check",
    "--user-data-dir=`"$profile`"", "--window-size=$w,$h", "--force-device-scale-factor=$Scale",
    "--hide-scrollbars", "--screenshot=`"$edge`"", $url)
if ($p.ExitCode -ne 0) { throw "Edge exited with $($p.ExitCode)" }
$native = Join-Path $Out "$Name-native.png"
$exe = Join-Path $root "target/release/examples/bootstrap_demo.exe"
$demoArgs = @("--export", $native, "--size", $Size, "--scale", "$Scale", "--theme", $Theme, "--renderer=$Renderer")
if ($Open) { $demoArgs += @("--open", $Open) }
if ($Tab) { $demoArgs += @("--tab", $Tab) }
if ($Switch) { $demoArgs += "--switch" }
& $exe @demoArgs | Out-Null
$report = Join-Path $Out "$Name-report.txt"
python -I (Join-Path $PSScriptRoot "compare_png.py") $edge $native (Join-Path $Out "$Name-side-by-side.png") --diff (Join-Path $Out "$Name-diff.png") | Tee-Object -FilePath $report

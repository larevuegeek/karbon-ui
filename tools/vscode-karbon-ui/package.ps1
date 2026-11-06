param([string]$OutputPath)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '../..')).Path
$manifest = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'package.json') -Raw | ConvertFrom-Json
if (-not $OutputPath) {
    $OutputPath = Join-Path $projectRoot "artifacts/$($manifest.name)-$($manifest.version).vsix"
}
$OutputPath = [System.IO.Path]::GetFullPath($OutputPath)
[System.IO.Directory]::CreateDirectory([System.IO.Path]::GetDirectoryName($OutputPath)) | Out-Null
$identity = "$($manifest.publisher).$($manifest.name)"
$vsixManifest = @"
<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="$($manifest.name)" Version="$($manifest.version)" Publisher="$($manifest.publisher)" />
    <DisplayName>$($manifest.displayName)</DisplayName>
    <Description xml:space="preserve">Coloration des templates HTML Karbon UI et expressions Rust.</Description>
    <Tags>karbon,ui,rust</Tags>
    <Categories>Programming Languages,Snippets</Categories>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="$($manifest.engines.vscode)" />
      <Property Id="Microsoft.VisualStudio.Code.ExtensionKind" Value="ui" />
    </Properties>
  </Metadata>
  <Installation><InstallationTarget Id="Microsoft.VisualStudio.Code" /></Installation>
  <Dependencies />
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true" />
  </Assets>
</PackageManifest>
"@
$contentTypes = @'
<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="json" ContentType="application/json" />
  <Default Extension="md" ContentType="text/markdown" />
  <Default Extension="vsixmanifest" ContentType="text/xml" />
</Types>
'@
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$stream = [System.IO.File]::Open($OutputPath, [System.IO.FileMode]::Create)
$archive = [System.IO.Compression.ZipArchive]::new($stream, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($entry in @{'extension.vsixmanifest'=$vsixManifest;'[Content_Types].xml'=$contentTypes}.GetEnumerator()) {
        $writer = [System.IO.StreamWriter]::new($archive.CreateEntry($entry.Key).Open(), [System.Text.UTF8Encoding]::new($false))
        try { $writer.Write($entry.Value) } finally { $writer.Dispose() }
    }
    foreach ($relative in @('package.json','language-configuration.json','syntaxes/karbon-ui.tmLanguage.json','snippets/karbon-ui.json','README.md')) {
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $archive, (Join-Path $PSScriptRoot $relative), "extension/$relative",
            [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally { $archive.Dispose(); $stream.Dispose() }
Write-Output "$identity : $OutputPath ($((Get-Item -LiteralPath $OutputPath).Length) octets)"

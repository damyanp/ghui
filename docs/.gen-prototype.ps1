# Embeds docs/.prototype-data.json into pivoting-prototype.html.
# Run: pwsh docs/.gen-prototype.ps1
$ErrorActionPreference = 'Stop'
$dataPath = Join-Path $PSScriptRoot '.prototype-data.json'
$outPath  = Join-Path $PSScriptRoot 'pivoting-prototype.html'
$tmplPath = Join-Path $PSScriptRoot '.prototype-template.html'

if (-not (Test-Path $dataPath)) { throw "Missing $dataPath" }
if (-not (Test-Path $tmplPath)) { throw "Missing $tmplPath" }

$data = Get-Content $dataPath -Raw -Encoding UTF8
$data = $data -replace '</script', '<\/script'

$tmpl = Get-Content $tmplPath -Raw -Encoding UTF8
$marker = '__GHUI_DATA_PLACEHOLDER__'
if ($tmpl -notlike "*$marker*") { throw "Placeholder not found in template" }

$idx = $tmpl.IndexOf($marker)
$out = $tmpl.Substring(0, $idx) + $data + $tmpl.Substring($idx + $marker.Length)

Set-Content -Path $outPath -Value $out -NoNewline -Encoding UTF8

$fi = Get-Item $outPath
Write-Host "Wrote $($fi.FullName) ($($fi.Length) bytes)"

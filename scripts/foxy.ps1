$ErrorActionPreference = "Stop"

$repo = if ($env:FOXY_REPO) { $env:FOXY_REPO } else { "Wgmlgz/foxy-jumpscare" }
$apiUrl = "https://api.github.com/repos/$repo/releases/latest"
$release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "Accept" = "application/vnd.github+json" }
$assets = $release.assets

$arch = if ([Environment]::Is64BitOperatingSystem) { "x64" } else { "x86" }

# Use only the portable binary (no installer).
$asset = $assets | Where-Object { $_.name -match "_windows_${arch}\.exe$" } | Select-Object -First 1
if (-not $asset) {
  throw "No matching portable Windows binary asset was found in the latest release."
}

if ($env:FOXY_PRINT_ONLY -eq "1") {
  $asset.browser_download_url
  exit 0
}

$tempDir = Join-Path $env:TEMP ("foxy-jumpscare-" + [Guid]::NewGuid().ToString("N"))
New-Item -Path $tempDir -ItemType Directory | Out-Null

$assetPath = Join-Path $tempDir $asset.name
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $assetPath

Start-Process -FilePath $assetPath

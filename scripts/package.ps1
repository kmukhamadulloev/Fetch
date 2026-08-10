param(
  [Parameter(Mandatory = $true)][string]$Target,
  [string]$Label = $Target
)
$ErrorActionPreference = 'Stop'
$version = (Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"' | Select-Object -First 1).Matches.Groups[1].Value
if (-not $version) { $version = '0.1.0' }
$binary = Join-Path "target/$Target/release" 'fetch.exe'
if (-not (Test-Path $binary)) { throw "Release binary not found: $binary" }
$artifact = "fetch-v$version-$Label"
$stage = "target/package/$artifact"
New-Item -ItemType Directory -Force -Path $stage, 'target/release-artifacts' | Out-Null
Copy-Item $binary, README.md, LICENSE, THIRD_PARTY_NOTICES.md -Destination $stage
$archive = "target/release-artifacts/$artifact.zip"
Compress-Archive -Path $stage -DestinationPath $archive -Force
$hash = (Get-FileHash -Algorithm SHA256 $archive).Hash.ToLowerInvariant()
"$hash  $artifact.zip" | Set-Content -Encoding ascii "$archive.sha256"
Write-Output $archive

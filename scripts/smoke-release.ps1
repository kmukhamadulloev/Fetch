param([Parameter(Mandatory = $true)][string]$Binary)
$ErrorActionPreference = 'Stop'
$smokeRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("fetch-smoke-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path "$smokeRoot/data", "$smokeRoot/downloads" | Out-Null
$config = Join-Path $smokeRoot 'fetch.toml'
@"
bind_address = "127.0.0.1"
port = 18991
data_directory = "$($smokeRoot.Replace('\', '\\'))/data"
download_directory = "$($smokeRoot.Replace('\', '\\'))/downloads"
open_browser_on_start = false
"@ | Set-Content -Encoding utf8 $config
$env:FETCH_CONFIG = $config
$process = Start-Process -FilePath $Binary -PassThru -RedirectStandardOutput "$smokeRoot/fetch.log" -RedirectStandardError "$smokeRoot/fetch-error.log"
try {
  for ($attempt = 0; $attempt -lt 100; $attempt++) {
    if ($process.HasExited) { throw "Fetch exited during startup" }
    try {
      $status = Invoke-RestMethod -Uri 'http://127.0.0.1:18991/api/status' -TimeoutSec 1
      if ($status.server -eq 'ready') { exit 0 }
    } catch { Start-Sleep -Milliseconds 100 }
  }
  throw 'Fetch did not become ready'
} finally {
  if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force }
  Remove-Item Env:FETCH_CONFIG -ErrorAction SilentlyContinue
  Remove-Item -Recurse -Force $smokeRoot
}

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
$process = Start-Process -FilePath $Binary -ArgumentList '--no-tray' -PassThru -RedirectStandardOutput "$smokeRoot/fetch.log" -RedirectStandardError "$smokeRoot/fetch-error.log"
$ready = $false
try {
  for ($attempt = 0; $attempt -lt 100; $attempt++) {
    if ($process.HasExited) { throw "Fetch exited during startup" }
    try {
      $status = Invoke-RestMethod -Uri 'http://127.0.0.1:18991/api/status' -TimeoutSec 1
      if ($status.server -eq 'ready') {
        $ready = $true
        break
      }
    } catch { Start-Sleep -Milliseconds 100 }
  }
  if (-not $ready) { throw 'Fetch did not become ready' }
} finally {
  if (-not $process.HasExited) {
    Stop-Process -Id $process.Id -Force
    $null = $process.WaitForExit(10000)
  }
  Remove-Item Env:FETCH_CONFIG -ErrorAction SilentlyContinue

  # Runtime bootstrap may still have a temporary download open when the status
  # endpoint first becomes ready. Windows can retain that handle briefly after
  # process termination, so cleanup must tolerate the filesystem release race.
  $removed = $false
  for ($attempt = 0; $attempt -lt 10; $attempt++) {
    try {
      Remove-Item -Recurse -Force $smokeRoot -ErrorAction Stop
      $removed = $true
      break
    } catch {
      if ($attempt -lt 9) { Start-Sleep -Milliseconds 250 }
    }
  }
  if (-not $removed) {
    Write-Warning "Fetch passed its startup smoke test, but temporary files could not be removed: $smokeRoot"
  }
}

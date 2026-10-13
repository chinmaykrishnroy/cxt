$ErrorActionPreference = 'Stop'
$repo = 'chinmaykrishnroy/cxt'
$version = if ($env:CXT_VERSION) { $env:CXT_VERSION } else { 'latest' }
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$asset = "cxt-$arch.exe"
$base = if ($version -eq 'latest') { "https://github.com/$repo/releases/latest/download" } else { "https://github.com/$repo/releases/download/$version" }
$url = "$base/$asset"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
$installDir = if ($env:CXT_INSTALL_DIR) { $env:CXT_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\cxt' }
New-Item -ItemType Directory -Force $tmp, $installDir | Out-Null
try {
  $download = Join-Path $tmp 'cxt.exe'
  $checksums = Join-Path $tmp 'SHA256SUMS.txt'
  Invoke-WebRequest -UseBasicParsing $url -OutFile $download
  Invoke-WebRequest -UseBasicParsing "$base/SHA256SUMS.txt" -OutFile $checksums

  $line = Get-Content $checksums | Where-Object { $_ -match "\s+$([regex]::Escape($asset))$" } | Select-Object -First 1
  if (-not $line) { throw "Checksum for $asset not found" }
  $expected = ($line -split '\s+')[0].ToLowerInvariant()
  $actual = (Get-FileHash -Algorithm SHA256 $download).Hash.ToLowerInvariant()
  if ($actual -ne $expected) { throw 'SHA256 verification failed' }

  Copy-Item $download (Join-Path $installDir 'cxt.exe') -Force
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $pathEntries = @($userPath -split ';' | Where-Object { $_ })
  if ($pathEntries -notcontains $installDir) {
    [Environment]::SetEnvironmentVariable('Path', (($pathEntries + $installDir) -join ';'), 'User')
    Write-Host "Added $installDir to your user PATH. Open a new terminal to use cxt."
  }
  Write-Host "Installed cxt to $(Join-Path $installDir 'cxt.exe')"
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}


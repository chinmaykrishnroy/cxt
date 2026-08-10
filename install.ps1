$ErrorActionPreference = 'Stop'
$repo = 'chinmaykrishnroy/cxt'
$version = if ($env:CXT_VERSION) { $env:CXT_VERSION } else { 'latest' }
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$url = "https://github.com/$repo/releases/$version/download/cxt-$arch.exe"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
$installDir = if ($env:CXT_INSTALL_DIR) { $env:CXT_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\cxt' }
New-Item -ItemType Directory -Force $tmp, $installDir | Out-Null
try {
  Invoke-WebRequest -UseBasicParsing $url -OutFile (Join-Path $tmp 'cxt.exe')
  Copy-Item (Join-Path $tmp 'cxt.exe') (Join-Path $installDir 'cxt.exe') -Force
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  $pathEntries = @($userPath -split ';' | Where-Object { $_ })
  if ($pathEntries -notcontains $installDir) {
    [Environment]::SetEnvironmentVariable('Path', (($pathEntries + $installDir) -join ';'), 'User')
    Write-Host "Added $installDir to your user PATH. Open a new terminal to use cxt."
  }
  Write-Host "Installed cxt to $(Join-Path $installDir 'cxt.exe')"
} finally { Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue }

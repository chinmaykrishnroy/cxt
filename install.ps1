$ErrorActionPreference = 'Stop'
$repo = 'chinmaykrishnroy/cxt'
$version = if ($env:CXT_VERSION) { $env:CXT_VERSION } else { 'latest' }
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$url = "https://github.com/$repo/releases/$version/download/cxt-$arch.zip"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
$installDir = if ($env:CXT_INSTALL_DIR) { $env:CXT_INSTALL_DIR } else { Join-Path $HOME '.cargo\bin' }
New-Item -ItemType Directory -Force $tmp, $installDir | Out-Null
try { Invoke-WebRequest -UseBasicParsing $url -OutFile (Join-Path $tmp 'cxt.zip'); Expand-Archive (Join-Path $tmp 'cxt.zip') $tmp -Force; Copy-Item (Join-Path $tmp "cxt-$arch.exe") (Join-Path $installDir 'cxt.exe') -Force; Write-Host "Installed cxt to $(Join-Path $installDir 'cxt.exe')" } finally { Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue }

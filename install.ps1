# Install the Puddi toolchain (`pud`) from GitHub releases on Windows.
#
#   irm https://raw.githubusercontent.com/puddi-lang/puddi/main/install.ps1 | iex
#
# Environment:
#   PUD_VERSION          release tag to install, such as v0.1.0 (default: latest)
#   PUD_INSTALL          install root; pud.exe goes to $env:PUD_INSTALL\bin (default: ~\.pud)
#   PUD_NO_MODIFY_PATH   set to 1 to leave the user PATH alone

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

$Repo = 'puddi-lang/puddi'
$Version = if ($env:PUD_VERSION) { $env:PUD_VERSION } else { 'latest' }
$Root = if ($env:PUD_INSTALL) { $env:PUD_INSTALL } else { Join-Path $HOME '.pud' }
$Bin = Join-Path $Root 'bin'

function Say($msg) { Write-Host "pud-install: $msg" }

$arch = $env:PROCESSOR_ARCHITECTURE
if ($env:PROCESSOR_ARCHITEW6432) { $arch = $env:PROCESSOR_ARCHITEW6432 }
switch ($arch) {
    'AMD64' { $target = 'x86_64-pc-windows-msvc' }
    # Windows on ARM runs the x64 build through emulation.
    'ARM64' { $target = 'x86_64-pc-windows-msvc' }
    default { throw "pud-install: unsupported CPU architecture: $arch" }
}

$asset = "pud-$target.zip"
$base = if ($Version -eq 'latest') { "https://github.com/$Repo/releases/latest/download" } else { "https://github.com/$Repo/releases/download/$Version" }

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("pud-install-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    Say "downloading $asset ($Version)"
    $zip = Join-Path $tmp $asset
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset" -OutFile $zip
    Invoke-WebRequest -UseBasicParsing -Uri "$base/$asset.sha256" -OutFile "$zip.sha256"

    $expected = ((Get-Content "$zip.sha256" -Raw).Trim() -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLowerInvariant()
    if ($expected -ne $actual) { throw "pud-install: checksum mismatch for $asset (expected $expected, got $actual)" }

    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    New-Item -ItemType Directory -Force -Path $Bin | Out-Null
    Copy-Item -Force (Join-Path $tmp "pud-$target\pud.exe") (Join-Path $Bin 'pud.exe')
} finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

$pud = Join-Path $Bin 'pud.exe'
Say "installed $(& $pud version) to $pud"

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$entries = @($userPath -split ';' | Where-Object { $_ })
if ($entries -notcontains $Bin -and $env:PUD_NO_MODIFY_PATH -ne '1') {
    [Environment]::SetEnvironmentVariable('Path', (@($Bin) + $entries) -join ';', 'User')
    $env:Path = "$Bin;$env:Path"
    Say "added $Bin to the user PATH; open a new terminal to pick it up elsewhere"
}

if (Get-Command node -ErrorAction SilentlyContinue) {
    $major = [int]((node -p 'process.versions.node.split(".")[0]') 2>$null)
    if ($major -lt 22) { Say "note: ``pud test`` and the browser target need Node.js 22 or newer (found $(node --version))" }
} else {
    Say "note: ``pud test`` and the browser target need Node.js 22 or newer; ``pud run`` works without it"
}

Say 'get started: pud new hello; cd hello; pud run'

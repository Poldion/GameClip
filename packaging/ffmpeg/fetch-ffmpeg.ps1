# SPDX-License-Identifier: GPL-3.0-or-later
# Lädt den in ffmpeg.lock gepinnten FFmpeg-Build, prüft die SHA-256 und entpackt nach
# third_party/ffmpeg/{bin,include,lib}. Ein vorhandenes Archiv mit passender Prüfsumme wird
# nicht erneut geladen. In CI wird third_party/ffmpeg/bin an PATH angehängt.
$ErrorActionPreference = 'Stop'

$repo = Resolve-Path "$PSScriptRoot/../.."
$lock = @{}
foreach ($line in Get-Content "$PSScriptRoot/ffmpeg.lock") {
    if ($line -match '^\s*([a-z0-9_]+)\s*=\s*(.+?)\s*$') { $lock[$Matches[1]] = $Matches[2] }
}

$target = Join-Path $repo 'third_party/ffmpeg'
$download = Join-Path $target 'download'
$zip = Join-Path $download ([IO.Path]::GetFileName($lock.url))
New-Item -ItemType Directory -Force $download | Out-Null

function Get-Sha256($path) { (Get-FileHash $path -Algorithm SHA256).Hash.ToLowerInvariant() }

if (-not (Test-Path $zip) -or (Get-Sha256 $zip) -ne $lock.sha256) {
    Write-Host "Lade $($lock.url)"
    Invoke-WebRequest $lock.url -OutFile $zip
}
$actual = Get-Sha256 $zip
if ($actual -ne $lock.sha256) {
    Remove-Item $zip
    throw "Prüfsumme falsch: erwartet $($lock.sha256), erhalten $actual"
}

$stamp = Join-Path $target '.version'
if (-not (Test-Path $stamp) -or (Get-Content $stamp -Raw).Trim() -ne $lock.sha256) {
    $extract = Join-Path $target 'extract'
    Remove-Item -Recurse -Force $extract, (Join-Path $target 'bin'), (Join-Path $target 'include'), (Join-Path $target 'lib') -ErrorAction SilentlyContinue
    Expand-Archive $zip -DestinationPath $extract
    $root = (Get-ChildItem $extract -Directory | Select-Object -First 1).FullName
    foreach ($dir in 'bin', 'include', 'lib') { Move-Item (Join-Path $root $dir) $target }
    Copy-Item (Join-Path $root 'LICENSE.txt') (Join-Path $target 'LICENSE.txt') -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force $extract
    Set-Content $stamp $lock.sha256
}

# Bindings für rusty_ffmpeg (FFMPEG_BINDING_PATH in .cargo/config.toml). Braucht libclang
# (LLVM; auf den GitHub-Windows-Runnern vorinstalliert).
$binding = Join-Path $target 'binding.rs'
$bindingStamp = Join-Path $target '.binding-version'
if (-not (Test-Path $binding) -or -not (Test-Path $bindingStamp) -or (Get-Content $bindingStamp -Raw).Trim() -ne $lock.sha256) {
    if (-not $env:LIBCLANG_PATH -and (Test-Path 'C:\Program Files\LLVM\bin\libclang.dll')) {
        $env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
    }
    cargo run --quiet --release --manifest-path (Join-Path $repo 'tools/ffmpeg-bindgen/Cargo.toml') -- (Join-Path $target 'include') $binding
    if ($LASTEXITCODE -ne 0) { throw "ffmpeg-bindgen fehlgeschlagen ($LASTEXITCODE)" }
    Set-Content $bindingStamp $lock.sha256
}

$bin = Join-Path $target 'bin'
if ($env:GITHUB_PATH) { Add-Content $env:GITHUB_PATH $bin }
Write-Host "FFmpeg $($lock.version) bereit in $target"

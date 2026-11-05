# SPDX-License-Identifier: GPL-3.0-or-later
# Baut das MSIX-Paket (Design, Datei 69): Release-Builds, FFmpeg-DLLs, Lizenzen und Symbole in
# packaging/out/layout, Platzhalter im Manifest ersetzen, MakeAppx pack, optional signieren.
#
#   -NoSign    nur packen (CI signiert über SignPath)
#   -TestCert  selbst signiertes Zertifikat mit Subjekt = Publisher anlegen bzw. wiederverwenden,
#              nach Cert:\LocalMachine\TrustedPeople importieren (Administratorrechte!) und
#              mit SignTool signieren. Ändert den Zertifikatsspeicher des Rechners.
param(
    [switch]$NoSign,
    [switch]$TestCert,
    [string]$Version = '0.1.0.0',
    [string]$Publisher = 'CN=GameClip Dev'
)
$ErrorActionPreference = 'Stop'

$repo = Resolve-Path "$PSScriptRoot/.."
$out = Join-Path $repo 'packaging/out'
$layout = Join-Path $out 'layout'
$msix = Join-Path $out "GameClip_$Version`_x64.msix"

$sdk = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin\*\x64\makeappx.exe' |
    Sort-Object FullName | Select-Object -Last 1
if (-not $sdk) { throw 'Windows SDK (makeappx.exe) nicht gefunden' }
$bin = $sdk.DirectoryName

# 1. Release-Build.
Push-Location $repo
try {
    cargo build --release -p gameclipd -p gameclip-worker
    if ($LASTEXITCODE -ne 0) { throw "cargo build fehlgeschlagen ($LASTEXITCODE)" }
} finally { Pop-Location }

# 2. Layout.
Remove-Item -Recurse -Force $layout -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $layout, (Join-Path $layout 'assets'), (Join-Path $layout 'licenses') | Out-Null
foreach ($exe in 'gameclipd.exe', 'gameclip-worker.exe') {
    Copy-Item (Join-Path $repo "target/release/$exe") $layout
}
# Nur die DLLs, die Dienst und Worker laden (dumpbin /dependents, 2026-11); avdevice und
# avfilter (41 MB) braucht keines der beiden, ffmpeg.exe/ffplay.exe gehören nicht ins Paket.
foreach ($dll in 'avcodec-62.dll', 'avformat-62.dll', 'avutil-60.dll', 'swresample-6.dll', 'swscale-9.dll') {
    Copy-Item (Join-Path $repo "third_party/ffmpeg/bin/$dll") $layout
}
Copy-Item (Join-Path $repo 'packaging/assets/*.png') (Join-Path $layout 'assets')
Copy-Item (Join-Path $repo 'LICENSE') (Join-Path $layout 'licenses/GameClip-LICENSE.txt')
Copy-Item (Join-Path $repo 'third_party/ffmpeg/LICENSE.txt') (Join-Path $layout 'licenses/FFmpeg-LICENSE.txt')
$lock = Get-Content (Join-Path $repo 'packaging/ffmpeg/ffmpeg.lock') | Where-Object { $_ -match '^(version|source) =' }
Set-Content (Join-Path $layout 'licenses/FFmpeg-SOURCE.txt') (@(
    'FFmpeg ist unter der GPL-3.0 lizenziert. Der Quellcode der mitgelieferten Version:'
) + $lock)

$manifest = Get-Content (Join-Path $repo 'packaging/AppxManifest.template.xml') -Raw
$manifest = $manifest.Replace('{{VERSION}}', $Version).Replace('{{PUBLISHER}}', $Publisher)
Set-Content (Join-Path $layout 'AppxManifest.xml') $manifest -Encoding utf8NoBOM

# 3. Packen.
& (Join-Path $bin 'makeappx.exe') pack /d $layout /p $msix /o /h SHA256
if ($LASTEXITCODE -ne 0) { throw "MakeAppx fehlgeschlagen ($LASTEXITCODE)" }

if ($NoSign -or -not $TestCert) {
    Write-Host "Unsigniertes Paket: $msix"
    return
}

# 4. Test-Signatur (Administratorrechte für TrustedPeople).
$cert = Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $Publisher } | Select-Object -First 1
if (-not $cert) {
    $cert = New-SelfSignedCertificate -Type Custom -Subject $Publisher -KeyUsage DigitalSignature `
        -FriendlyName 'GameClip Test' -CertStoreLocation Cert:\CurrentUser\My `
        -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
}
$cer = Join-Path $out 'GameClip-Test.cer'
Export-Certificate -Cert $cert -FilePath $cer | Out-Null
$trusted = Get-ChildItem Cert:\LocalMachine\TrustedPeople |
    Where-Object { $_.Thumbprint -eq $cert.Thumbprint }
if (-not $trusted) {
    $admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).
        IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if ($admin) {
        Import-Certificate -FilePath $cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null
    } else {
        # Nur dieser Schritt braucht Administratorrechte: Windows fragt per UAC nach.
        Write-Host 'Zertifikat wird nach LocalMachine\TrustedPeople importiert (UAC-Abfrage) …'
        $cmd = "Import-Certificate -FilePath '$cer' -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
        $p = Start-Process pwsh -Verb RunAs -Wait -PassThru -ArgumentList '-NoProfile', '-Command', $cmd
        if ($p.ExitCode -ne 0) { throw "Import des Zertifikats abgelehnt oder fehlgeschlagen ($($p.ExitCode))" }
    }
}
& (Join-Path $bin 'signtool.exe') sign /fd SHA256 /sha1 $cert.Thumbprint $msix
if ($LASTEXITCODE -ne 0) { throw "SignTool fehlgeschlagen ($LASTEXITCODE)" }
Write-Host "Signiertes Paket: $msix"

# SPDX-License-Identifier: GPL-3.0-or-later
# Installiert GameClip aus einem Release mit Testzertifikat.
#
# Liegt neben GameClip_<Version>_x64.msix und GameClip-Release.cer. Rechtsklick →
# „Mit PowerShell ausführen“. Nur beim ersten Mal fragt Windows nach Administratorrechten:
# Das Zertifikat wird unter „Lokaler Computer\Vertrauenswürdige Personen“ eingetragen, damit
# Windows Pakete mit dieser Signatur annimmt. Mehr ändert das Skript nicht.
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot

try {
    $cerPath = Join-Path $here 'GameClip-Release.cer'
    $msix = Get-ChildItem (Join-Path $here 'GameClip_*_x64.msix') | Sort-Object Name | Select-Object -Last 1
    if (-not (Test-Path $cerPath)) { throw 'GameClip-Release.cer fehlt im selben Ordner.' }
    if (-not $msix) { throw 'GameClip_<Version>_x64.msix fehlt im selben Ordner.' }

    $cert = New-Object Security.Cryptography.X509Certificates.X509Certificate2 $cerPath
    $trusted = Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Thumbprint -EQ $cert.Thumbprint
    if (-not $trusted) {
        Write-Host "Zertifikat $($cert.Subject) ($($cert.Thumbprint)) wird eingetragen – Windows fragt nach Administratorrechten …"
        $cmd = "Import-Certificate -FilePath '$cerPath' -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null"
        $p = Start-Process powershell.exe -Verb RunAs -Wait -PassThru -ArgumentList '-NoProfile', '-Command', $cmd
        if ($p.ExitCode -ne 0) { throw "Zertifikat nicht eingetragen (abgelehnt oder Fehler $($p.ExitCode))." }
    }

    # Läuft GameClip, beendet Windows es für das Update und startet es nicht neu.
    Write-Host "Installiere $($msix.Name) …"
    Add-AppxPackage -Path $msix.FullName -ForceApplicationShutdown
    Write-Host 'Fertig. GameClip im Startmenü öffnen.' -ForegroundColor Green
} catch {
    Write-Host "Fehler: $_" -ForegroundColor Red
}
Read-Host 'Enter zum Schließen'

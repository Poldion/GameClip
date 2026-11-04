# SPDX-License-Identifier: GPL-3.0-or-later
# Erzeugt die App-Symbole für das MSIX-Paket (Design, Datei 71): dunkles abgerundetes Quadrat
# mit rotem Aufnahmepunkt. Die PNGs werden eingecheckt; das Skript bleibt für Änderungen.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$out = Join-Path $PSScriptRoot 'assets'
New-Item -ItemType Directory -Force $out | Out-Null

function New-Logo([int]$w, [int]$h, [string]$name, [double]$scale = 0.62) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $g.Clear([System.Drawing.Color]::Transparent)
    $s = [Math]::Min($w, $h) * $scale
    $x = ($w - $s) / 2; $y = ($h - $s) / 2; $r = $s * 0.22
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($x, $y, 2 * $r, 2 * $r, 180, 90)
    $path.AddArc($x + $s - 2 * $r, $y, 2 * $r, 2 * $r, 270, 90)
    $path.AddArc($x + $s - 2 * $r, $y + $s - 2 * $r, 2 * $r, 2 * $r, 0, 90)
    $path.AddArc($x, $y + $s - 2 * $r, 2 * $r, 2 * $r, 90, 90)
    $path.CloseFigure()
    $g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 32, 36, 44))), $path)
    $d = $s * 0.42
    $g.FillEllipse((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 230, 57, 70))), ($w - $d) / 2, ($h - $d) / 2, $d, $d)
    $g.Dispose()
    $bmp.Save((Join-Path $out $name), [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

New-Logo 44 44 'Square44x44Logo.png' 0.9
New-Logo 50 50 'StoreLogo.png' 0.9
New-Logo 150 150 'Square150x150Logo.png'
New-Logo 310 150 'Wide310x150Logo.png'
New-Logo 310 310 'Square310x310Logo.png'
Write-Host "Symbole in $out"

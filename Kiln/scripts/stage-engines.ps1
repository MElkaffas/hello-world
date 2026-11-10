<#
  Copies the conversion engines installed on this build machine into a staging folder and links
  it as src-tauri\engines, so `npx tauri build` ships them inside the installer.

  Prerequisites (install once with winget):
    Gyan.FFmpeg, ImageMagick.ImageMagick, JohnMacFarlane.Pandoc, oschwartz10612.Poppler,
    QPDF.QPDF, 7zip.7zip, UB-Mannheim.TesseractOCR

  Usage:  powershell -ExecutionPolicy Bypass -File scripts\stage-engines.ps1 [-Stage D:\kiln-build\engines-stage]
#>
param([string]$Stage = "D:\kiln-build\engines-stage")
$ErrorActionPreference = 'Stop'
$pkgs = "$env:LOCALAPPDATA\Microsoft\WinGet\Packages"
function Need($p, $what) { if (-not $p -or -not (Test-Path $p)) { throw "$what not found. Install it first (see header)." }; $p }

if (Test-Path $Stage) { Remove-Item $Stage -Recurse -Force }
New-Item -ItemType Directory $Stage | Out-Null

$ff = Need ((Get-ChildItem $pkgs -Recurse -Filter ffmpeg.exe -ErrorAction SilentlyContinue | Select-Object -First 1).Directory.Parent.FullName) "FFmpeg"
New-Item -ItemType Directory "$Stage\ffmpeg" | Out-Null
Copy-Item "$ff\bin\ffmpeg.exe", "$ff\bin\ffprobe.exe" "$Stage\ffmpeg\"
Copy-Item "$ff\LICENSE" "$Stage\ffmpeg\LICENSE.txt" -ErrorAction SilentlyContinue

$im = Need ((Get-ChildItem "C:\Program Files" -Directory -Filter "ImageMagick*" | Select-Object -First 1).FullName) "ImageMagick"
robocopy $im "$Stage\imagemagick" /E /XD www uninstall /XF ChangeLog.md unins*.* /NFL /NDL /NJH /NJS | Out-Null

$pd = Need "$env:LOCALAPPDATA\Pandoc\pandoc.exe" "Pandoc"
New-Item -ItemType Directory "$Stage\pandoc" | Out-Null
Copy-Item $pd "$Stage\pandoc\"
Copy-Item "$env:LOCALAPPDATA\Pandoc\COPYING*" "$Stage\pandoc\" -ErrorAction SilentlyContinue

$pop = Need ((Get-ChildItem $pkgs -Recurse -Filter pdftoppm.exe -ErrorAction SilentlyContinue | Select-Object -First 1).DirectoryName) "Poppler"
robocopy $pop "$Stage\poppler" /E /NFL /NDL /NJH /NJS | Out-Null

$qp = Need ((Get-ChildItem "C:\Program Files" -Directory -Filter "qpdf*" | Select-Object -First 1).FullName) "qpdf"
robocopy "$qp\bin" "$Stage\qpdf" /E /NFL /NDL /NJH /NJS | Out-Null

Need "C:\Program Files\7-Zip\7z.exe" "7-Zip" | Out-Null
New-Item -ItemType Directory "$Stage\7zip" | Out-Null
Copy-Item "C:\Program Files\7-Zip\7z.exe", "C:\Program Files\7-Zip\7z.dll", "C:\Program Files\7-Zip\License.txt" "$Stage\7zip\"

Need "C:\Program Files\Tesseract-OCR\tesseract.exe" "Tesseract" | Out-Null
robocopy "C:\Program Files\Tesseract-OCR" "$Stage\tesseract" /E /XD doc /XF unins*.* *.html /NFL /NDL /NJH /NJS | Out-Null

$link = Join-Path $PSScriptRoot "..\src-tauri\engines"
if (Test-Path $link) { cmd /c rmdir "$link" | Out-Null }
cmd /c mklink /J "$link" "$Stage" | Out-Null
"{0:N0} MB staged in $Stage and linked as src-tauri\engines" -f ((Get-ChildItem $Stage -Recurse -File | Measure-Object Length -Sum).Sum / 1MB)

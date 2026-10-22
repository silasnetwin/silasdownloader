$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Der Ordner enthaelt im Repository keine Dateien, weil die Werkzeuge nicht
# mitversioniert werden. Er muss also erst angelegt werden.
$ziel = Join-Path $PSScriptRoot '..\src-tauri\resources\bin'
New-Item -ItemType Directory -Force -Path $ziel | Out-Null
Set-Location $ziel

Write-Host '=== SilasDownloader Werkzeuge aktualisieren ===' -ForegroundColor Cyan

$temp = Join-Path $env:TEMP 'SilasDownloader-tools'

function Test-RealBinary([string]$Path, [long]$MinSize) {
    return (Test-Path $Path) -and ((Get-Item $Path).Length -ge $MinSize)
}

New-Item -ItemType Directory -Force -Path $temp | Out-Null

if (-not (Test-RealBinary 'yt-dlp.exe' 500000)) {
    Write-Host '[1/4] Lade yt-dlp Nightly ...' -ForegroundColor Yellow
    Invoke-WebRequest -UseBasicParsing 'https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp.exe' -OutFile 'yt-dlp.exe'
}

if (-not ((Test-RealBinary 'ffmpeg.exe' 5000000) -and (Test-RealBinary 'ffprobe.exe' 1000000))) {
    Write-Host '[2/4] Lade FFmpeg Essentials ...' -ForegroundColor Yellow
    $zip = Join-Path $temp 'ffmpeg.zip'
    $unpack = Join-Path $temp 'ffmpeg-unpack'
    Remove-Item $unpack -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-WebRequest -UseBasicParsing 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip' -OutFile $zip
    Expand-Archive $zip -DestinationPath $unpack -Force
    $ffmpeg = Get-ChildItem $unpack -Recurse -Filter 'ffmpeg.exe' | Select-Object -First 1
    $ffprobe = Get-ChildItem $unpack -Recurse -Filter 'ffprobe.exe' | Select-Object -First 1
    if (-not $ffmpeg -or -not $ffprobe) { throw 'FFmpeg wurde im Archiv nicht gefunden.' }
    Copy-Item $ffmpeg.FullName 'ffmpeg.exe' -Force
    Copy-Item $ffprobe.FullName 'ffprobe.exe' -Force
}

if (-not (Test-RealBinary 'deno.exe' 5000000)) {
    Write-Host '[3/4] Lade Deno (JavaScript-Runtime) ...' -ForegroundColor Yellow
    $zip = Join-Path $temp 'deno.zip'
    $unpack = Join-Path $temp 'deno-unpack'
    Remove-Item $unpack -Recurse -Force -ErrorAction SilentlyContinue
    Invoke-WebRequest -UseBasicParsing 'https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip' -OutFile $zip
    Expand-Archive $zip -DestinationPath $unpack -Force
    $deno = Get-ChildItem $unpack -Recurse -Filter 'deno.exe' | Select-Object -First 1
    if (-not $deno) { throw 'deno.exe wurde nicht gefunden.' }
    Copy-Item $deno.FullName 'deno.exe' -Force
}

if (-not (Test-RealBinary 'aria2c.exe' 1000000)) {
    Write-Host '[4/4] Lade aria2c (Turbo-Downloads) ...' -ForegroundColor Yellow
    $zip = Join-Path $temp 'aria2.zip'
    $unpack = Join-Path $temp 'aria2-unpack'
    Remove-Item $unpack -Recurse -Force -ErrorAction SilentlyContinue

    # Zuerst die feste Adresse: die API von GitHub ist ohne Anmeldung schnell am
    # Limit und laesst dann jeden weiteren Aufruf scheitern.
    $quellen = @(
        'https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip'
    )

    # Nur als Ausnahme: die neueste Version erfragen (mit Token, falls vorhanden).
    try {
        $kopf = @{ 'User-Agent' = 'SilasDownloader' }
        if ($env:GITHUB_TOKEN) { $kopf['Authorization'] = "Bearer $env:GITHUB_TOKEN" }
        $release = Invoke-RestMethod -UseBasicParsing -Headers $kopf 'https://api.github.com/repos/aria2/aria2/releases/latest'
        $asset = $release.assets | Where-Object { $_.name -match 'win-64bit.*\.zip$' } | Select-Object -First 1
        if ($asset) { $quellen += $asset.browser_download_url }
    } catch {
        Write-Host '  (GitHub-API nicht erreichbar - feste Adresse genuegt)' -ForegroundColor DarkGray
    }

    $geladen = $false
    foreach ($quelle in $quellen) {
        try {
            Invoke-WebRequest -UseBasicParsing $quelle -OutFile $zip
            $geladen = $true
            break
        } catch {
            Write-Host "  Fehlgeschlagen: $quelle" -ForegroundColor DarkYellow
        }
    }
    if (-not $geladen) { throw 'aria2c konnte nicht geladen werden.' }
    Expand-Archive $zip -DestinationPath $unpack -Force
    $aria2 = Get-ChildItem $unpack -Recurse -Filter 'aria2c.exe' | Select-Object -First 1
    if (-not $aria2) { throw 'aria2c.exe wurde nicht gefunden.' }
    Copy-Item $aria2.FullName 'aria2c.exe' -Force
}

Write-Host ''
Write-Host 'FERTIG — die Werkzeuge liegen in src-tauri\resources\bin' -ForegroundColor Green
Get-ChildItem -Filter '*.exe' | Select-Object Name, @{ Name = 'MB'; Expression = { [math]::Round($_.Length / 1MB, 1) } } | Format-Table -AutoSize

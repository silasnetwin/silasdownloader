# Baut den Installer. Liegt ein Signaturschluessel vor, entstehen dabei auch die
# Dateien fuer das Auto-Update (Setup-Datei, Signatur und latest.json).
#
#   ./scripts/build.ps1
#
# Der Schluessel und sein Passwort liegen absichtlich ausserhalb des
# Repositories. Auf diesem Rechner sind ausserdem Signaturvariablen anderer
# Projekte gesetzt — die werden hier ueberschrieben, damit nicht der falsche
# Schluessel verwendet wird.

$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

# Pfad zum eigenen Schluessel, bei Bedarf ueber die Umgebung anpassbar.
$keyPath = $env:SILAS_SIGNING_KEY
if ([string]::IsNullOrWhiteSpace($keyPath)) {
    $keyPath = Join-Path $env:USERPROFILE '.tauri\silasdownloader.key'
}

# Variablen anderer Projekte entfernen.
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PATH -ErrorAction SilentlyContinue

if (-not (Test-Path $keyPath)) {
    Write-Host 'Kein Signaturschluessel gefunden.' -ForegroundColor Yellow
    Write-Host 'Der Installer entsteht ohne Update-Dateien; die App laesst sich dann nicht aktualisieren.' -ForegroundColor Yellow
    npx tauri build --config '{"bundle":{"createUpdaterArtifacts":false}}'
    exit $LASTEXITCODE
}

# Das Passwort steht in einer Datei neben dem Schluessel oder in der Umgebung.
$password = $env:SILAS_SIGNING_PASSWORD
if ([string]::IsNullOrEmpty($password)) {
    $passwordFile = [System.IO.Path]::ChangeExtension($keyPath, '.pw')
    if (Test-Path $passwordFile) {
        $password = (Get-Content $passwordFile -Raw).Trim()
    }
}

$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $keyPath -Raw)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $password

if ([string]::IsNullOrEmpty($password)) {
    Write-Host 'Achtung: kein Passwort fuer den Schluessel gefunden.' -ForegroundColor Yellow
} else {
    Write-Host 'Signaturschluessel gefunden — der Installer wird signiert.' -ForegroundColor Green
}

npx tauri build
exit $LASTEXITCODE

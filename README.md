<div align="center">

<img src="public/brand-logo.svg" alt="SilasDownloader" width="96" height="96" />

# SilasDownloader

**Portabler Desktop-Downloader für Videos, Serien und Musik von beliebigen Seiten.**

Alles Nötige steckt in der App: kein Python, kein Rust, keine Zusatzprogramme.
Ein Klick auf die Setup-Datei, ein zweiter auf den Link — fertig.

[![Prüfungen](https://github.com/silasnetwin/silasdownloader/actions/workflows/pruefungen.yml/badge.svg)](https://github.com/silasnetwin/silasdownloader/actions/workflows/pruefungen.yml)
[![Release](https://img.shields.io/github/v/release/silasnetwin/silasdownloader?label=Release&color=3fa37a)](https://github.com/silasnetwin/silasdownloader/releases/latest)
[![Lizenz](https://img.shields.io/badge/Lizenz-MIT-3fa37a)](LICENSE)

</div>

---

## Was die App kann

| | |
|---|---|
| 🎬 **Videos** | Link einfügen, fertig. Die App erkennt selbst, ob es ein einzelnes Video, eine Playlist oder eine Serie mit allen Folgen ist. |
| 📺 **Serien** | Serienseiten (etwa KiKA „Alle Folgen") werden in die einzelnen Folgen aufgelöst und nach Staffel gefiltert. |
| 🎵 **Musik** | Spotify-Playlist, -Album oder einzelner Track: jeder Titel wird gesucht, dabei nach **Titel, Interpret und Laufzeit** geprüft (keine Mixe oder „How it was made"-Videos) und als MP3 mit **Album, Tracknummer, Jahr, Songtext und quadratischem Cover** beschriftet. |
| 🔄 **Konverter** | Video in Audio, Tonspur in MP3 oder FLAC, mit Cover und Tags — per Drag & Drop. |
| 📡 **Direkt auf den Server** | Fertige Dateien legt die App auf deinen Homeserver. Die Zugangsdaten liegen in einem **verschlüsselten Tresor**. |
| 🖱️ **Aus dem Browser** | Die mitgelieferte Erweiterung übergibt die aktuelle Seite oder einen Link per `silasdownloader://` an die App. |
| 📥 **Im Infobereich** | Ein Symbol neben der Uhr zeigt den Fortschritt. Schließen legt die App dorthin — die Downloads laufen weiter. |
| 🌍 **Deutsch & Englisch** | Die Oberfläche lässt sich mitten im Betrieb umstellen (*Einstellungen → App & Workflow → Sprache*). Die Wahl bleibt gespeichert. |
| ⚡ **Tempo** | Bis zu 12 Aufträge parallel, Turbo-Downloader und Fragment-Download. Aus einem Auftrag mit 2,4 MB/s werden bei sechs Aufträgen 11,6 MB/s. |
| 🧩 **Nichts installiert** | yt-dlp, FFmpeg, Deno und aria2c sind bereits enthalten. |

## Installation

1. [Release herunterladen](https://github.com/silasnetwin/silasdownloader/releases/latest) —
   `SilasDownloader_<Version>_x64-setup.exe`
2. Setup starten. Die App legt sich unter `%LOCALAPPDATA%\SilasDownloader` ab und
   bringt ein Verknüpfung auf dem Desktop mit.
   Windows zeigt beim ersten Start eventuell „unbekannter Herausgeber" — das ist
   bei Programmen ohne kostenpflichtiges Zertifikat normal. Über *Weitere
   Informationen → Trotzdem ausführen* geht es weiter.
3. Link einfügen, „Holen" drücken, „Herunterladen" klicken.

Updates holt sich die App selbst — der Tab **Über** prüft auf neue Versionen.

## Auto-Update

Die App bringt einen eigenen Updater mit. Er fragt beim Start einmal leise nach
und lässt sich jederzeit über **Über → Nach Updates suchen** auslösen. Findet er
eine neue Version, lädt er den Installer herunter, prüft die **Signatur** und
startet die App danach neu.

Damit das sicher bleibt, nimmt die App nur Pakete an, die mit dem passenden
privaten Schlüssel signiert wurden. Der öffentliche Schlüssel steht in
`src-tauri/tauri.conf.json`, der private liegt **nicht** im Repository, sondern
als Secret bei GitHub (`TAURI_SIGNING_PRIVATE_KEY`). Verloren heißt verloren: ohne
diesen Schlüssel lassen sich keine Updates mehr ausliefern.

Eine neue Version veröffentlichen:

```powershell
# Version an drei Stellen erhöhen: package.json, src-tauri/Cargo.toml, src-tauri/tauri.conf.json
git commit -am "Version 1.3.0"
git tag v1.3.0
git push origin main --tags
```

Der Workflow `.github/workflows/release.yml` baut daraufhin den Installer, signiert
ihn, legt `latest.json` an und veröffentlicht das Release. Die App findet es beim
nächsten Start.

## Der Server-Upload

Fertige Dateien können direkt auf einen eigenen Server gelegt werden, ohne
Netzlaufwerk oder Explorer-Kopieren:

1. In den **Einstellungen** unter *Server-Upload* werden Passwort, Tailscale-Adresse
   und der API-Schlüssel von Jellyfin eingetragen. Erst danach lässt sich der
   Upload einschalten.
2. Die Dateien gehen per SSH in den Zielordner der passenden Bibliothek. **Filme**
   landen in der Filmsammlung, **Serien** in der Seriensammlung. Die App erkennt
   selbst, was geladen wurde. Musik bleibt immer lokal.
3. Danach stößt die App den Bibliotheks-Scan an, damit Jellyfin die Datei aufnimmt.

**Der Tresor:** Adresse, API-Schlüssel und Zielordner stehen nicht in den
Einstellungen, sondern in einer verschlüsselten Datei (`tresor.dat` im
AppData-Ordner). Der Schlüssel wird aus dem Passwort abgeleitet (Argon2id),
verschlüsselt wird mit AES-256-GCM. Ohne das Passwort ist die Datei wertlos —
das gilt auch, wenn sie vom Rechner kopiert wird.

**Welches Passwort ist gemeint?** Du denkst es dir beim ersten Einrichten selbst
aus. Es ist ein neues Passwort nur für diese App — nicht das Passwort deines
Servers, nicht das von Jellyfin und nicht das von Windows. Aus ihm berechnet die
App den Schlüssel für die verschlüsselte Datei. Gespeichert wird nur die Datei,
nie das Passwort. Solange die App läuft, merkt sie sich das Passwort und speichert
Änderungen ohne erneute Eingabe. Nach einem Neustart tippst du es einmal ein —
die Felder sind bereits ausgefüllt, weil die sichtbaren Angaben (Adresse, Name,
Benutzer, Jellyfin-Adresse und Zielordner) zusätzlich in `tresor-info.json`
liegen. Der API-Schlüssel bleibt ausschließlich im verschlüsselten Teil.

Das Passwort schützt die **App**, nicht den Server: auf der Leitung wird weiterhin
mit dem SSH-Schlüssel gearbeitet, es läuft also kein Kennwort über das Netz.

## Songtexte

Bei Spotify-Downloads holt die App den Songtext von
[LRCLIB](https://lrclib.net) — einer freien Sammlung ohne Schlüssel und ohne
Anmeldung — und schreibt ihn als **USLT** in die MP3. Damit zeigen ihn auch
iTunes und der Windows Media Player. Findet sich kein Text, fehlt der Rahmen
einfach. Abschaltbar unter **Einstellungen → Spotify → Songtexte einbetten**.

## Was die App speichert

Alles bleibt auf dem eigenen Rechner — es gibt keine Konten und keine Telemetrie.
Im Ordner `%APPDATA%\SilasDownloader` liegen:

| Datei | Inhalt |
|---|---|
| `settings.json` | deine Einstellungen und Presets |
| `history.json` | der Verlauf der letzten Downloads |
| `tresor.dat` | die Server-Angaben, verschlüsselt mit deinem Passwort |
| `tresor-info.json` | nur die sichtbaren Angaben (Adresse, Name, Benutzer, Zielordner) |

Die App schickt nichts an einen Server des Entwicklers. Nach außen geht nur, was
für einen Download nötig ist: die abgerufene Seite, der Textabruf bei LRCLIB und
die Update-Abfrage bei GitHub.

Dieses Repository enthält **keine** persönlichen Daten, keine Serveradressen, keine
Schlüssel und keine Zugangsdaten. Die Beispiele in der Oberfläche sind erfunden
(`100.100.1.1`, `mein-server`), und die Tests lesen ihre Angaben aus der Umgebung.

## Bedienung

- **Link einfügen** — Videos, Playlists und Serienseiten werden erkannt.
- **Drag & Drop** — ein Link aus dem Browser direkt ins Fenster ziehen.
- **Infobereich** — das Symbol neben der Uhr zeigt den Fortschritt. Ein Linksklick
  holt das Fenster nach vorn, das Menü bietet *Fenster zeigen* und *Beenden*.
  Schließt du das Fenster, läuft die App im Hintergrund weiter (abschaltbar unter
  *Einstellungen → App & Workflow*).
- **Beim Anmelden starten** — auf Wunsch startet die App beim Anmelden von Windows
  direkt in den Infobereich.
- **Sprache** — die Oberfläche gibt es auf Deutsch und auf Englisch. Die Umschaltung
  liegt unter *Einstellungen → App & Workflow* und wirkt sofort.
- **Einstellungen** — Presets, Parallelität, Zielordner, Benachrichtigungen.
- **Verlauf** — was zuletzt geladen wurde.

## Links aus dem Browser

Die App nimmt Adressen der Bauart `silasdownloader://download?url=…` entgegen.
Eine zweite Instanz landet nicht doppelt auf dem Bildschirm: wird die App bereits
ausgeführt, reicht Windows die Adresse an das laufende Fenster weiter.

Dafür liegt im Ordner `extension/` eine fertige Browser-Erweiterung für Chrome
und Edge:

1. `chrome://extensions` öffnen (bei Edge `edge://extensions`).
2. **Entwicklermodus** einschalten.
3. **Entpackte Erweiterung laden** und den Ordner `extension/` auswählen.

Danach gibt es einen Knopf in der Werkzeugleiste, „Diese Seite mit
SilasDownloader laden" und „Mit SilasDownloader laden" im Kontextmenü. Windows
fragt beim ersten Mal, ob die App die Adresse öffnen darf — danach nicht mehr.

## Oberfläche prüfen

Die Oberfläche hat eigene Tests. Sie laufen gegen den Entwicklungsserver, wobei
die Rust-Seite durch feste Antworten ersetzt wird (`src/lib/e2e.ts`). Im fertigen
Programm ist dieser Code nicht enthalten:

```powershell
npm run test:ui          # Playwright-Tests der Oberfläche
npm run test:ui -- --ui  # mit Oberfläche zum Zusehen
```

Die Tests laufen bei jedem Push in der Aktion *Prüfungen* mit.

## Bauen aus dem Quellcode

```powershell
npm install                 # Abhängigkeiten
./scripts/fetch-tools.ps1   # yt-dlp, FFmpeg, Deno und aria2c holen
npm run app:dev             # Entwicklung
npm run app:build           # Installer (NSIS) bauen
```

`app:build` ruft `scripts/build.ps1` auf. Liegt ein Signaturschlüssel unter
`%USERPROFILE%\.tauri\silasdownloader.key`, entstehen dabei die Update-Dateien
(Setup, Signatur und `latest.json`). Das Passwort dazu steht in
`%USERPROFILE%\.tauri\silasdownloader.pw` oder in `$env:SILAS_SIGNING_PASSWORD`.
Ohne Schlüssel wird der Installer trotzdem gebaut — die App kann sich dann nur
nicht selbst aktualisieren.

Weitere Skripte:

| Befehl | Zweck |
|---|---|
| `npm run build` | nur die Oberfläche bauen |
| `npm run typecheck` | TypeScript prüfen |
| `npm run test:ui` | Oberflächentests (Playwright) |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust-Tests |
| `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored` | Tests mit Internetzugang |

Die Tests für den Server-Upload lesen ihre Angaben aus der Umgebung und werden
sonst übersprungen:

```powershell
$env:SILAS_TEST_HOST = "100.100.1.1"
$env:SILAS_TEST_ALIAS = "mein-server"
$env:SILAS_TEST_API_KEY = "…"
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored
```

## Aufbau

```
src/                Oberfläche (React 19, TypeScript, Vite)
  components/       gemeinsame Bausteine
  pages/            Start, Downloads, Spotify, Konverter, Verlauf, Einstellungen, Über
  lib/e2e.ts        Prüfstand der Oberfläche (nur im Entwicklungsmodus)
  styles/           Design-System, hell und dunkel
src-tauri/          Backend (Rust)
  src/engine/       Download-Engine, Metadaten, Konverter, Serien
  src/spotify/      Spotify-Ablauf
  src/lyrics.rs     Songtexte von LRCLIB
  src/server.rs     Verbindung zum Server (Jellyfin und SSH)
  src/tray.rs       Symbol im Infobereich mit Fortschritt
  src/vault.rs      verschlüsselter Tresor
  resources/bin/    die mitgelieferten Werkzeuge
extension/          Browser-Erweiterung für Chrome und Edge
tests-ui/           Oberflächentests mit Playwright
```

**Stack:** Tauri 2 · React 19 · TypeScript · Vite · Rust · yt-dlp · FFmpeg · Deno · aria2c

## Technische Feinheiten

Drei Fallen, an denen die Engine unter Windows sonst hängenbleibt — sie sind in
`src-tauri/src/engine/process.rs` und `src-tauri/src/engine/tools.rs` abgesichert:

- **Die Ausgabe landet in Dateien statt in Pipes.** Pipes können unter Windows
  blockieren oder mitten in der Zeile sterben.
- **Die Ausgabe wird als cp1252 gelesen.** yt-dlp schreibt mit der
  Windows-Codepage; ein Umlaut im Dateinamen ist sonst kein gültiges UTF-8.
- **Die Statusdatei wird neu geschrieben, nicht fortgeschrieben.** Wer nur eine
  Byte-Position merkt, überspringt den Inhalt — der Dateiname der fertigen Datei
  käme nie an, und die MP3-Datei bekäme weder Tags noch Cover.

## Beitragen

Fehler, Verbesserungen und neue Quellen sind willkommen. Issues und Pull
Requests im [Repository](https://github.com/silasnetwin/silasdownloader).

## Rechte

Verwende Downloads nur für Inhalte, für die du die nötigen Rechte oder die
Erlaubnis des Rechteinhabers hast. Die App umgeht keine DRM-, Paywall- oder
Zugangsbeschränkungen und lädt nur Inhalte, die öffentlich erreichbar sind.

## Lizenz

MIT

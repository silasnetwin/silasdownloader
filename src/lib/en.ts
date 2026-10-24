//! Die englische Fassung der Oberfläche.
//!
//! Der Schlüssel ist immer der deutsche Satz, so wie er in der Oberfläche steht.
//! Fehlt hier ein Eintrag, zeigt die App den deutschen Text. Eine Prüfung in
//! `tests-ui` achtet darauf, dass kein Schlüssel doppelt vorkommt.
//!
//! Übersetzt wird nicht Wort für Wort, sondern so, wie ein englisches Programm
//! schreiben würde. Die Namen der Werkzeuge (yt-dlp, FFmpeg) bleiben.

export const en: Record<string, string> = {
  /* ---------- Navigation und Rahmen ---------- */
  Start: "Home",
  Downloads: "Downloads",
  Spotify: "Spotify",
  Konverter: "Converter",
  Verlauf: "History",
  Einstellungen: "Settings",
  Über: "About",
  bereit: "ready",
  "aktiv": "{n} active",
  "Helles Design": "Light theme",
  "Dunkles Design": "Dark theme",

  /* ---------- Statusleiste ---------- */
  "Bereit": "Ready",
  "{n} Download laufen": "{n} downloads running",
  "{n} Downloads laufen": "{n} downloads running",
  "{done} fertig · {failed} fehlgeschlagen · {parallel} parallel · yt-dlp {version}":
    "{done} done · {failed} failed · {parallel} parallel · yt-dlp {version}",
  "{percent} % erledigt": "{percent} % done",

  /* ---------- Startseite ---------- */
  "SILAS DOWNLOADER": "SILAS DOWNLOADER",
  "Alles von": "Anything, from",
  "überall": "anywhere",
  "Link einfügen — die App findet das Video, die Serie oder die Playlist und lädt sie im Hintergrund.":
    "Paste a link — the app finds the video, the series or the playlist and downloads it in the background.",
  "Video-, Playlist- oder Serienlink einfügen": "Paste a video, playlist or series link",
  "Suche läuft": "Searching",
  Holen: "Fetch",
  "Bitte einen Link einfügen.": "Please paste a link.",
  Herunterladen: "Download",
  "Weitere Videos auf dieser Seite": "More videos on this page",
  "{n} Streams": "{n} streams",
  "{n} Videos auf der Seite gefunden": "{n} videos found on the page",
  "{n} Staffeln": "{n} seasons",
  Alle: "All",
  Keine: "None",
  "{n} laden": "Download {n}",
  "Staffel {n}": "Season {n}",
  "Einstellungen für diesen Download": "Settings for this download",
  "{n} parallel": "{n} parallel",
  Video: "Video",
  "Nur Audio": "Audio only",
  Qualität: "Quality",
  Container: "Container",
  Audioformat: "Audio format",
  Zielordner: "Destination folder",
  "Ordner öffnen": "Open folder",
  Metadaten: "Metadata",
  Cover: "Cover",
  "Metadaten-Datei": "Metadata file",
  Beschreibung: "Description",
  Untertitel: "Subtitles",
  "Eigener Ordner pro Video": "Own folder per video",
  "Gleichzeitige Downloads: {n}": "Simultaneous downloads: {n}",
  "Verbindungen pro Stream: {n}": "Connections per stream: {n}",
  "Beste Qualität": "Best quality",
  "Kleinste Datei": "Smallest file",
  "Tipp: Über": "Tip: under",
  "lässt sich alles anpassen — Presets, Zielordner, Metadaten und Tempo.":
    "you can adjust everything — presets, destination folder, metadata and speed.",
  "{count} Videos sind in der Queue.": "{count} videos are queued.",
  "„{title}\" ist in der Queue.": "“{title}” is queued.",
  "„{name}\" übernommen.": "“{name}” applied.",

  /* ---------- Downloads ---------- */
  QUEUE: "QUEUE",
  "Hier siehst du exakt, was gerade passiert und wo die Datei landet.":
    "See exactly what is happening and where the file ends up.",
  "Aktive Aufträge": "Active jobs",
  "{pending} wartend · {active} aktiv · {done} fertig": "{pending} waiting · {active} active · {done} done",
  "Nur offene": "Only open",
  Protokoll: "Log",
  "Fertige entfernen": "Remove finished",
  "Keine Downloads in der Queue.": "No downloads in the queue.",
  "Gehe auf Home, füge einen Link ein und starte den Download.":
    "Go to Home, paste a link and start the download.",
  "Keine offenen Aufträge.": "No open jobs.",
  "Alle Downloads sind durch. Blende den Filter aus, um fertige zu sehen.":
    "All downloads are done. Turn off the filter to see finished ones.",
  "Die Ausgabe der Download-Engine.": "The output of the download engine.",
  Leeren: "Clear",
  "Noch keine Einträge.": "No entries yet.",

  /* ---------- Auftragszeile ---------- */
  Wartet: "Waiting",
  Analysiert: "Analyzing",
  Download: "Download",
  Verarbeitung: "Processing",
  Fertig: "Done",
  Fehlgeschlagen: "Failed",
  Abgebrochen: "Cancelled",
  Lädt: "Loading",
  "Speed {value}": "Speed {value}",
  "ETA {value}": "ETA {value}",
  "Größe {value}": "Size {value}",
  "Fehlgeschlagen:": "Failed:",
  "(automatisch wiederholt: {n}/2)": "(retried automatically: {n}/2)",
  Abbrechen: "Cancel",
  "Erneut versuchen": "Try again",
  "Im Explorer zeigen": "Show in Explorer",
  Entfernen: "Remove",

  /* ---------- Spotify ---------- */
  SPOTIFY: "SPOTIFY",
  "Eine Playlist, ein": "One playlist, one",
  Ordner: "folder",
  "Spotify-Link einfügen. Jeder Titel wird auf YouTube gesucht, als MP3 geladen und mit Album, Album-Interpret, Tracknummer und Cover beschriftet — ganz ohne Spotify-Konto.":
    "Paste a Spotify link. Every track is searched on YouTube, downloaded as MP3 and tagged with album, album artist, track number and cover — no Spotify account needed.",
  "Bitte einen Spotify-Link einfügen.": "Please paste a Spotify link.",
  "Liest …": "Reading …",
  Einlesen: "Read",
  "Cover einbetten": "Embed cover",
  "Songtexte einbetten": "Embed lyrics",
  "Als MP3 laden": "Download as MP3",
  "{n} Titel gefunden": "{n} tracks found",
  "{n} laut Spotify": "{n} according to Spotify",
  "und {n} weitere": "and {n} more",
  "Die Cover kommen direkt von Spotify und sind quadratisch. Das Album, die Tracknummer und das Jahr stehen in den Tags — bei Playlists sucht die App sie nach, weil Spotify sie dort nicht nennt.":
    "The covers come straight from Spotify and are square. Album, track number and year go into the tags — for playlists the app looks them up, because Spotify does not name them there.",
  Fortschritt: "Progress",
  "{done} fertig": "{done} done",
  "{failed} fehlgeschlagen": "{failed} failed",
  "{n} gestartet": "{n} started",
  "{n} MP3 gespeichert": "{n} MP3 saved",
  ", {n} fehlgeschlagen": ", {n} failed",

  /* ---------- Konverter ---------- */
  KONVERTER: "CONVERTER",
  "Media-Konverter": "Media converter",
  "Videos und Audio lokal mit FFmpeg umwandeln — ohne Upload und ohne Internet.":
    "Convert videos and audio locally with FFmpeg — no upload and no internet.",
  Quelldatei: "Source file",
  "Datei auswählen oder in das Fenster ziehen.": "Choose a file or drag it into the window.",
  "Datei wählen": "Choose file",
  Datei: "File",
  "Noch keine Datei gewählt.": "No file chosen yet.",
  "Ziehe eine Datei hierher oder nutze die Schaltfläche oben.":
    "Drag a file here or use the button above.",
  Zielformat: "Target format",
  "MP4 und MOV werden neu kodiert, MKV wird unverändert kopiert.":
    "MP4 and MOV are re-encoded, MKV is copied unchanged.",
  "Konvertiert …": "Converting …",
  Konvertieren: "Convert",
  "Ergebnis zeigen": "Show result",
  Medien: "Media",
  "Bitte zuerst eine Datei auswählen.": "Please choose a file first.",
  "Konvertierung abgeschlossen.": "Conversion finished.",
  "H.264, mit Schnellstart": "H.264, fast start",
  "ohne Neucodierung": "without re-encoding",
  "VP9 / Opus": "VP9 / Opus",
  "AAC, 256 kbit/s": "AAC, 256 kbit/s",
  unverändert: "unchanged",
  verlustfrei: "lossless",

  /* ---------- Verlauf ---------- */
  VERLAUF: "HISTORY",
  "Alle abgeschlossenen Downloads dieses Geräts.": "All finished downloads on this device.",
  Einträge: "Entries",
  "{n} Downloads": "{n} downloads",
  "Verlauf löschen": "Clear history",
  "Verlauf gelöscht.": "History cleared.",
  "Noch keine Downloads.": "No downloads yet.",
  "Abgeschlossene Downloads erscheinen hier automatisch.":
    "Finished downloads appear here automatically.",
  Format: "Format",
  Datum: "Date",
  Öffnen: "Open",

  /* ---------- Über ---------- */
  ÜBER: "ABOUT",
  "Portabler Downloader für Videos, Serien und Musik von beliebigen Seiten. Alles Nötige steckt in der App — kein Python, kein Rust, keine Zusatzprogramme.":
    "Portable downloader for videos, series and music from any site. Everything it needs is inside the app — no Python, no Rust, no extra programs.",
  "Version {version}": "Version {version}",
  "Was die App kann": "What the app can do",
  "Drei Bereiche, ein Programm.": "Three areas, one program.",
  "Videos und Serien": "Videos and series",
  "Ein Link genügt: die App erkennt selbst, ob es ein Video, eine Playlist oder eine Serie mit allen Folgen ist.":
    "One link is enough: the app works out whether it is a video, a playlist or a series with all episodes.",
  "Musik aus Spotify": "Music from Spotify",
  "Playlist oder Album: jeder Titel wird gesucht, als MP3 geladen und mit Album, Tracknummer, Jahr und quadratischem Cover beschriftet.":
    "Playlist or album: every track is searched, downloaded as MP3 and tagged with album, track number, year and a square cover.",
  "Video in Audio, Tonspur in MP3 oder FLAC, mit Cover und Tags — per Drag & Drop.":
    "Video to audio, audio to MP3 or FLAC, with cover and tags — by drag & drop.",
  Geschwindigkeit: "Speed",
  "YouTube begrenzt die Rate pro Stream. Der Hebel ist die Parallelität.":
    "YouTube limits the rate per stream. Parallelism is the lever.",
  "{n} Auftrag": "{n} job",
  "{n} Aufträge": "{n} jobs",
  "Gemessen mit denselben sechs Videos. Die Zahl der gleichzeitigen Aufträge steht in den Einstellungen — aktuell {n}.":
    "Measured with the same six videos. The number of simultaneous jobs is in the settings — currently {n}.",
  Technik: "Technical",
  "Alles steckt in einer Datei.": "Everything is in one file.",
  Oberfläche: "Interface",
  Backend: "Backend",
  "JavaScript-Runtime": "JavaScript runtime",
  "Turbo-Downloader": "Turbo downloader",
  Parallel: "Parallel",
  "{n} Aufträge (Anzahl)": "{n} jobs",
  "Prüfe …": "Checking …",
  "Nach Updates suchen": "Check for updates",
  Quellcode: "Source code",
  Rechte: "Rights",
  "Bitte beachte die folgenden Hinweise.": "Please note the following.",
  "Verwende Downloads nur für Inhalte, für die du die nötigen Rechte oder die Erlaubnis des Rechteinhabers hast. Die App umgeht keine DRM-, Paywall- oder Zugangsbeschränkungen und lädt nur Inhalte, die öffentlich erreichbar sind.":
    "Only download content you have the rights to or the permission of the rights holder for. The app bypasses no DRM, paywall or access restriction and only downloads content that is publicly reachable.",
  "Fragen und Verbesserungsvorschläge sind im Repository willkommen.":
    "Questions and suggestions are welcome in the repository.",
  "Du hast schon die neueste Version.": "You already have the latest version.",
  "Version {version} ist da.": "Version {version} is here.",
  "Update bereit": "Update ready",

  /* ---------- Einstellungen: Rahmen ---------- */
  EINSTELLUNGEN: "SETTINGS",
  "Passe Downloader, Verhalten und Engine bis ins Detail an.":
    "Fine-tune the downloader, its behaviour and the engine.",
  "App & Workflow": "App & workflow",
  "Design, Startseite und Verhalten.": "Theme, start page and behaviour.",
  Design: "Theme",
  Dunkel: "Dark",
  Hell: "Light",
  Startseite: "Start page",
  Home: "Home",
  Playlists: "Playlists",
  "Vor doppelten Downloads warnen": "Warn about duplicate downloads",
  "Verlauf speichern": "Save history",
  "Ordner nach Abschluss öffnen": "Open folder when finished",
  "Schließen legt die App in den Infobereich": "Closing puts the app in the tray",
  "Beim Anmelden starten (im Hintergrund)": "Start on sign-in (in the background)",
  Sprache: "Language",
  Deutsch: "German",
  Englisch: "English",

  /* ---------- Einstellungen: Presets ---------- */
  "Eigene Presets": "Own presets",
  "Speichert die aktuellen Downloadoptionen unter einem Namen.":
    "Saves the current download options under a name.",
  "Preset-Name": "Preset name",
  "Preset speichern": "Save preset",
  "Preset „{name}\" gespeichert.": "Preset “{name}” saved.",
  Löschen: "Delete",

  /* ---------- Einstellungen: Geschwindigkeit ---------- */
  "Geschwindigkeit & Queue": "Speed & queue",
  "Parallelität ist der größte Hebel — YouTube begrenzt die Rate pro Stream.":
    "Parallelism is the biggest lever — YouTube limits the rate per stream.",
  "Gleichzeitige Downloads (1-12)": "Simultaneous downloads (1-12)",
  "Verbindungen pro Stream (2–16)": "Connections per stream (2–16)",
  "Parallele DASH/HLS-Fragmente (1–32)": "Parallel DASH/HLS fragments (1–32)",
  "Retry-Anzahl (0–30)": "Retry count (0–30)",
  "Speed-Limit KiB/s (0 = unbegrenzt)": "Speed limit KiB/s (0 = unlimited)",
  "Maximale Dateinamenlänge": "Maximum file name length",
  "Turbo-Download (aria2c)": "Turbo download (aria2c)",
  "Faststart (Index voran)": "Faststart (index first)",
  "Archiv (bereits Geladenes überspringen)": "Archive (skip already downloaded)",
  "Playlists in einem Durchgang": "Playlists in a single run",

  /* ---------- Einstellungen: Speicherort ---------- */
  "Speicherort & Dateinamen": "Location & file names",
  "Wo die Dateien landen und wie sie heissen.": "Where files land and what they are called.",
  Downloadordner: "Download folder",
  "Dateiname-Template": "File name template",
  "Ordner-Template": "Folder template",
  Variablen: "Variables",
  "Für jedes Video einen eigenen Unterordner": "A separate subfolder for every video",

  /* ---------- Einstellungen: Metadaten ---------- */
  "Metadaten & Zusatzdateien": "Metadata & extra files",
  "Steuere, wie viele Metadaten zu einer Datei dazukommen.":
    "Control how much metadata is added to a file.",
  "Metadaten einbetten": "Embed metadata",
  "Kapitel einbetten": "Embed chapters",
  ".info.json schreiben": "Write .info.json",
  ".description schreiben": "Write .description",
  "Untertitel laden": "Download subtitles",
  "Automatische Untertitel": "Automatic subtitles",
  "Untertitel-Sprachen": "Subtitle languages",
  "Untertitel-Format": "Subtitle format",

  /* ---------- Einstellungen: Engine ---------- */
  "Engine-Optionen (für Profis)": "Engine options (for experts)",
  "Werte werden direkt an yt-dlp weitergegeben.": "Values are passed straight to yt-dlp.",
  "Extractor-Argumente": "Extractor arguments",
  "Zusatz-Argumente": "Extra arguments",
  "Engine & Updates": "Engine & updates",
  "Die mitgelieferten Werkzeuge und ihre Versionen.": "The bundled tools and their versions.",
  "yt-dlp aktualisieren": "Update yt-dlp",
  "yt-dlp ist jetzt auf Version {version}.": "yt-dlp is now on version {version}.",
  "yt-dlp geht alle paar Wochen kaputt, wenn sich eine Seite ändert. Die Aktualisierung holt die neueste Version von GitHub. Vorher bitte laufende Downloads beenden.":
    "yt-dlp breaks every few weeks when a site changes. The update fetches the newest version from GitHub. Please stop running downloads first.",

  /* ---------- App: Meldungen ---------- */
  "SilasDownloader wird gestartet …": "SilasDownloader is starting …",
  "Einstellungen konnten nicht geladen werden.": "Settings could not be loaded.",
  "Download wird vorbereitet": "Preparing download",
  "Bereit für neuen Versuch": "Ready for another try",
  "Wird abgebrochen …": "Cancelling …",
  "Download abgeschlossen": "Download finished",
  "Download abgeschlossen.": "Download finished.",
  "Der Download ist fehlgeschlagen.": "The download failed.",
  "Fehlgeschlagen – Wiederholung {n}/2 läuft …": "Failed – retry {n}/2 running …",
  "Version {version} ist verfügbar — im Tab „Über\" installieren.":
    "Version {version} is available — install it on the “About” tab.",
  "Update verfügbar: Version {version}": "Update available: version {version}",
  Audio: "Audio",
};
